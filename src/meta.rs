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

/// What a photo says about itself, in the words a person would use.
#[derive(Debug, Default, PartialEq)]
pub struct Tags {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    /// EXIF text, "YYYY:MM:DD HH:MM:SS". Left as written, converted on the way out.
    pub taken: Option<String>,
    pub software: Option<String>,
    pub orientation: Option<u8>,
    pub iso: Option<u32>,
    pub f_number: Option<f64>,
    /// Already rendered, as "1/250 s" or "2 s".
    pub exposure: Option<String>,
    /// Decimal degrees, north and east positive.
    pub gps: Option<(f64, f64)>,
}

impl Tags {
    pub fn is_empty(&self) -> bool {
        *self == Tags::default()
    }
}

/// Read the tags of a file, or nothing when it carries none.
pub fn read_tags(path: &Path) -> Option<Tags> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;

    let text = |tag| {
        exif.get_field(tag, exif::In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string())
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
    };
    let number = |tag| {
        exif.get_field(tag, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
    };
    let rational = |tag| {
        exif.get_field(tag, exif::In::PRIMARY)
            .and_then(|f| match f.value {
                exif::Value::Rational(ref v) if !v.is_empty() => {
                    Some(v[0].num as f64 / v[0].denom as f64)
                }
                _ => None,
            })
    };

    let exposure = exif
        .get_field(exif::Tag::ExposureTime, exif::In::PRIMARY)
        .and_then(|f| match f.value {
            exif::Value::Rational(ref v) if !v.is_empty() => {
                let (num, den) = (v[0].num, v[0].denom);
                Some(if num == 1 && den > 1 {
                    format!("1/{den} s")
                } else if den == 0 {
                    String::new()
                } else if num < den {
                    format!("1/{} s", (den as f64 / num as f64).round() as i64)
                } else {
                    format!("{:.1} s", num as f64 / den as f64)
                })
            }
            _ => None,
        })
        .filter(|s| !s.is_empty());

    let gps = match (
        coordinate(&exif, exif::Tag::GPSLatitude),
        coordinate(&exif, exif::Tag::GPSLongitude),
    ) {
        (Some(lat), Some(lon)) => Some((
            signed(&exif, exif::Tag::GPSLatitudeRef, lat),
            signed(&exif, exif::Tag::GPSLongitudeRef, lon),
        )),
        _ => None,
    };

    let tags = Tags {
        make: text(exif::Tag::Make),
        model: text(exif::Tag::Model),
        lens: text(exif::Tag::LensModel),
        taken: text(exif::Tag::DateTimeOriginal).or_else(|| text(exif::Tag::DateTime)),
        software: text(exif::Tag::Software),
        orientation: number(exif::Tag::Orientation).map(|v| v as u8),
        iso: number(exif::Tag::PhotographicSensitivity),
        f_number: rational(exif::Tag::FNumber),
        exposure,
        gps,
    };
    if tags.is_empty() { None } else { Some(tags) }
}

/// Degrees, minutes and seconds as one number.
fn coordinate(exif: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    let field = exif.get_field(tag, exif::In::PRIMARY)?;
    let parts: Vec<f64> = match &field.value {
        exif::Value::Rational(v) if !v.is_empty() => {
            v.iter().map(|r| r.num as f64 / r.denom as f64).collect()
        }
        _ => return None,
    };
    let degrees = parts.first().copied()?;
    let minutes = parts.get(1).copied().unwrap_or(0.0);
    let seconds = parts.get(2).copied().unwrap_or(0.0);
    Some(degrees + minutes / 60.0 + seconds / 3600.0)
}

fn signed(exif: &exif::Exif, ref_tag: exif::Tag, value: f64) -> f64 {
    let south_or_west = exif
        .get_field(ref_tag, exif::In::PRIMARY)
        .map(|f| {
            let s = f.display_value().to_string();
            s.contains('S') || s.contains('W')
        })
        .unwrap_or(false);
    if south_or_west { -value } else { value }
}

/// Turn "2026:03:01 14:22:31" into the shape a template asks for.
///
/// The tokens are the strftime ones people already know, limited to the six that
/// a date from a photograph needs. An unknown token is left alone rather than
/// silently dropped, so a typo shows up in the name instead of disappearing.
pub fn format_date(taken: &str, template: &str) -> Option<String> {
    let bytes = taken.as_bytes();
    // The tag holds 2026:03:01, and some readers hand it over already turned into
    // 2026-03-01. Both are the same date, so both are accepted.
    let separator = |i: usize| matches!(bytes.get(i), Some(b':') | Some(b'-'));
    if bytes.len() < 19 || !separator(4) || !separator(7) {
        return None;
    }
    let part = |start: usize, len: usize| -> Option<&str> { taken.get(start..start + len) };
    let year = part(0, 4)?;
    let month = part(5, 2)?;
    let day = part(8, 2)?;
    let hour = part(11, 2)?;
    let minute = part(14, 2)?;
    let second = part(17, 2)?;

    let mut out = String::with_capacity(template.len() + 8);
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('Y') => out.push_str(year),
            Some('m') => out.push_str(month),
            Some('d') => out.push_str(day),
            Some('H') => out.push_str(hour),
            Some('M') => out.push_str(minute),
            Some('S') => out.push_str(second),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_reformatted_from_exif_text() {
        let taken = "2026:03:01 14:22:31";
        assert_eq!(
            format_date(taken, "%Y-%m-%d_%H-%M-%S").as_deref(),
            Some("2026-03-01_14-22-31")
        );
        assert_eq!(
            format_date(taken, "%Y/%m/%d").as_deref(),
            Some("2026/03/01")
        );
        assert_eq!(
            format_date(taken, "photo-%Y%m%d-%H%M").as_deref(),
            Some("photo-20260301-1422")
        );
        // An unknown token stays visible, so a typo in the template is not silent.
        assert_eq!(format_date(taken, "%Y_%q").as_deref(), Some("2026_%q"));
        // A reader that already normalised the separators is the same date.
        assert_eq!(
            format_date("2026-03-01 14:22:31", "%Y-%m-%d_%H-%M-%S").as_deref(),
            Some("2026-03-01_14-22-31")
        );
        // Not a date: no output rather than a garbage name.
        assert!(format_date("", "%Y").is_none());
        assert!(format_date("1 Jan 2026", "%Y").is_none());
    }

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
