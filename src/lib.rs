//! Image conversion, resizing, cropping and thumbnailing in one static binary,
//! with the same operations available as a library.
//!
//! The tool is deliberately narrow: eight operations that scripts actually do,
//! and a refusal with a reason for everything else. See the README for the
//! list of what it will not do and why.
//!
//! # File to file
//!
//! ```no_run
//! use std::path::Path;
//! use imgsmith::{process, Format, Options};
//!
//! let opts = Options { width: Some(1920), ..Options::default() };
//! let outcome = process(
//!     Path::new("photo.jpg"),
//!     Path::new("photo.webp"),
//!     Format::Webp,
//!     &opts,
//!     false,
//! )?;
//! println!("{}x{} to {}x{}", outcome.in_dims.0, outcome.in_dims.1,
//!          outcome.out_dims.0, outcome.out_dims.1);
//! # Ok::<(), imgsmith::Error>(())
//! ```
//!
//! # In memory
//!
//! ```
//! use imgsmith::{encodable, transform, DynamicImage, Fit, Format, Options};
//!
//! let img = DynamicImage::new_rgb8(200, 100);
//! let opts = Options { width: Some(100), fit: Fit::Inside, ..Options::default() };
//! let small = transform(img, &opts, None).unwrap();
//! assert_eq!((small.width(), small.height()), (100, 50));
//!
//! let png_bytes = encodable(&small, Format::Png).unwrap();
//! assert_eq!(&png_bytes[..4], b"\x89PNG");
//! ```

pub mod meta;
pub mod ops;

mod error;
mod img;

pub use error::Error;
pub use img::{
    Format, Options, Outcome, encodable, encode, load, process, transform, write_atomic,
};
pub use ops::{CropSpec, Fit, Flip, Gravity, orient, parse_crop, target_size};

/// The decoded image type every operation works on. Re-exported so a caller
/// cannot end up on a different `image` version than this crate.
pub use image::DynamicImage;

impl Default for Options {
    fn default() -> Self {
        Options {
            width: None,
            height: None,
            fit: Fit::Inside,
            crop: None,
            gravity: Gravity::Centre,
            rotate: None,
            flip: None,
            auto_orient: false,
            strip: false,
            quality: None,
            thumbnail: false,
            allow_upscale: true,
            max_pixels: 100_000_000,
        }
    }
}
