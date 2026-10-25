//! Metadata: reading the EXIF orientation, carrying JPEG metadata forward, and
//! refusing animated inputs.
//!
//! The tool re-encodes pixels. Nothing here preserves metadata it does not
//! explicitly copy, so the default is "whatever is not copied is gone".

use std::io::Read;
use std::path::Path;

/// EXIF orientation tag (1-8), if the file has one.
pub fn exif_orientation(path: &Path) -> Option<u8> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
    field.value.get_uint(0).map(|v| v as u8)
}

/// Animated inputs are refused rather than silently flattened to frame one.
pub fn animated_kind(path: &Path) -> Option<&'static str> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut head = vec![0u8; 64 * 1024];
    let n = file.read(&mut head).ok()?;
    head.truncate(n);

    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        return find(&head, b"acTL").map(|_| "animated PNG");
    }
    if head.starts_with(b"GIF8") {
        return find(&head, b"NETSCAPE2.0").map(|_| "animated GIF");
    }
    if head.len() > 16 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        return find(&head, b"ANIM").map(|_| "animated WebP");
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The JPEG APP segments worth carrying into a re-encoded JPEG.
///
/// `keep_exif = false` is used whenever the image has been auto-oriented: the
/// orientation tag would then be wrong, so the whole EXIF block goes rather
/// than being rewritten. ICC (APP2) survives either way, because a colour
/// profile is independent of orientation.
pub fn jpeg_app_segments(path: &Path, keep_exif: bool) -> Vec<Vec<u8>> {
    let Ok(data) = std::fs::read(path) else {
        return Vec::new();
    };
    if !data.starts_with(&[0xFF, 0xD8]) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            break;
        }
        let marker = data[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        // Standalone markers carry no length.
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if marker == 0xDA {
            break; // start of scan: no more headers
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if len < 2 || i + 2 + len > data.len() {
            break;
        }
        let keep = match marker {
            0xE1 => keep_exif, // EXIF or XMP
            0xE2 => true,      // ICC profile
            0xED => keep_exif, // IPTC
            _ => false,
        };
        if keep {
            out.push(data[i..i + 2 + len].to_vec());
        }
        i += 2 + len;
    }
    out
}

/// Insert segments immediately after the SOI marker of an encoded JPEG.
pub fn splice_jpeg(encoded: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
    if segments.is_empty() || !encoded.starts_with(&[0xFF, 0xD8]) {
        return encoded.to_vec();
    }
    let extra: usize = segments.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(encoded.len() + extra);
    out.extend_from_slice(&encoded[..2]);
    for segment in segments {
        out.extend_from_slice(segment);
    }
    out.extend_from_slice(&encoded[2..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_animation_markers() {
        let mut gif = Vec::new();
        gif.extend_from_slice(b"GIF89a");
        gif.extend_from_slice(b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00");
        assert!(find(&gif, b"NETSCAPE2.0").is_some());

        let png = b"\x89PNG\r\n\x1a\n....acTL....";
        assert!(find(png, b"acTL").is_some());

        let still = b"\x89PNG\r\n\x1a\n....IHDR....";
        assert!(find(still, b"acTL").is_none());
    }

    #[test]
    fn segment_parse_stops_at_scan_and_keeps_what_it_should() {
        // SOI, APP1, APP0, SOS
        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x08, 1, 2, 3, 4, 5, 6]); // APP1 len 8
        jpeg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 9, 9]); // APP0 len 4
        jpeg.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02]); // SOS
        jpeg.extend_from_slice(&[0x00, 0x11, 0x22]);

        let dir = std::env::temp_dir();
        let path = dir.join("imgsmith_segtest.jpg");
        std::fs::write(&path, &jpeg).unwrap();

        let kept = jpeg_app_segments(&path, true);
        assert_eq!(kept.len(), 1, "only APP1 should be kept");
        assert_eq!(kept[0][1], 0xE1);

        let none = jpeg_app_segments(&path, false);
        assert!(none.is_empty(), "EXIF dropped when orientation consumed");

        let spliced = splice_jpeg(&jpeg, &kept);
        assert_eq!(&spliced[..2], &[0xFF, 0xD8]);
        assert_eq!(spliced[2..4], [0xFF, 0xE1]);
        let _ = std::fs::remove_file(&path);
    }
}
