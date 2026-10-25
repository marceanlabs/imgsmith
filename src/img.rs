//! Decode, transform, encode. One file in, one file out, no shared state.

use std::path::{Path, PathBuf};

use clap::ValueEnum;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageReader};

use crate::error::Error;
use crate::meta;
use crate::ops::{self, CropSpec, Fit, Flip, Gravity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Png,
    Jpeg,
    Webp,
    Avif,
    Tiff,
    Gif,
    Bmp,
    Ico,
    Tga,
    Pnm,
    Qoi,
    Hdr,
    Exr,
    Farbfeld,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
            Format::Webp => "webp",
            Format::Avif => "avif",
            Format::Tiff => "tiff",
            Format::Gif => "gif",
            Format::Bmp => "bmp",
            Format::Ico => "ico",
            Format::Tga => "tga",
            Format::Pnm => "pnm",
            Format::Qoi => "qoi",
            Format::Hdr => "hdr",
            Format::Exr => "exr",
            Format::Farbfeld => "ff",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(Format::Png),
            "jpg" | "jpeg" | "jpe" => Some(Format::Jpeg),
            "webp" => Some(Format::Webp),
            "avif" => Some(Format::Avif),
            "tif" | "tiff" => Some(Format::Tiff),
            "gif" => Some(Format::Gif),
            "bmp" => Some(Format::Bmp),
            "ico" => Some(Format::Ico),
            "tga" => Some(Format::Tga),
            "pnm" | "ppm" | "pgm" | "pbm" => Some(Format::Pnm),
            "qoi" => Some(Format::Qoi),
            "hdr" => Some(Format::Hdr),
            "exr" => Some(Format::Exr),
            "ff" | "farbfeld" => Some(Format::Farbfeld),
            _ => None,
        }
    }

}

#[derive(Clone, Debug)]
pub struct Options {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: Fit,
    pub crop: Option<CropSpec>,
    pub gravity: Gravity,
    pub rotate: Option<u16>,
    pub flip: Option<Flip>,
    pub auto_orient: bool,
    pub strip: bool,
    pub quality: Option<u8>,
    pub thumbnail: bool,
    pub allow_upscale: bool,
    pub max_pixels: u64,
}

#[derive(Debug)]
pub struct Outcome {
    pub in_dims: (u32, u32),
    pub out_dims: (u32, u32),
    pub in_bytes: u64,
    pub out_bytes: u64,
}

pub fn load(path: &Path, max_pixels: u64) -> Result<DynamicImage, Error> {
    let reader = ImageReader::open(path)
        .map_err(|e| Error::io(path, e))?
        .with_guessed_format()
        .map_err(|e| Error::io(path, e))?;

    // AVIF is written by this tool but not read: its decoder needs a C library,
    // and one C dependency is the thing this tool exists to avoid.
    if reader.format() == Some(image::ImageFormat::Avif) {
        return Err(Error::Unsupported {
            path: path.to_path_buf(),
            what: "reading AVIF needs a C decoder, which this build does not carry. \
                   AVIF output is supported; convert an AVIF input with avifdec or sharp first"
                .into(),
        });
    }
    // Not every format can report its dimensions from the header alone (AVIF
    // cannot), so the guard runs twice: cheaply here where it can, and again
    // after decoding, which catches the rest before the image is used.
    if let Ok((w, h)) = reader.into_dimensions() {
        let pixels = w as u64 * h as u64;
        if max_pixels > 0 && pixels > max_pixels {
            return Err(Error::TooLarge {
                path: path.to_path_buf(),
                pixels,
                limit: max_pixels,
            });
        }
    }
    let img = ImageReader::open(path)
        .map_err(|e| Error::io(path, e))?
        .with_guessed_format()
        .map_err(|e| Error::io(path, e))?
        .decode()
        .map_err(|e| Error::image(path, e))?;
    let pixels = u64::from(img.width()) * u64::from(img.height());
    if max_pixels > 0 && pixels > max_pixels {
        return Err(Error::TooLarge {
            path: path.to_path_buf(),
            pixels,
            limit: max_pixels,
        });
    }
    Ok(img)
}

pub fn transform(
    img: DynamicImage,
    opts: &Options,
    orientation: Option<u8>,
) -> Result<DynamicImage, Error> {
    let mut img = img;

    // 1. Orientation first, so every later coordinate matches what a person sees.
    if opts.auto_orient
        && let Some(tag) = orientation
    {
        img = ops::orient(img, tag);
    }

    // 2. Crop in source pixels, before any scaling.
    if let Some(spec) = &opts.crop {
        let (x, y, w, h) = ops::crop_rect((img.width(), img.height()), spec, opts.gravity)
            .map_err(Error::Usage)?;
        img = img.crop_imm(x, y, w, h);
    }

    // 3. Resize.
    let boxed = (opts.width, opts.height);
    let (tw, th) = ops::target_size(
        (img.width(), img.height()),
        boxed,
        opts.fit,
        opts.allow_upscale,
    );
    if opts.fit == Fit::Cover
        && let (Some(bw), Some(bh)) = (opts.width, opts.height)
    {
        if (tw, th) != (img.width(), img.height()) {
            img = img.resize_exact(tw, th, image::imageops::FilterType::Lanczos3);
        }
        let x = img.width().saturating_sub(bw) / 2;
        let y = img.height().saturating_sub(bh) / 2;
        img = img.crop_imm(x, y, bw.min(img.width()), bh.min(img.height()));
    } else if (tw, th) != (img.width(), img.height()) {
        img = if opts.thumbnail {
            img.thumbnail(tw, th)
        } else {
            img.resize_exact(tw, th, image::imageops::FilterType::Lanczos3)
        };
    }

    // 4. Rotation and mirroring last: they are free at this point.
    if let Some(deg) = opts.rotate {
        img = match deg {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _ => img,
        };
    }
    if let Some(flip) = opts.flip {
        img = match flip {
            Flip::H => img.fliph(),
            Flip::V => img.flipv(),
        };
    }

    Ok(img)
}

fn color_type(img: &DynamicImage) -> ExtendedColorType {
    match img.color() {
        image::ColorType::L8 => ExtendedColorType::L8,
        image::ColorType::La8 => ExtendedColorType::La8,
        image::ColorType::Rgb8 => ExtendedColorType::Rgb8,
        image::ColorType::Rgba8 => ExtendedColorType::Rgba8,
        image::ColorType::L16 => ExtendedColorType::L16,
        image::ColorType::La16 => ExtendedColorType::La16,
        image::ColorType::Rgb16 => ExtendedColorType::Rgb16,
        image::ColorType::Rgba16 => ExtendedColorType::Rgba16,
        image::ColorType::Rgb32F => ExtendedColorType::Rgb32F,
        image::ColorType::Rgba32F => ExtendedColorType::Rgba32F,
        // ColorType is non-exhaustive; anything new is assumed to be 8-bit RGBA.
        _ => ExtendedColorType::Rgba8,
    }
}

/// Native bytes when the format accepts them, otherwise RGBA8.
fn raw_supported(img: &DynamicImage, allowed: &[ExtendedColorType]) -> (Vec<u8>, ExtendedColorType) {
    let native = color_type(img);
    if allowed.contains(&native) {
        (img.as_bytes().to_vec(), native)
    } else {
        let rgba = img.to_rgba8();
        (rgba.as_raw().clone(), ExtendedColorType::Rgba8)
    }
}

/// Alpha over white. Formats without an alpha channel need a decision, and white
/// is the one that keeps a logo usable rather than turning its edges black.
fn flatten_white(img: &DynamicImage) -> image::RgbImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (dst, src) in out.pixels_mut().zip(rgba.pixels()) {
        let a = u32::from(src[3]);
        let blend = |c: u8| ((u32::from(c) * a + 255 * (255 - a)) / 255) as u8;
        *dst = image::Rgb([blend(src[0]), blend(src[1]), blend(src[2])]);
    }
    out
}

pub fn encode(
    img: &DynamicImage,
    fmt: Format,
    quality: u8,
    path: &Path,
) -> Result<Vec<u8>, Error> {
    let mut buf: Vec<u8> = Vec::new();
    let (w, h) = (img.width(), img.height());
    let fail = |e: image::ImageError| Error::image(path, e);

    match fmt {
        Format::Jpeg => {
            let flat = flatten_white(img);
            let (fw, fh) = flat.dimensions();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality)
                .write_image(flat.as_raw(), fw, fh, ExtendedColorType::Rgb8)
                .map_err(fail)?;
        }
        Format::Png => {
            let (data, ct) = raw_supported(
                img,
                &[
                    ExtendedColorType::L8,
                    ExtendedColorType::La8,
                    ExtendedColorType::Rgb8,
                    ExtendedColorType::Rgba8,
                    ExtendedColorType::L16,
                    ExtendedColorType::La16,
                    ExtendedColorType::Rgb16,
                    ExtendedColorType::Rgba16,
                ],
            );
            image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                image::codecs::png::CompressionType::Default,
                image::codecs::png::FilterType::Adaptive,
            )
            .write_image(&data, w, h, ct)
            .map_err(fail)?;
        }
        Format::Webp => {
            let (data, ct) = raw_supported(
                img,
                &[
                    ExtendedColorType::L8,
                    ExtendedColorType::La8,
                    ExtendedColorType::Rgb8,
                    ExtendedColorType::Rgba8,
                ],
            );
            image::codecs::webp::WebPEncoder::new_lossless(&mut buf)
                .write_image(&data, w, h, ct)
                .map_err(fail)?;
        }
        Format::Avif => {
            let (data, ct) = if img.color().has_alpha() {
                let rgba = img.to_rgba8();
                (rgba.as_raw().clone(), ExtendedColorType::Rgba8)
            } else {
                let rgb = img.to_rgb8();
                (rgb.as_raw().clone(), ExtendedColorType::Rgb8)
            };
            image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut buf, 6, quality)
                .write_image(&data, w, h, ct)
                .map_err(fail)?;
        }
        Format::Tiff => {
            let (data, ct) = raw_supported(
                img,
                &[
                    ExtendedColorType::L8,
                    ExtendedColorType::Rgb8,
                    ExtendedColorType::Rgba8,
                ],
            );
            let mut cursor = std::io::Cursor::new(&mut buf);
            image::codecs::tiff::TiffEncoder::new(&mut cursor)
                .write_image(&data, w, h, ct)
                .map_err(fail)?;
        }
        Format::Gif => {
            let frame = image::Frame::new(img.to_rgba8());
            image::codecs::gif::GifEncoder::new(&mut buf)
                .encode_frame(frame)
                .map_err(fail)?;
        }
        Format::Bmp => {
            let (data, ct) = raw_supported(
                img,
                &[
                    ExtendedColorType::L8,
                    ExtendedColorType::Rgb8,
                    ExtendedColorType::Rgba8,
                ],
            );
            image::codecs::bmp::BmpEncoder::new(&mut buf)
                .write_image(&data, w, h, ct)
                .map_err(fail)?;
        }
        Format::Ico => {
            if w > 256 || h > 256 {
                return Err(Error::Unsupported {
                    path: path.to_path_buf(),
                    what: format!(
                        "ICO accepts at most 256x256, this image is {w}x{h} (use --thumb 256)"
                    ),
                });
            }
            let rgba = img.to_rgba8();
            image::codecs::ico::IcoEncoder::new(&mut buf)
                .write_image(rgba.as_raw(), w, h, ExtendedColorType::Rgba8)
                .map_err(fail)?;
        }
        Format::Tga => {
            let (data, ct) = raw_supported(
                img,
                &[ExtendedColorType::Rgb8, ExtendedColorType::Rgba8],
            );
            image::codecs::tga::TgaEncoder::new(&mut buf)
                .write_image(&data, w, h, ct)
                .map_err(fail)?;
        }
        Format::Pnm => {
            let flat = flatten_white(img);
            let (fw, fh) = flat.dimensions();
            image::codecs::pnm::PnmEncoder::new(&mut buf)
                .write_image(flat.as_raw(), fw, fh, ExtendedColorType::Rgb8)
                .map_err(fail)?;
        }
        Format::Qoi => {
            let (data, ct) = raw_supported(
                img,
                &[ExtendedColorType::Rgb8, ExtendedColorType::Rgba8],
            );
            image::codecs::qoi::QoiEncoder::new(&mut buf)
                .write_image(&data, w, h, ct)
                .map_err(fail)?;
        }
        Format::Hdr => {
            if !(8..=32_768).contains(&w) || !(8..=32_768).contains(&h) {
                return Err(Error::Unsupported {
                    path: path.to_path_buf(),
                    what: format!("HDR needs dimensions between 8 and 32768, this is {w}x{h}"),
                });
            }
            let rgb = img.to_rgb32f();
            let bytes = f32_ne_bytes(rgb.as_raw());
            image::codecs::hdr::HdrEncoder::new(&mut buf)
                .write_image(&bytes, w, h, ExtendedColorType::Rgb32F)
                .map_err(fail)?;
        }
        Format::Exr => {
            let (bytes, ct) = if img.color().has_alpha() {
                (f32_ne_bytes(img.to_rgba32f().as_raw()), ExtendedColorType::Rgba32F)
            } else {
                (f32_ne_bytes(img.to_rgb32f().as_raw()), ExtendedColorType::Rgb32F)
            };
            let mut cursor = std::io::Cursor::new(&mut buf);
            image::codecs::openexr::OpenExrEncoder::new(&mut cursor)
                .write_image(&bytes, w, h, ct)
                .map_err(fail)?;
        }
        Format::Farbfeld => {
            let rgba = img.to_rgba16();
            let bytes = u16_ne_bytes(rgba.as_raw());
            image::codecs::farbfeld::FarbfeldEncoder::new(&mut buf)
                .encode(&bytes, w, h)
                .map_err(fail)?;
        }
    }

    Ok(buf)
}

/// Native-endian f32 samples as bytes, without `unsafe`.
fn f32_ne_bytes(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        out.extend_from_slice(&sample.to_ne_bytes());
    }
    out
}

/// Native-endian u16 samples as bytes, without `unsafe`.
fn u16_ne_bytes(samples: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        out.extend_from_slice(&sample.to_ne_bytes());
    }
    out
}

/// Full pipeline for one file.
pub fn process(
    input: &Path,
    output: &Path,
    fmt: Format,
    opts: &Options,
    force: bool,
) -> Result<Outcome, Error> {
    if !force && output.exists() {
        return Err(Error::Exists(output.to_path_buf()));
    }
    if let Some(kind) = meta::animated_kind(input) {
        return Err(Error::Animated {
            path: input.to_path_buf(),
            kind,
        });
    }

    let orientation = if opts.auto_orient {
        meta::exif_orientation(input)
    } else {
        None
    };
    let in_bytes = std::fs::metadata(input)
        .map_err(|e| Error::io(input, e))?
        .len();

    let img = load(input, opts.max_pixels)?;
    let in_dims = (img.width(), img.height());
    let img = transform(img, opts, orientation)?;
    let out_dims = (img.width(), img.height());

    let quality = opts.quality.unwrap_or(match fmt {
        Format::Avif => 80,
        _ => 85,
    });
    let encoded = encode(&img, fmt, quality, output)?;

    let encoded = if fmt == Format::Jpeg && !opts.strip {
        let consumed = orientation.is_some_and(|tag| tag != 1);
        let segments = meta::jpeg_app_segments(input, !consumed);
        meta::splice_jpeg(&encoded, &segments)
    } else {
        encoded
    };

    write_atomic(output, &encoded)?;

    Ok(Outcome {
        in_dims,
        out_dims,
        in_bytes,
        out_bytes: encoded.len() as u64,
    })
}

/// Write through a temporary file in the destination directory, then rename.
/// A failed encode never leaves a half-written image where the original was
/// expected to be.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "out".into());
    let tmp: PathBuf = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::io(path, e)
    })
}
