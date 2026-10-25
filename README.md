# imgsmith

One program to convert, resize, crop and thumbnail images. There is nothing else
to install.

```console
$ imgsmith photo.jpg -o photo.webp --width 1920
$ imgsmith photos/ -o thumbs/ --thumb 400
```

imgsmith is a single file. It does not need an image library installed first, a
plugin folder, or a service running in the background. Copy it onto a machine,
run it, and it works.

## Install

```console
$ cargo install imgsmith
$ brew install marceanlabs/imgsmith/imgsmith
```

Prebuilt binaries for Linux, macOS and Windows are on the
[releases page](https://github.com/marceanlabs/imgsmith/releases).

## What it does

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

The work always happens in the same order: turn the photo upright, crop it, resize
it, rotate it. That order is fixed on purpose, so a crop always refers to the
photo the way you see it rather than the way the camera happened to save it.

## Reading what a file says about itself

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

## Renaming by the date the photo was taken

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

## What it will not do

PDF and PostScript are not supported. Those need Ghostscript, and imgsmith
deliberately has nothing behind it. SVG is a set of drawing instructions rather
than a picture, so that is out too.

A GIF, APNG or animated WebP is a series of frames. imgsmith handles single
images, so it stops with a message rather than quietly saving the first frame.

HEIC and AVIF photos straight from a phone cannot be opened, because they need a C
decoder. Writing AVIF works.

There are no filters, effects, text or colour management. No plugins, and no
network access.

## Formats

Reads JPEG, PNG, WebP, TIFF, GIF, BMP, ICO, TGA, PNM, QOI, HDR, OpenEXR and
farbfeld. Writes all of those plus AVIF.

WebP output is lossless, so `--quality` with WebP is an error rather than a number
that gets quietly ignored.

## How fast it is

Ubuntu 26.04, four cores, one 4032x3024 JPEG, median of three runs, against
ImageMagick 7.1.2. `scripts/bench.sh` reproduces the table.

| Task | imgsmith | ImageMagick |
|---|---|---|
| Resize to 1920 wide | 1.00 s | 1.13 s |
| Thumbnail 400 | 0.18 s | 0.29 s |
| JPEG to PNG, 12 megapixels | 6.65 s | 11.80 s |
| Installed size | one 6.6 MB file | 25 packages, 67 MB |

The table is not all one way, and it is worth saying so. The resize win is 11%.
Memory use is slightly worse. A JPEG written at the same quality setting comes out
larger, because imgsmith keeps the full colour detail instead of discarding part
of it.

What changes is the install: one file against 25 packages, and one piece of code
reading untrusted files instead of a stack of them.

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

The test suite runs on every change, and a nightly job feeds the decoder random
and damaged files to check that it fails safely instead of crashing.

## Licence

Apache-2.0.
