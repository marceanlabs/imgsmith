#![no_main]
//! Every byte an attacker can send imgsmith is a file. This feeds arbitrary
//! bytes down the same path the binary takes: format sniffing, decode, the
//! EXIF and animation probes, one transform, one encode.

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("imgsmith-fuzz");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(name)
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let path = scratch("input");
    if std::fs::write(&path, data).is_err() {
        return;
    }

    // The metadata probes run on every file the tool touches, before any decode.
    let _ = imgsmith::meta::exif_orientation(&path);
    let _ = imgsmith::meta::animated_kind(&path);
    let _ = imgsmith::meta::jpeg_app_segments(&path, true);
    let _ = imgsmith::meta::jpeg_app_segments(&path, false);

    // 30 megapixels keeps a hostile header from asking for a 40 GB allocation
    // while still exercising the guard.
    if let Ok(img) = imgsmith::load(&path, 30_000_000) {
        let opts = imgsmith::Options {
            width: Some(64),
            height: Some(64),
            thumbnail: true,
            ..imgsmith::Options::default()
        };
        if let Ok(small) = imgsmith::transform(img, &opts, Some(6)) {
            let _ = imgsmith::encodable(&small, imgsmith::Format::Png);
            let _ = imgsmith::encodable(&small, imgsmith::Format::Jpeg);
            let _ = imgsmith::encodable(&small, imgsmith::Format::Webp);
        }
    }
});
