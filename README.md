# imgsmith

Convert, resize, crop and thumbnail images from one static binary. No delegates,
no plugins, no ImageMagick install.

```console
$ imgsmith photo.jpg -o photo.webp --width 1920
$ imgsmith photos/ -o thumbs/ --thumb 400
```

## Install

```console
$ cargo install imgsmith
$ brew install marceanlabs/imgsmith/imgsmith
```

Or a prebuilt binary for Linux, macOS or Windows from the
[releases page](https://github.com/marceanlabs/imgsmith/releases).

## What it does

| Operation | Flag |
|---|---|
| Format conversion | output extension or `--format` |
| Resize | `--width`, `--height`, `--fit` |
| Crop | `--crop`, `--gravity` |
| Rotate and mirror | `--rotate`, `--flip` |
| EXIF auto-orient | `--auto-orient` |
| Thumbnail | `--thumb` |
| Batch | a directory as input |
| Quality | `--quality` |
| Read the metadata | `--info` |
| Rename by date taken | `--rename-by-date` |

The order is fixed: orient, crop, resize, rotate. Crop coordinates are always
pixels of the upright original.

## The metadata it can read

```console
$ imgsmith photo.jpg --info
file:  photo.jpg
size:  4032x3024
camera: Canon EOS R5
lens:  RF24-105mm F4 L IS USM
taken: 2026-03-01 14:22:31
shot:  1/250 s  f/4  ISO 400
gps:   51.507389, -0.127800

$ imgsmith photos/ --rename-by-date --dry-run
would rename photos/IMG_0431.jpg -> photos/2026-03-01_14-22-31.jpg
```

`--rename-by-date` takes a template: `--date-format "%Y/%m/%d"` would name the
file after a folder that has to exist, and `%Y%m%d-%H%M` is the six tokens
supported. A file with no date is left where it is and reported, never renamed
after its file time, because a wrong date in a name is worse than no change.

## What it refuses

PDF and PostScript need Ghostscript, and SVG is resvg's job. Animated GIF, APNG
and animated WebP are refused with a message rather than flattened to one frame.
HEIC and AVIF input need a C decoder; AVIF output is supported. No filters,
effects, text or colour management, no plugins, no network input.

Reads JPEG, PNG, WebP, TIFF, GIF, BMP, ICO, TGA, PNM, QOI, HDR, OpenEXR and
farbfeld. Writes the same plus AVIF. WebP output is lossless only, so `--quality`
with WebP is an error rather than a number that does nothing.

## Measured

Ubuntu 26.04, four cores, a 4032x3024 JPEG, median of three, against ImageMagick
7.1.2. `scripts/bench.sh` regenerates it.

| Task | imgsmith | ImageMagick |
|---|---|---|
| Resize to 1920 wide | 1.00 s | 1.13 s |
| Thumbnail 400 | 0.18 s | 0.29 s |
| JPEG to PNG, 12 MP | 6.65 s | 11.80 s |
| Footprint | 6.6 MB binary | 25 packages, 67 MB |

Speed is not the pitch. The resize margin is 11%, memory use is slightly worse,
and JPEG output at the same quality number is about half as large again because
it keeps 4:4:4 chroma. What changes is what has to be installed, and what code
parses untrusted bytes.

## As a library

```rust
use imgsmith::{process, Format, Options};

let opts = Options { width: Some(1920), ..Options::default() };
process("photo.jpg".as_ref(), "photo.webp".as_ref(), Format::Webp, &opts, false)?;
```

Files are never overwritten without `--force`, output is written to a temporary
file and renamed, and images over 100 megapixels are refused before decoding. 44
tests and a nightly fuzz run are in the repo.

## Licence

Apache-2.0.
