//! The public API, used the way a downstream crate would use it.
//!
//! If this file does not compile, the crate is not a library no matter what the
//! README says.

use std::path::Path;

use imgsmith::{
    DynamicImage, Error, Fit, Flip, Format, Gravity, Options, encodable, load, process, transform,
};

fn test_image() -> DynamicImage {
    let mut img = image::RgbImage::new(200, 100);
    for (x, _, p) in img.enumerate_pixels_mut() {
        *p = if x < 100 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        };
    }
    DynamicImage::ImageRgb8(img)
}

#[test]
fn transform_resizes_in_memory() {
    let opts = Options {
        width: Some(100),
        ..Options::default()
    };
    let out = transform(test_image(), &opts, None).unwrap();
    assert_eq!((out.width(), out.height()), (100, 50));
}

#[test]
fn transform_crops_then_rotates() {
    let opts = Options {
        crop: Some(imgsmith::parse_crop("100x100+100+0").unwrap()),
        gravity: Gravity::Centre,
        rotate: Some(90),
        ..Options::default()
    };
    let out = transform(test_image(), &opts, None).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));
    // The crop took the blue right half, so after a 90 degree turn the top-left
    // pixel is still blue.
    let rgb = out.to_rgb8();
    assert_eq!(rgb.get_pixel(5, 5).0, [0, 0, 255]);
}

#[test]
fn transform_honours_orientation() {
    let opts = Options {
        auto_orient: true,
        ..Options::default()
    };
    let out = transform(test_image(), &opts, Some(6)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 200));
}

#[test]
fn flip_and_fit_fill_work_together() {
    let opts = Options {
        width: Some(50),
        height: Some(50),
        fit: Fit::Fill,
        flip: Some(Flip::H),
        ..Options::default()
    };
    let out = transform(test_image(), &opts, None).unwrap();
    assert_eq!((out.width(), out.height()), (50, 50));
    let rgb = out.to_rgb8();
    // Flipped, so the left edge is the blue half.
    assert_eq!(rgb.get_pixel(2, 2).0, [0, 0, 255]);
}

#[test]
fn encodes_every_supported_format() {
    let img = test_image();
    for (fmt, magic) in [
        (Format::Png, &b"\x89PNG"[..]),
        (Format::Jpeg, &b"\xFF\xD8"[..]),
        (Format::Webp, &b"RIFF"[..]),
        (Format::Gif, &b"GIF8"[..]),
        (Format::Bmp, &b"BM"[..]),
        (Format::Qoi, &b"qoif"[..]),
    ] {
        let bytes = encodable(&img, fmt).unwrap();
        assert_eq!(&bytes[..magic.len()], magic, "{fmt:?} magic bytes");
    }
}

#[test]
fn format_from_extension_is_case_insensitive() {
    assert_eq!(Format::from_extension("JPG"), Some(Format::Jpeg));
    assert_eq!(Format::from_extension("jpeg"), Some(Format::Jpeg));
    assert_eq!(Format::from_extension("tif"), Some(Format::Tiff));
    assert_eq!(Format::from_extension("nope"), None);
}

#[test]
fn process_writes_a_file_and_reports_sizes() {
    let dir = std::env::temp_dir().join("imgsmith-api-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("in.png");
    let output = dir.join("out.jpg");
    test_image().save(&input).unwrap();

    let opts = Options {
        width: Some(64),
        ..Options::default()
    };
    let outcome = process(&input, &output, Format::Jpeg, &opts, false).unwrap();
    assert_eq!(outcome.out_dims, (64, 32));
    assert!(outcome.out_bytes > 0);
    assert!(output.exists());

    // Second run without force must refuse.
    let err = process(&input, &output, Format::Jpeg, &opts, false).unwrap_err();
    assert!(matches!(err, Error::Exists(_)));
}

#[test]
fn error_messages_name_the_file() {
    let err = load(Path::new("/definitely/not/here.png"), 0).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("here.png"), "message: {text}");
}
