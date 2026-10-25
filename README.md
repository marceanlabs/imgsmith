# imgsmith

[![crates.io](https://img.shields.io/crates/v/imgsmith.svg)](https://crates.io/crates/imgsmith)
[![ci](https://github.com/marceanlabs/imgsmith/actions/workflows/ci.yml/badge.svg)](https://github.com/marceanlabs/imgsmith/actions/workflows/ci.yml)

Convert, resize, crop and thumbnail images with one file and no other software.

```console
$ imgsmith photo.jpg -o photo.webp --width 1920
$ imgsmith photos/ -o thumbs/ --thumb 400
```

imgsmith is a single file. It does not need an image library installed first, a
plugin folder, or a service running in the background. Copy it onto a machine,
run it, and it works.

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [What it will not do](#what-it-will-not-do)
- [Performance](#performance)
- [Using it from Rust](#using-it-from-rust)
- [Contributing](#contributing)
- [Licence](#licence)

## Install

```console
$ cargo install imgsmith
$ brew install marceanlabs/imgsmith/imgsmith
```

Building from source needs Rust 1.85 or newer. Prebuilt binaries are on the
[releases page](https://github.com/marceanlabs/imgsmith/releases):

| Platform | Target |
|---|---|
| Linux x86_64 | `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` |
| Linux arm64 | `aarch64-unknown-linux-gnu` |
| macOS, Apple silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

The musl build is a fully static file, so it runs on any Linux distribution
whatever its C library.

## Usage

| Job | How you ask for it |
|---|---|
| Change the format | name the output file `.webp`, or pass `--format` |
| Resize | `--width`, `--height`, `--fit` |
| Crop | `--crop`, `--gravity` |
| Rotate or mirror | `--rotate`, `--flip` |
| Turn a photo the right way up | `--auto-orient` |
| Make a thumbnail | `--thumb` |
| Work through a folder | give a folder as the input |
| Choose the quality | `--quality` |
| Read the camera data | `--info` |
| Rename files by the date taken | `--rename-by-date` |

It reads JPEG, PNG, WebP, TIFF, GIF, BMP, ICO, TGA, PNM, QOI, HDR, OpenEXR and
farbfeld, and writes all of those plus AVIF.

The work always happens in the same order: turn the photo upright, crop it, resize
it, rotate it. That order is fixed on purpose, so a crop always refers to the
photo the way you see it rather than the way the camera happened to save it.

### Reading what a file says about itself

```console
$ imgsmith photo.jpg --info
file:  photo.jpg
size:  4032x3024
camera: Canon EOS R5
lens:  RF24-105mm F4 L IS USM
taken: 2026-03-01 14:22:31
shot:  1/250 s  f/4  ISO 400
gps:   51.507389, -0.127800
```

### Renaming by the date the photo was taken

```console
$ imgsmith photos/ --rename-by-date --dry-run
would rename photos/IMG_0431.jpg -> photos/2026-03-01_14-22-31.jpg
```

The default name is the date and time. You can ask for a different shape with
`--date-format`, which understands six tokens:

| Token | Becomes |
|---|---|
| `%Y` | year, 2026 |
| `%m` | month, 03 |
| `%d` | day, 01 |
| `%H` | hour, 14 |
| `%M` | minute, 22 |
| `%S` | second, 31 |

Anything else in the template is copied as written, so `photo-%Y%m%d-%H%M` gives
`photo-20260301-1422.jpg`. If a name is already taken, a number is added to the
end of it.

A file with no date in it is left where it is and reported. imgsmith will not fall
back to the file's modified time, because a wrong date in a filename is worse than
no change at all.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | every file was written |
| 1 | at least one file could not be processed |
| 2 | the arguments were wrong, so nothing was done |

## What it will not do

PDF and PostScript are not supported. Those need Ghostscript, and imgsmith
deliberately has nothing behind it. SVG is a set of drawing instructions rather
than a picture, so that is out too.

A GIF, APNG or animated WebP is a series of frames. imgsmith handles single
images, so it stops with a message rather than quietly saving the first frame.

HEIC and AVIF photos straight from a phone cannot be opened, because they need a C
decoder. Writing AVIF works.

WebP output is lossless, so `--quality` with WebP is an error rather than a number
that gets quietly ignored.

There are no filters, effects, text or colour management. No plugins, and no
network access.

## Performance

On a 12 megapixel JPEG resized to 1920 wide, imgsmith is 11% quicker than
ImageMagick 7.1.2. The install is where the gap is widest: one 6.6 MB file against
25 packages and 67 MB.

The full table, the method, and the two places where it is not the faster tool are
in [BENCHMARKS.md](BENCHMARKS.md).

## Using it from Rust

Every command is also a function.

```rust
use imgsmith::{process, Format, Options};

let opts = Options { width: Some(1920), ..Options::default() };
process("photo.jpg".as_ref(), "photo.webp".as_ref(), Format::Webp, &opts, false)?;
```

Files are never overwritten without `--force`. Output is written to a temporary
file and renamed into place when it is complete, so an interrupted run cannot
leave a half written image. Images over 100 megapixels are refused before decoding
starts.

## Contributing

Questions and bug reports go in the [issues](https://github.com/marceanlabs/imgsmith/issues).
Please open one before writing a patch: imgsmith is kept small on purpose, so a
change that makes it do one more thing usually will not be taken, while a change
that makes it do the same thing correctly will.

Security problems should go through [private reporting](https://github.com/marceanlabs/imgsmith/security/advisories/new)
rather than a public issue.

## Licence

Apache-2.0.
