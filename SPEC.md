# The complete imgsmith, and what "forever" costs

A specification, not a plan of record. Every number labelled "measured" comes
from a command run against this machine; every number labelled "estimate" is a
guess with its reasoning shown.

## Who it is for

People who write scripts that touch images: a build pipeline making web assets, a
sysadmin converting a folder, a CI job resizing screenshots. Their work is a shell
line inside something else, which decides what "complete" means here:

- **Complete means every operation that appears in a script.** Not every
  operation ImageMagick has. Text rendering, filters, distortion maps and
  compositing are for people drawing, not for people scripting, and they stay out.
- **Complete means the output is predictable.** Same input and same flags give
  byte-identical output, on every platform, forever. Today that holds and is not
  promised; it should be.
- **Complete means it never surprises.** No silent flattening, no quiet quality
  drops, no reading a config file the caller did not write.

## The operation set, as it would stand at 1.0

| Operation | Today | 1.0 |
|---|---|---|
| Convert between formats | yes | yes |
| Resize, fit inside/outside/fill/cover | yes | yes |
| Crop with gravity | yes | yes |
| Rotate and mirror | yes | yes |
| EXIF auto-orient | yes | yes |
| Thumbnail | yes | yes |
| Batch a directory, parallel | yes | yes |
| Quality control | JPEG/AVIF only, 4:4:4 forced | **subsampling and optimised Huffman tables** |
| Strip metadata | all or nothing | all, or **one named tag group** (e.g. GPS only) |
| Read metadata | yes | yes |
| Write metadata | no | **yes: set, replace, delete tags** |
| Optimise size at equal quality | no | **yes: PNG re-deflate, JPEG tables, WebP lossless** |
| Deterministic output | not promised | **promised and tested** |
| Colour: apply an ICC profile | no, only carried | **convert to sRGB** or refuse |
| HEIC input | refused | refused (needs a C decoder) |
| Lossy WebP output | refused | refused (no pure-Rust VP8 encoder exists) |

That is the whole of it. Fifteen rows, no text, no filters, no compositing.

## The dependency ladder

The question is not "how do we remove dependencies" but "which dependencies can we
verify". A container we can check byte by byte; an encoder for a video codec we
cannot.

**Ours already, and worth keeping ours** (measured: about 2,900 lines of our own):

- The command line, job planning, the walk, the thread pool.
- Geometry: fit modes, crop specification and gravity, rotation arithmetic.
- EXIF reading, and the JPEG segment walk that carries metadata forward.
- The pixel-space guard that refuses decompression bombs.

**Ours next, because it is a container and a container is verifiable:**

| Piece | Lines (estimate) | How it is checked |
|---|---|---|
| EXIF write: set, replace, delete a tag | 600 | our own reader, plus Pillow, plus a byte diff on the segment |
| PNG container and filter choice | 500 | re-read with ImageMagick and compare pixels |
| TIFF container with deflate | 700 | same, plus a round trip through our own reader |
| JPEG encoder: subsampling, optimised tables | 2,500 | PSNR against ImageMagick at the same quality number |
| ICC to sRGB conversion | 1,500 | the ICC's own test charts, and a delta-E budget |

Those four bring the tool to the point where the only things it cannot do are the
ones that need a video codec.

**Depend on, forever, and say so plainly:**

| Dependency | Why it stays | Pure Rust |
|---|---|---|
| `miniz_oxide` | DEFLATE. A bug here corrupts every PNG. Owning it is 2,000 lines of bit-level code with no upside | yes |
| `zune-jpeg` or `image`'s decoder | JPEG decoding: progressive, restart markers, arithmetic coding | yes |
| `image-webp` | WebP decode and lossless encode | yes |
| `ravif`/`rav1e` | AVIF encoding: an AV1 encoder is decades of work | yes |
| `kamadak-exif` | only until our own reader replaces it | yes |

**Never ours:** a VP8 encoder (lossy WebP) or an AV1 encoder from scratch. Both are
research projects, not features, and the honest answer to "why can't it write lossy
WebP" is one sentence in the README.

## What "forever" actually requires

Completeness is the easy half. The half that makes an app outlive its author:

1. **A frozen surface at 1.0.** Every flag in the README keeps working. A flag can
   be deprecated with a warning for five years before it is removed. Adding a flag
   is a minor version; changing what one means is a major one.
2. **No silent dependency drift.** Pin exact versions in `Cargo.lock`, commit the
   lock file, and vendor the dependencies that matter (measured: 138 crates, about
   40 MB of source) into `vendor/` so a yanked crate cannot break a build. Build
   with `--offline` in CI, once a week, forever.
3. **Reproducible binaries.** Same source, same toolchain, same bytes. Rust makes
   this reachable; it has to be tested, not assumed.

**Enforced since 0.1.4:** `scripts/golden.sh` runs eleven conversions against the
release binary and compares sha256 with `tests/golden/manifest.txt`. CI runs it on
Linux x86_64, macOS arm64 and Windows x86_64, so a hash that moves on one
architecture fails the build. Measured: all eleven agree on all three today,
including resize, crop, rotate, thumbnail and every encoder we write.

What can still break the promise, and what to do about it:

| Risk | Why it bites | Rule |
|---|---|---|
| A dependency bump changes DEFLATE output | PNG bytes change without a line of our code changing | the lock file is part of the contract; bumping `miniz_oxide` is a checked, deliberate act |
| Float maths differs by CPU | SIMD and FMA round differently on aarch64 | the golden run on three architectures is the alarm; if it fires, that path becomes integer-only |
| An encoder gains a fast path | codegen change moves the bytes | same alarm, same rule |
4. **No state.** No config file, no cache directory, no environment variable that
   changes behaviour, no telemetry, no network. A tool whose behaviour depends on
   a hidden file cannot be trusted in a script.
5. **Output stability as a promise.** The encoder settings that produce today's
   bytes are part of the contract; changing them is a major version, because
   someone's asset hashes will change.
6. **The refusal list is part of the product.** Each refusal has a reason and a
   suggested alternative, and it does not shrink quietly.

## What it costs, honestly

- **Size:** owning the containers adds roughly 300 KB to 500 KB of code
  (estimate; the EXIF writer measured 512 KB of *dependency*, ours would be
  smaller). Still under 8 MB.
- **Dependencies:** from 129 crates today to about 45 (estimate: the AVIF subtree
  is 40 crates and 571,000 lines on its own; the JPEG, PNG and TIFF paths are
  small).
- **Time:** the four owned pieces are about a month of evenings, and the JPEG
  encoder is two thirds of it.
- **Risk:** every line we own is a line that can corrupt a photo. That risk is
  acceptable only because each piece is checked against a second implementation
  on every commit, which is the same standard the current code is held to.

## The shortest honest summary

The perfect version of this tool for its demographic is **small, boring,
predictable and complete in the sense a script cares about**: fifteen operations,
no config, no network, byte-identical output, a frozen command line, and an
explicit list of what it will never do with the reason beside it. It owns every
container it touches and depends only on the entropy coders and video codecs it
cannot reasonably replace.
