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

AVIF output sits behind the default-on `avif` feature. The encoder is 40 crates
and 571,000 lines, half of this crate's dependency tree, so a library user who
never writes AVIF can drop it:

```console
$ cargo install imgsmith --no-default-features   # 70 packages, 3.6 MB binary
```

Without the feature, `--format avif` fails with a message naming the flag rather
than writing something unexpected.

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

Ubuntu 26.04, four cores, a 4032x3024 JPEG from a seeded `plasma:fractal` so the
source file is byte identical on every run, median of three runs, ImageMagick
7.1.2 Q16 for comparison. Measured 25 October 2026 against imgsmith 0.1.1.

| Task | imgsmith | ImageMagick |
|---|---|---|
| Resize to 1920 wide, quality 85 | 1.00 s, 556 KB | 1.13 s, 364 KB |
| Thumbnail 400 | 0.18 s | 0.29 s |
| JPEG to PNG, 12 MP | 6.65 s, 15.0 MB | 11.80 s, 14.0 MB |
| Eight files to 1600 wide | 1.81 s with 4 workers, 6.65 s with 1 | not measured |
| Program start (`--version`) | 18 ms | 28 ms |
| Peak memory, 12 MP resize | 138 MB | 128 MB |
| Footprint | 5.8 MB binary, links libc, libm and libgcc | 25 packages, 67 MB on a clean Ubuntu (see below) |

`scripts/bench.sh` regenerates that table; a number that cannot be reproduced
by running it does not belong here. The ImageMagick footprint is the dependency
closure of `apt-get -s install imagemagick` on a clean 26.04, measured before
ImageMagick was installed on this machine; on a box that already has it the
closure reads higher and is not comparable.

Two things in that table need saying plainly:

- **JPEG output at the same quality number is larger, by about half on this
  image.** imgsmith writes 4:4:4 chroma, ImageMagick writes 4:2:0 at quality 85,
  so the file keeps full colour resolution and costs bytes. There is no flag to
  choose subsampling in this version.
- **Speed is not the pitch.** The resize margin is 11%, the thumbnail margin is
  38%, PNG is 44% faster but 7% larger, and memory use is slightly worse. What
  changes is what has to be installed and what code parses untrusted bytes.

## Install

```console
$ cargo install imgsmith
$ brew install marceanlabs/imgsmith/imgsmith
```

Or download a prebuilt binary for Linux, macOS or Windows from the
[releases page](https://github.com/marceanlabs/imgsmith/releases), or build from
source with `cargo build --release`.

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
| Fuzzing | `cargo +nightly fuzz run decode`, seeded from six real formats, run nightly in CI |

Both checks are in the repo and run against the release binary:
`scripts/verify_formats.sh` and `scripts/verify_real_world.py`.

## Fuzzing

Every byte the tool reads is a file from somewhere untrusted, so the decode path
is fuzzed rather than only tested:

```console
$ cargo install cargo-fuzz          # once, plus: rustup toolchain install nightly
$ mkdir -p fuzz/corpus/decode && cp fuzz/seeds/* fuzz/corpus/decode/
$ cargo +nightly fuzz run decode -- -max_total_time=900
```

The target writes the input to a file and then takes the same path the binary
takes: format sniffing, decode, the EXIF and animation probes, one thumbnail
transform, three encoders. Seeds in `fuzz/seeds/` are real PNG, JPEG, GIF, BMP,
WebP and TIFF files, so the fuzzer starts inside the parsers.

`.github/workflows/fuzz.yml` runs it for fifteen minutes a night and uploads the
crashing input as a build artefact when one is found. A crash is not a proof of a
bug in this crate: the parsers come from `image`, so the first step is to
reproduce it, `cargo fuzz tmin` it, and report it upstream if the fault is theirs.

## Licence

Apache-2.0.
