# Image CLI — scope decision

Written 25 October 2026. Evidence gathered the same day, all figures measured or pulled
from the source API in this session, not quoted from blogs.

Working name: not chosen yet. Directory: `/root/work/imgtool/`.

## Verdict

Build a single-binary CLI that does the eight operations people actually script, and
refuse everything else. The gap is the **tool layer**, not the codecs: Rust already has
mature decoders for every common format, and the one serious replacement attempt
(wondermagick) scoped itself to convert-and-resize, called that "the most common workload
for imagemagick", then stalled in June 2026 with 10 open issues and 119 stars.

Security is the wedge, and it is a measured one, not a slogan: ImageMagick's own history
is 923 entries in the NVD against 32 for libvips. A single static binary that never loads
a delegate also removes the install pain, which is the second most-asked ImageMagick
question on Stack Overflow.

## Method, and what it cannot see

There is no public telemetry for "which ImageMagick operation gets run". Every usage
figure below is a proxy, and the proxies are graded:

| Signal | What it really measures | Grade |
|---|---|---|
| Stack Overflow question counts per verb | where people get stuck, which correlates with use but is not use | B2 |
| Library download volume (PyPI, npm, crates.io) | how often the job is automated in code; includes CI, mirrors and bots | A1 for the number, C for "users" |
| Wondermagick's stated scope | one funded expert's judgement of the common workload | B2 |
| libvips shipping `vipsthumbnail` as a separate binary | thumbnailing is a distinct first-class job, not a feature | B1 |
| ImageMagick install weight, measured here | deployment cost, exactly | A1 |

Limits stated plainly: counts are all-time question totals, not per-year; npm downloads
were taken for the week ending 22 Oct 2026 and PyPI for the last 30 days; crates figures
are total and trailing-90-day. None of these can distinguish a hundred users running
something a million times from a million users running it once.

## The evidence

| Claim | Figure | Source | Date | Grade |
|---|---|---|---|---|
| Image work is enormous in every ecosystem | Pillow 436,470,920 downloads/month; sharp 76,118,281/week (≈327M/month) | pypistats.org, api.npmjs.org | 25 Oct 2026 | A1 |
| Rust has the engine already | `image` crate: 225.8M downloads total, 67.8M in the last 90 days | crates.io API | 25 Oct 2026 | A1 |
| The C incumbent is the security outlier | 923 NVD entries matching "ImageMagick", 32 for libvips, 128 for GraphicsMagick | NVD API keyword search | 25 Oct 2026 | A1 (keyword, so a floor and not vendor-scoped) |
| Installing it is a real cost | `apt-get install imagemagick` on Ubuntu 26.04 = 25 new packages, 67 MB installed | measured on this host | 25 Oct 2026 | A1 |
| Installation is the number-two pain | 2,521 SO questions "imagemagick install" vs 1,155 "imagemagick resize" | Stack Exchange API | 25 Oct 2026 | B2 |
| The serious Rust attempt picked convert+resize and stopped | 119 stars, 10 open issues, last push 2026-06-21; funded once by Sovereign Tech Agency, 91% of which went to image-rs | GitHub API, README | 25 Oct 2026 | B2 |
| Narrow single-job Rust image CLIs do get adopted | oxipng 4,295 stars, 596,218 crate downloads/90d; resvg 4,155 stars, 12.3M/90d | GitHub + crates.io | 25 Oct 2026 | A1 |
| Formats Rust can already do | AVIF, BMP, EXR, GIF(still), HDR, ICO, JPEG, PNG, PNM, QOI, TGA, TIFF, WebP | image crate README | 25 Oct 2026 | B1 |
| HEIC demand is real and out of reach | pillow-heif 18.6M/month; heic-convert 1.09M/week; heic-decode 1.68M/week | PyPI + npm APIs | 25 Oct 2026 | A1 |
| Both incumbents are major projects | ImageMagick 17,730 stars; libvips 11,762 stars | GitHub API | 25 Oct 2026 | A1 |

Operation mix, Stack Overflow all-time question totals:

| Verbatim query | Questions |
|---|---|
| imagemagick convert | 3,294 |
| imagemagick install | 2,521 |
| imagemagick pdf | 1,378 |
| imagemagick resize | 1,155 |
| imagemagick crop | 687 |
| imagemagick gif | 644 |
| imagemagick transparency | 591 |
| imagemagick tiff | 586 |
| imagemagick quality | 508 |
| imagemagick font | 363 |
| imagemagick memory | 352 |
| imagemagick batch | 327 |
| imagemagick draw | 303 |
| imagemagick composite | 232 |
| imagemagick webp | 223 |
| imagemagick exif | 148 |
| imagemagick watermark | 128 |
| imagemagick heic | 85 |
| imagemagick avif | 5 |

## In scope for v1

Each item earns its place from a row above. Nothing is here because ImageMagick has it.

| # | Operation | Why it is in | Command shape |
|---|---|---|---|
| 1 | Convert between formats | the single largest topic (3,294) and every other job ends in it | `img foo.png foo.webp` |
| 2 | Resize (width, height, one dimension, fit modes) | 1,155 questions; the operation sharp and Pillow are most used for | `--width 1920 --fit inside` |
| 3 | Crop (box, aspect ratio, gravity) | 687 questions; pairs with resize in every pipeline | `--crop 800x600 --gravity centre` |
| 4 | Rotate 90/180/270, flip, flop, and auto-orient from EXIF | the silent bug: phone photos arrive sideways and the EXIF tag is ignored by most scripts | `--auto-orient` |
| 5 | Thumbnail: fit in a box, strip metadata, quality preset, one pass over many files | libvips ships a dedicated `vipsthumbnail` binary for exactly this, which is the strongest available evidence that it is its own job | `--thumb 400` |
| 6 | Batch: many inputs, directory recursion, output dir, parallelism, refuse-to-overwrite default | 327 + 141 questions; single-file tools are what people already have | `in/ --out out/ --jobs 8` |
| 7 | Quality and effort controls (JPEG quality, WebP effort, AVIF speed) | 508 questions; the only knob that decides whether output is usable | `--quality 82` |
| 8 | Metadata: preserved by default, `--strip` removes EXIF/XMP/GPS | privacy is the common reason people reach for this; stripping by default would silently change results | `--strip` |

Cross-cutting, non-negotiable:

- One static binary, no shared-library dependency, no delegates, no plugins — the
  architecture that produced the CVE record is not copied.
- Exit codes that mean something, errors that name the file and the reason.
- No overwrite unless `--force`. A batch tool that eats originals is unrecoverable.
- Works on a 12-megapixel phone JPEG in well under a second, and streams large files
  rather than holding whole multi-hundred-megabyte images in RAM (352 questions on memory).

## Out of scope for v1, and why

| Left out | Why | What the user should use instead |
|---|---|---|
| PDF, PS, EPS in or out | Needs Ghostscript as a delegate, which is the highest-risk part of ImageMagick's surface. 1,378 questions exist because it is painful, not because it should be imitated | pdfsurgeon (same author, already shipped) |
| SVG rendering | resvg is mature, 4,155 stars, 12.3M downloads/90d. Competing with it is not a gap | resvg |
| Animated GIF, APNG, animated WebP | Multi-frame is a different data model and a different product. 644 questions, and gifski already answers them better | gifski |
| Filters and effects (blur, distort, morphology, artistic) | The long tail: hundreds of options, no top-20 demand evidence, each one a permanent compatibility obligation | ImageMagick, or the `image` crate directly |
| Compositing, montage, compare | 232 questions, and it is a different product with a different UX | ImageMagick |
| Drawing, vector text, fonts | 363 + 303 questions, but text shaping (kerning, fallback fonts, HiDPI) is a subsystem, not a flag. Revisit as one `watermark` command only if v1 lands | ImageMagick |
| Colour management, ICC profiles | Invisible when right, wrong in ways users cannot debug, and no demand evidence in the top 20 | leave it to libvips |
| HEIC/HEIF decoding | 18.6M/month of demand, but the only decoders are C (libheif). Shipping it breaks the one-binary promise. **This is the number-one v2 candidate** | pillow-heif, heic-convert |
| JPEG XL | `image` cannot do it, and the all-time question volume is negligible | libjxl |
| Delegates, plugins, a scripting language | This is the design decision that made ImageMagick unsafe and hard to install | nothing; it is refused on purpose |
| URLs or network input | SSRF surface for no user benefit | download first, then run the tool |
| GUI or TUI | Different product, different user | nothing |

## What would kill it

Written before the build, so it can be checked honestly later.

1. **Wondermagick resumes and ships releases.** If it puts out binaries with convert and
   resize in the next quarter, the gap closes. Then this becomes a differentiated batch
   and thumbnail tool or it does not happen.
2. **Crates downloads under ~5,000 in the first 90 days after launch.** CLI adoption in
   Rust is measurable and this is the floor that separates "used" from "published". Under
   it, the demand thesis is wrong for a CLI and the answer is a library or nothing.
3. **AVIF encode is too slow or too large in `image` (ravif).** Then AVIF drops out of v1
   rather than shipping something that takes 4 seconds a photo.
4. **The batch/thumbnail path is not meaningfully faster than `sharp-cli` or libvips.**
   Speed is not the pitch; safety and zero install are. If speed is also lost there is no
   reason to switch, and the honest output is "use libvips".

## What shipped, 25 October 2026

`imgsmith` 0.1.0, 34 tests, clippy clean, release binary 5.8 MB. Everything below is
measured, not intended.

**All eight in-scope operations exist** and are covered by end-to-end tests against the
real binary: conversion, resize with four fit modes, crop with gravity and offsets,
rotate and mirror, EXIF auto-orient, thumbnail, batch with directory mirroring and
parallel workers, quality, and `--strip`.

**Four things the plan got wrong, all found while building:**

1. **AVIF is write-only.** Encoding AVIF is pure Rust (`ravif`), but decoding it needs
   `avif-native`, which links `dav1d`, a C library. One C dependency defeats the entire
   premise, so AVIF input is refused with an explicit message. This is the same rule that
   already excluded HEIC, applied consistently.
2. **WebP output is lossless only.** `image` 0.25 has no lossy WebP encoder and neither
   does `image-webp`. `--quality` on WebP output is therefore an error, not a number that
   quietly does nothing.
3. **TIFF output is uncompressed**, because the encoder has no compression setting. A
   300x950 image is 835 KB. Documented in the README.
4. **There is no streaming.** `image` decodes whole images, so a 12 MP resize peaks at
   145 MB against ImageMagick's 132 MB. The claim in the original brief that this would
   use less memory is wrong; the decompression-bomb guard is what actually protects the
   machine.

**Performance, honestly:** 10% faster than ImageMagick 7.1.2 on a 12 MP resize (1.08 s
against 1.20 s), 29% faster on a 400 px thumbnail (0.26 s against 0.37 s), 1.6x faster
on JPEG to PNG, and 3.6x on eight files with four workers. Speed is not the pitch and
must not become one.

**What is still unproven:** whether anyone downloads it. The kill test stands: under
about 5,000 crate downloads in the first 90 days, the demand thesis is wrong for a CLI.

## Next step

Write the launch surface (README exists), then decide on publishing: the name is free on
crates.io as of 25 October 2026, and `cargo publish` is the only way to hold it.
