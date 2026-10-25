# imgsmith

Convert, resize, crop and thumbnail images from one static binary. No delegates,
no plugins, no ImageMagick install, no PDF, no SVG, no animation.

```console
$ imgsmith photo.jpg -o photo.webp --width 1920
$ imgsmith photos/ -o thumbs/ --thumb 400
```

## Why this exists

ImageMagick is 25 packages and 67 MB on Ubuntu 26.04, it delegates PDF to
Ghostscript, and it carries 923 entries in the NVD. The Rust decoders for every
common format have existed for years and are solid. What was missing was the
command line tool around them, so this is that tool, deliberately narrow.

## What it does

| Operation | Flag | Example |
|---|---|---|
| Format conversion | output extension or `--format` | `imgsmith in.png out.jpg` |
| Resize | `--width`, `--height`, `--fit` | `imgsmith in.jpg -o out.jpg -w 1920` |
| Crop | `--crop`, `--gravity` | `imgsmith in.jpg -o out.jpg --crop 800x600` |
| Rotate and mirror | `--rotate`, `--flip` | `imgsmith in.jpg -o out.jpg --rotate 90` |
| EXIF auto-orient | `--auto-orient` | `imgsmith phone.jpg -o out.jpg --auto-orient` |
| Thumbnail | `--thumb` | `imgsmith in.jpg -o t.jpg --thumb 400` |
| Batch | a directory as input | `imgsmith shots/ -o web/ --width 1600` |
| Quality | `--quality` | `imgsmith in.png out.avif --quality 60` |

Order of operations is fixed and documented because it changes the result:
orient, then crop, then resize, then rotate and mirror. Crop coordinates are
therefore always in the pixels of the upright original.

`--fit` takes four values:

| Value | Result |
|---|---|
| `inside` (default) | fits inside the box, aspect preserved |
| `outside` | covers the box, one side overflows |
| `fill` | exact size, aspect not preserved |
| `cover` | covers the box, then crops the overflow away |

## What it refuses, and why

This list is the design. Each refusal removes a class of bug rather than a
feature.

| Refused | Reason |
|---|---|
| PDF, PostScript, EPS | those need Ghostscript as a delegate, which is the highest risk part of ImageMagick's surface. Use a PDF tool |
| SVG | resvg already does this well |
| Animated GIF, APNG, animated WebP | refused with a message, never silently flattened to the first frame. Use gifski for GIF |
| HEIC, and AVIF **input** | both need a C decoder, and one C dependency defeats the point. AVIF **output** is supported |
| Filters, effects, drawing, text | no demand evidence in the top twenty ImageMagick questions, and text shaping is a subsystem |
| Colour management, ICC transforms | invisible when right, undebuggable when wrong |
| Plugins, delegates, a scripting language | the architecture that produced 923 vulnerabilities |
| Network or URL input | an SSRF surface for no user benefit |

## Formats

| | Formats |
|---|---|
| Read | JPEG, PNG, WebP (still), TIFF, GIF (still), BMP, ICO, TGA, PNM, QOI, HDR, OpenEXR, farbfeld |
| Write | the same, plus AVIF |

PNG output is deflate compressed at the default level, which is why a 12 MP
conversion is faster than ImageMagick's and about 10% larger. TIFF output is
uncompressed, because the encoder in use has no compression setting. WebP output is **lossless only**:
passing `--quality` with WebP is an error rather than a number that silently
does nothing.

## Metadata

The tool re-encodes pixels and copies only what it can copy correctly.

| Case | Behaviour |
|---|---|
| JPEG in, JPEG out | EXIF, IPTC and ICC are carried over |
| `--strip` | nothing is carried, output has no metadata |
| `--auto-orient` | the image is rotated and the EXIF block is dropped, because the orientation tag would then be wrong. ICC survives |
| Any other format pair | no metadata is carried in this version |

## Safety

- Images over 100 megapixels are refused before decoding (`--max-mp` to change,
  `0` to disable). This is a decompression bomb guard.
- Existing files are never overwritten without `--force`.
- Output is written to a temporary file and renamed, so a failed encode cannot
  leave a half written image.
- No `unsafe` code in this crate, no C dependencies, no child processes.

## Measured

Ubuntu 26.04, 4 cores, a 4032x3024 JPEG (3.8 MB) from `plasma:fractal`, median
of three runs, ImageMagick 7.1.2 Q16 for comparison.

| Task | imgsmith | ImageMagick |
|---|---|---|
| Resize to 1920 wide, quality 85 | 1.12 s, 592 KB | 1.20 s, 583 KB |
| Thumbnail 400 | 0.28 s | 0.37 s |
| JPEG to PNG, 12 MP | 5.3 s, 18.5 MB | 8.4 s, 16.9 MB |
| Eight files to 1600 wide | 2.07 s with 4 workers, 7.48 s with 1 | not measured |
| Program start (`--version`) | 5 ms | 10 ms |
| Peak memory, 12 MP resize | 145 MB | 132 MB |
| Footprint | 5.8 MB binary, links libc, libm and libgcc | 25 packages, 67 MB |

Speed is not the pitch. The resize margin is about 7%, which is inside
run-to-run noise on this machine, the thumbnail margin is about 25%, and memory
use is slightly worse. What changes is what has to be installed and what code parses
untrusted bytes.

## Install

```console
$ cargo install imgsmith
```

Or from source: `cargo build --release`, the binary is `target/release/imgsmith`.

## Use it as a library

The binary is a thin caller over the same crate.

```rust
use imgsmith::{process, Format, Options};

let opts = Options { width: Some(1920), ..Options::default() };
let outcome =
    process("photo.jpg".as_ref(), "photo.webp".as_ref(), Format::Webp, &opts, false).unwrap();
println!("{}x{}", outcome.out_dims.0, outcome.out_dims.1);
```

In memory, without touching the filesystem:

```rust
use imgsmith::{encodable, transform, DynamicImage, Fit, Format, Options};

let img = DynamicImage::new_rgb8(200, 100);
let opts = Options { width: Some(100), fit: Fit::Inside, ..Options::default() };
let small = transform(img, &opts, None).unwrap();
let png_bytes = encodable(&small, Format::Png).unwrap();
```

`imgsmith::DynamicImage` is re-exported from `image`, so a caller cannot end up
on a different version than the crate was built against.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | every file converted |
| 1 | one or more files failed, the reason is on stderr per file |
| 2 | bad usage |

## Verified

| Check | Result |
|---|---|
| Tests | 44: 12 unit, 8 public API, 22 end-to-end against the binary, 2 doctests |
| Real camera files | 21 files covering all eight EXIF orientations and five vendors, dimensions and an 8x8 colour grid matched against ImageMagick |
| Formats | 14 written and 13 read back, every output opened by ImageMagick |
| Awkward inputs | CMYK, progressive, 16-bit, greyscale and alpha all convert; a corrupted file is refused with a named error |

Both checks are in the repo and run against the release binary:
`scripts/verify_formats.sh` and `scripts/verify_real_world.py`.

## Licence

Apache-2.0.
