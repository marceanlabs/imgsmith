//! End-to-end tests against the real binary. Nothing here is mocked: every
//! assertion runs `imgsmith` and reads the file it wrote.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_imgsmith")
}

fn workdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("imgsmith-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn solid_png(path: &Path, w: u32, h: u32, rgb: [u8; 3]) {
    let img = image::RgbImage::from_pixel(w, h, image::Rgb(rgb));
    img.save(path).unwrap();
}

/// Left half red, right half blue: a crop is only correct if the colour moves.
fn split_png(path: &Path, w: u32, h: u32) {
    let mut img = image::RgbImage::new(w, h);
    for (x, _, p) in img.enumerate_pixels_mut() {
        *p = if x < w / 2 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        };
    }
    img.save(path).unwrap();
}

fn dims(path: &Path) -> (u32, u32) {
    // Decode rather than read the header: AVIF has no header-only dimensions.
    let img = image::open(path).unwrap();
    (img.width(), img.height())
}

fn pixel(path: &Path, x: u32, y: u32) -> [u8; 3] {
    let img = image::open(path).unwrap().to_rgb8();
    img.get_pixel(x, y).0
}

fn magic(path: &Path) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    data[..4.min(data.len())].to_vec()
}

/// Insert a minimal EXIF APP1 segment carrying only an orientation tag.
fn add_exif_orientation(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    let mut payload: Vec<u8> = Vec::new();
    payload.extend_from_slice(b"Exif\0\0");
    payload.extend_from_slice(b"II\x2A\x00");
    payload.extend_from_slice(&8u32.to_le_bytes());
    payload.extend_from_slice(&1u16.to_le_bytes());
    payload.extend_from_slice(&0x0112u16.to_le_bytes());
    payload.extend_from_slice(&3u16.to_le_bytes());
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&orientation.to_le_bytes());
    payload.extend_from_slice(&[0, 0]);
    payload.extend_from_slice(&0u32.to_le_bytes());

    let len = (payload.len() + 2) as u16;
    let mut out = Vec::with_capacity(jpeg.len() + payload.len() + 4);
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(&[0xFF, 0xE1]);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

fn jpeg_with_orientation(path: &Path, w: u32, h: u32, orientation: u16) {
    let mut buf = Vec::new();
    let img = image::RgbImage::from_pixel(w, h, image::Rgb([10, 120, 200]));
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90)
        .encode(img.as_raw(), w, h, image::ExtendedColorType::Rgb8)
        .unwrap();
    std::fs::write(path, add_exif_orientation(&buf, orientation)).unwrap();
}

fn has_app1(path: &Path) -> bool {
    let data = std::fs::read(path).unwrap();
    data.windows(2).any(|w| w == [0xFF, 0xE1])
}

#[test]
fn converts_png_to_jpeg() {
    let dir = workdir("convert");
    let input = dir.join("in.png");
    let output = dir.join("out.jpg");
    solid_png(&input, 320, 200, [12, 200, 90]);

    let out = run(&[input.to_str().unwrap(), "-o", output.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (320, 200));
    assert_eq!(magic(&output)[..2], [0xFF, 0xD8], "not a JPEG");
}

#[test]
fn resize_width_keeps_aspect() {
    let dir = workdir("resize");
    let input = dir.join("in.png");
    let output = dir.join("out.png");
    solid_png(&input, 400, 200, [0, 0, 0]);

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--width",
        "200",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (200, 100));
}

#[test]
fn thumb_fits_inside_and_does_not_upscale() {
    let dir = workdir("thumb");
    let big = dir.join("big.png");
    let small = dir.join("small.png");
    let out_big = dir.join("out_big.png");
    let out_small = dir.join("out_small.png");
    solid_png(&big, 400, 200, [1, 2, 3]);
    solid_png(&small, 40, 20, [1, 2, 3]);

    let out = run(&[
        big.to_str().unwrap(),
        "-o",
        out_big.to_str().unwrap(),
        "--thumb",
        "50",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&out_big), (50, 25));

    let out = run(&[
        small.to_str().unwrap(),
        "-o",
        out_small.to_str().unwrap(),
        "--thumb",
        "100",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&out_small), (40, 20), "thumbnail must never upscale");
}

#[test]
fn crop_by_offset_takes_the_right_side() {
    let dir = workdir("crop");
    let input = dir.join("in.png");
    let left = dir.join("left.png");
    let right = dir.join("right.png");
    split_png(&input, 100, 100);

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        left.to_str().unwrap(),
        "--crop",
        "50x50+0+0",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&left), (50, 50));
    assert_eq!(pixel(&left, 5, 5), [255, 0, 0]);

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        right.to_str().unwrap(),
        "--crop",
        "50x50+50+0",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(pixel(&right, 5, 5), [0, 0, 255]);
}

#[test]
fn crop_gravity_picks_the_corner() {
    let dir = workdir("gravity");
    let input = dir.join("in.png");
    let output = dir.join("out.png");
    split_png(&input, 100, 100);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--crop",
        "50x50",
        "--gravity",
        "south-east",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(pixel(&output, 5, 5), [0, 0, 255]);
}

#[test]
fn auto_orient_applies_and_drops_the_tag() {
    let dir = workdir("orient");
    let input = dir.join("in.jpg");
    let output = dir.join("out.jpg");
    jpeg_with_orientation(&input, 200, 100, 6);

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--auto-orient",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (100, 200), "orientation 6 is a 90 degree turn");
    assert!(!has_app1(&output), "the orientation tag must not survive");
}

#[test]
fn metadata_is_carried_unless_stripped() {
    let dir = workdir("meta");
    let input = dir.join("in.jpg");
    let kept = dir.join("kept.jpg");
    let stripped = dir.join("stripped.jpg");
    jpeg_with_orientation(&input, 120, 80, 1);
    assert!(has_app1(&input), "fixture must carry EXIF");

    let out = run(&[input.to_str().unwrap(), "-o", kept.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(has_app1(&kept), "EXIF should be carried by default");

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        stripped.to_str().unwrap(),
        "--strip",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!has_app1(&stripped), "--strip must remove EXIF");
}

#[test]
fn refuses_animated_gif() {
    let dir = workdir("animated");
    let input = dir.join("anim.gif");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GIF89a");
    bytes.extend_from_slice(b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00");
    bytes.extend_from_slice(b"\x3B");
    std::fs::write(&input, bytes).unwrap();

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        dir.join("out.gif").to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("animated"),
        "message should name the problem: {}",
        stderr(&out)
    );
}

#[test]
fn refuses_to_overwrite_without_force() {
    let dir = workdir("overwrite");
    let input = dir.join("in.png");
    let output = dir.join("out.png");
    solid_png(&input, 60, 60, [9, 9, 9]);
    std::fs::write(&output, b"do not touch").unwrap();

    let out = run(&[input.to_str().unwrap(), "-o", output.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("--force"), "{}", stderr(&out));
    assert_eq!(std::fs::read(&output).unwrap(), b"do not touch");

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (60, 60));
}

#[test]
fn batch_directory_mirrors_structure() {
    let dir = workdir("batch");
    let src = dir.join("src");
    let nested = src.join("holiday");
    std::fs::create_dir_all(&nested).unwrap();
    solid_png(&src.join("one.png"), 100, 50, [1, 1, 1]);
    solid_png(&nested.join("two.png"), 80, 40, [2, 2, 2]);
    let outdir = dir.join("out");

    let out = run(&[
        src.to_str().unwrap(),
        "-o",
        outdir.to_str().unwrap(),
        "--width",
        "50",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&outdir.join("one.png")), (50, 25));
    assert_eq!(dims(&outdir.join("holiday/two.png")), (50, 25));
}

#[test]
fn quality_changes_jpeg_size() {
    let dir = workdir("quality");
    let input = dir.join("in.png");
    let low = dir.join("low.jpg");
    let high = dir.join("high.jpg");
    // Noise compresses badly, so quality actually shows up in the byte count.
    let mut img = image::RgbImage::new(400, 300);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let v = ((x * 7 + y * 13) % 256) as u8;
        *p = image::Rgb([v, v.wrapping_mul(3), v.wrapping_add(11)]);
    }
    img.save(&input).unwrap();

    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        low.to_str().unwrap(),
        "--quality",
        "20",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        high.to_str().unwrap(),
        "--quality",
        "95",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));

    let low_size = std::fs::metadata(&low).unwrap().len();
    let high_size = std::fs::metadata(&high).unwrap().len();
    assert!(
        low_size < high_size / 2,
        "quality 20 ({low_size} B) should be far smaller than quality 95 ({high_size} B)"
    );
}

#[test]
fn ico_over_256_is_refused_with_a_fix() {
    let dir = workdir("ico");
    let input = dir.join("in.png");
    solid_png(&input, 400, 400, [0, 0, 0]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        dir.join("out.ico").to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("--thumb 256"), "{}", stderr(&out));
}

#[test]
fn max_pixels_guard_refuses_large_images() {
    let dir = workdir("bomb");
    let input = dir.join("in.png");
    solid_png(&input, 1200, 1200, [0, 0, 0]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        dir.join("out.png").to_str().unwrap(),
        "--max-mp",
        "1",
    ]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("megapixels"), "{}", stderr(&out));
}

#[test]
fn webp_quality_is_refused_rather_than_ignored() {
    let dir = workdir("webpq");
    let input = dir.join("in.png");
    solid_png(&input, 40, 40, [3, 3, 3]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        dir.join("out.webp").to_str().unwrap(),
        "--quality",
        "70",
    ]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("lossless"), "{}", stderr(&out));
}

#[test]
fn webp_output_is_written_losslessly() {
    let dir = workdir("webp");
    let input = dir.join("in.png");
    let output = dir.join("out.webp");
    solid_png(&input, 64, 48, [200, 10, 10]);
    let out = run(&[input.to_str().unwrap(), "-o", output.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (64, 48));
    // VP8L is lossless WebP; VP8 is lossy.
    let data = std::fs::read(&output).unwrap();
    assert!(
        data.windows(4).any(|w| w == b"VP8L"),
        "expected lossless WebP"
    );
}

#[test]
fn avif_is_written() {
    let dir = workdir("avifout");
    let input = dir.join("in.png");
    let output = dir.join("out.avif");
    solid_png(&input, 64, 48, [180, 40, 20]);
    let out = run(&[input.to_str().unwrap(), "-o", output.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let data = std::fs::read(&output).unwrap();
    assert_eq!(&data[4..8], b"ftyp", "not an ISO base media file");
}

#[test]
fn avif_input_is_refused_with_the_reason() {
    let dir = workdir("avifin");
    let input = dir.join("in.avif");
    // A real AVIF header is enough: the refusal happens before decoding.
    let mut data = vec![0u8; 32];
    data[4..8].copy_from_slice(b"ftyp");
    data[8..12].copy_from_slice(b"avif");
    std::fs::write(&input, data).unwrap();
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        dir.join("out.png").to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("C decoder"),
        "the message must say why: {}",
        stderr(&out)
    );
}

#[test]
fn dry_run_writes_nothing() {
    let dir = workdir("dry");
    let input = dir.join("in.png");
    let output = dir.join("out.png");
    solid_png(&input, 30, 30, [0, 0, 0]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--dry-run",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!output.exists());
    assert!(stderr(&out).contains("would write"));
}

#[test]
fn rotate_90_swaps_dimensions() {
    let dir = workdir("rotate");
    let input = dir.join("in.png");
    let output = dir.join("out.png");
    solid_png(&input, 200, 100, [0, 0, 0]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--rotate",
        "90",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (100, 200));
}

#[test]
fn cover_fit_produces_the_exact_box() {
    let dir = workdir("cover");
    let input = dir.join("in.png");
    let output = dir.join("out.png");
    solid_png(&input, 1000, 500, [0, 0, 0]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--width",
        "300",
        "--height",
        "300",
        "--fit",
        "cover",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(dims(&output), (300, 300));
}

#[test]
fn unknown_output_extension_points_at_format() {
    let dir = workdir("badext");
    let input = dir.join("in.png");
    solid_png(&input, 20, 20, [0, 0, 0]);
    let out = run(&[
        input.to_str().unwrap(),
        "-o",
        dir.join("out.zzz").to_str().unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("--format"), "{}", stderr(&out));
}

#[test]
fn every_single_file_format_round_trips() {
    let dir = workdir("formats");
    let input = dir.join("in.png");
    let mut img = image::RgbImage::new(64, 64);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgb([(x * 4) as u8, (y * 4) as u8, 128]);
    }
    img.save(&input).unwrap();

    for ext in ["png", "jpg", "webp", "tiff", "gif", "bmp", "tga", "pnm", "qoi"] {
        let output = dir.join(format!("out.{ext}"));
        let out = run(&[input.to_str().unwrap(), "-o", output.to_str().unwrap()]);
        assert!(
            out.status.success(),
            "{ext} failed: {}",
            stderr(&out)
        );
        assert_eq!(dims(&output), (64, 64), "{ext} lost its dimensions");
    }
}
