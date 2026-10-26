//! Geometry: target sizing, crop placement, EXIF orientation.
//!
//! Every decision here is arithmetic on integers, so it is unit-tested directly
//! rather than inferred from an encoded file.

use clap::ValueEnum;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Fit {
    /// Exact size. Aspect ratio is not preserved.
    Fill,
    /// Largest size that fits inside the box, aspect preserved, never exceeds it.
    Inside,
    /// Smallest size that covers the box, aspect preserved, one side overflows.
    Outside,
    /// Cover the box, aspect preserved, then centre-crop the overflow away.
    Cover,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Gravity {
    Centre,
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Flip {
    /// Mirror left to right.
    H,
    /// Mirror top to bottom.
    V,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CropSpec {
    pub width: u32,
    pub height: u32,
    /// Optional offset: `WxH+X+Y`.
    pub offset: Option<(i64, i64)>,
}

/// Parse `800x600` or `800x600+10+20` (offset may be negative).
pub fn parse_crop(raw: &str) -> Result<CropSpec, String> {
    let s = raw.trim();
    let (dims, offset) = match s.find(['+', '-']) {
        Some(idx) => (&s[..idx], Some(&s[idx..])),
        None => (s, None),
    };
    let (w, h) = dims
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("--crop expects WxH or WxH+X+Y, got {raw:?}"))?;
    let width: u32 = w
        .trim()
        .parse()
        .map_err(|_| format!("--crop width {w:?} is not a number"))?;
    let height: u32 = h
        .trim()
        .parse()
        .map_err(|_| format!("--crop height {h:?} is not a number"))?;
    if width == 0 || height == 0 {
        return Err("--crop width and height must be greater than zero".into());
    }
    let offset = match offset {
        None => None,
        Some(rest) => {
            // Split "±X±Y" keeping the signs.
            let mut parts = Vec::new();
            let mut cur = String::new();
            for ch in rest.chars() {
                if (ch == '+' || ch == '-') && !cur.is_empty() {
                    parts.push(cur.clone());
                    cur.clear();
                }
                cur.push(ch);
            }
            parts.push(cur);
            if parts.len() != 2 {
                return Err(format!("--crop offset {rest:?} expects +X+Y"));
            }
            let x: i64 = parts[0]
                .parse()
                .map_err(|_| format!("--crop offset X {:?} is not a number", parts[0]))?;
            let y: i64 = parts[1]
                .parse()
                .map_err(|_| format!("--crop offset Y {:?} is not a number", parts[1]))?;
            Some((x, y))
        }
    };
    Ok(CropSpec {
        width,
        height,
        offset,
    })
}

/// Target dimensions for the requested box.
///
/// `allow_upscale == false` clamps the result to the source size, which is what
/// thumbnailing wants and what a resize usually does not.
#[must_use]
pub fn target_size(
    src: (u32, u32),
    want: (Option<u32>, Option<u32>),
    fit: Fit,
    allow_upscale: bool,
) -> (u32, u32) {
    let (sw, sh) = (src.0.max(1), src.1.max(1));
    let (bw, bh) = want;
    let (w, h) = match (bw, bh) {
        (None, None) => (sw, sh),
        (Some(w), None) => {
            let scale = w as f64 / sw as f64;
            (w, round(sh as f64 * scale))
        }
        (None, Some(h)) => {
            let scale = h as f64 / sh as f64;
            (round(sw as f64 * scale), h)
        }
        (Some(w), Some(h)) => match fit {
            Fit::Fill => (w, h),
            Fit::Inside => {
                let scale = (w as f64 / sw as f64).min(h as f64 / sh as f64);
                (round(sw as f64 * scale), round(sh as f64 * scale))
            }
            Fit::Outside | Fit::Cover => {
                let scale = (w as f64 / sw as f64).max(h as f64 / sh as f64);
                (round(sw as f64 * scale), round(sh as f64 * scale))
            }
        },
    };
    if !allow_upscale {
        // Never enlarge: the scale is capped at 1.0, not merely clipped to the box.
        let scale = (w as f64 / sw as f64).min(h as f64 / sh as f64).min(1.0);
        (round(scale * sw as f64), round(scale * sh as f64))
    } else {
        (w.max(1), h.max(1))
    }
}

/// Placement rectangle for a crop, clamped to the image.
///
/// With an explicit offset the box is used as given. Without one, `gravity`
/// decides where in the source the box sits.
pub fn crop_rect(
    src: (u32, u32),
    spec: &CropSpec,
    gravity: Gravity,
) -> Result<(u32, u32, u32, u32), String> {
    let (sw, sh) = src;
    if spec.width > sw || spec.height > sh {
        return Err(format!(
            "crop {}x{} is larger than the image {}x{}",
            spec.width, spec.height, sw, sh
        ));
    }
    let (mut x, mut y) = match spec.offset {
        Some((x, y)) => (x, y),
        None => {
            let dx = sw - spec.width;
            let dy = sh - spec.height;
            let cx = match gravity {
                Gravity::West | Gravity::NorthWest | Gravity::SouthWest => 0,
                Gravity::East | Gravity::NorthEast | Gravity::SouthEast => dx,
                _ => dx / 2,
            };
            let cy = match gravity {
                Gravity::North | Gravity::NorthWest | Gravity::NorthEast => 0,
                Gravity::South | Gravity::SouthWest | Gravity::SouthEast => dy,
                _ => dy / 2,
            };
            (cx as i64, cy as i64)
        }
    };
    // Clamp: a box that starts outside the image is pulled back inside.
    x = x.clamp(0, (sw - spec.width) as i64);
    y = y.clamp(0, (sh - spec.height) as i64);
    Ok((x as u32, y as u32, spec.width, spec.height))
}

/// EXIF orientation tag to the transform that displays the image upright.
///
/// 1 is already upright. 2-8 are the mirrored and rotated cases; 5 and 7 are
/// the two transposes, which are a rotation plus a mirror.
#[must_use]
pub fn orient(img: image::DynamicImage, orientation: u8) -> image::DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn round(v: f64) -> u32 {
    v.round().max(1.0) as u32
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on a failed assertion")]
    use super::*;

    #[test]
    fn inside_preserves_aspect_and_never_exceeds() {
        assert_eq!(
            target_size((4000, 3000), (Some(1920), Some(1920)), Fit::Inside, true),
            (1920, 1440)
        );
        assert_eq!(
            target_size((3000, 4000), (Some(1920), Some(1920)), Fit::Inside, true),
            (1440, 1920)
        );
    }

    #[test]
    fn outside_covers_the_box() {
        let (w, h) = target_size((4000, 3000), (Some(1920), Some(1920)), Fit::Outside, true);
        assert!(w >= 1920 && h >= 1920);
        assert_eq!((w, h), (2560, 1920));
    }

    #[test]
    fn fill_is_exact() {
        assert_eq!(
            target_size((4000, 3000), (Some(800), Some(800)), Fit::Fill, true),
            (800, 800)
        );
    }

    #[test]
    fn single_dimension_keeps_aspect() {
        assert_eq!(
            target_size((4000, 3000), (Some(1000), None), Fit::Inside, true),
            (1000, 750)
        );
        assert_eq!(
            target_size((4000, 3000), (None, Some(750)), Fit::Inside, true),
            (1000, 750)
        );
    }

    #[test]
    fn no_upscale_never_grows() {
        assert_eq!(
            target_size((200, 100), (Some(400), Some(400)), Fit::Inside, false),
            (200, 100)
        );
    }

    #[test]
    fn crop_offsets_parse() {
        assert_eq!(
            parse_crop("800x600").unwrap(),
            CropSpec {
                width: 800,
                height: 600,
                offset: None
            }
        );
        assert_eq!(
            parse_crop("800x600+10+20").unwrap(),
            CropSpec {
                width: 800,
                height: 600,
                offset: Some((10, 20))
            }
        );
        assert_eq!(
            parse_crop("800x600-5-20").unwrap(),
            CropSpec {
                width: 800,
                height: 600,
                offset: Some((-5, -20))
            }
        );
        assert!(parse_crop("800").is_err());
        assert!(parse_crop("0x10").is_err());
    }

    #[test]
    fn crop_gravity_places_the_box() {
        let spec = CropSpec {
            width: 100,
            height: 100,
            offset: None,
        };
        assert_eq!(
            crop_rect((1000, 800), &spec, Gravity::Centre).unwrap(),
            (450, 350, 100, 100)
        );
        assert_eq!(
            crop_rect((1000, 800), &spec, Gravity::NorthWest).unwrap(),
            (0, 0, 100, 100)
        );
        assert_eq!(
            crop_rect((1000, 800), &spec, Gravity::SouthEast).unwrap(),
            (900, 700, 100, 100)
        );
    }

    #[test]
    fn crop_offset_is_clamped_inside() {
        let spec = CropSpec {
            width: 100,
            height: 100,
            offset: Some((5000, 5000)),
        };
        assert_eq!(
            crop_rect((1000, 800), &spec, Gravity::Centre).unwrap(),
            (900, 700, 100, 100)
        );
        assert!(crop_rect((50, 50), &spec, Gravity::Centre).is_err());
    }

    #[test]
    fn orientation_six_swaps_dimensions() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(200, 100));
        let out = orient(img, 6);
        assert_eq!((out.width(), out.height()), (100, 200));
    }

    #[test]
    fn orientation_one_is_unchanged() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(200, 100));
        let out = orient(img, 1);
        assert_eq!((out.width(), out.height()), (200, 100));
    }
}
