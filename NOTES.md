# imgsmith — notes

Working directory for the ImageMagick-class Rust CLI. Shipped as `imgsmith` 0.1.0.

- `SCOPE.md` — the scope decision, with every claim graded and the kill tests, plus a
  "what shipped and where the plan was wrong" section at the end.
- `README.md` — the public document. Leads with the refusals.
- `src/` — `ops.rs` (geometry, unit tested), `img.rs` (decode, transform, encode),
  `meta.rs` (EXIF orientation, JPEG segments, animation detection), `error.rs`.
- `tests/cli.rs` — 22 end-to-end tests that run the real binary.

## Status, 25 October 2026

Published: crates.io `imgsmith` 0.1.1, repo github.com/marceanlabs/imgsmith (Marcean Labs). The live plan is
the refined scope at the top of `SCOPE.md`: distribution, not features.
`cargo install imgsmith` was run from the registry and converted a file, so the
published crate is verified end to end, not just built locally.

## Decisions taken during the build

- Name `imgsmith`: was free on crates.io on 25 Oct 2026, now taken by us.
- AVIF is **output only**. Decoding needs `dav1d`, a C library. Same rule that excluded
  HEIC, applied consistently.
- WebP is **lossless only**, because no pure-Rust lossy encoder exists. `--quality` with
  WebP is an error rather than a silent no-op.
- Metadata: carried for JPEG to JPEG only. `--auto-orient` drops the EXIF block, because
  the orientation tag would be wrong afterwards. ICC survives.
- Alpha in formats without an alpha channel is flattened onto **white**, not black.
- Order is fixed: orient, crop, resize, rotate/mirror.
- `--max-mp` defaults to 100 megapixels as a decompression-bomb guard.

## Do not re-litigate

- PDF is out. A PDF tool is the PDF answer, and Ghostscript is why ImageMagick's attack
  surface is what it is.
- SVG is out. resvg won it.
- Delegates and a plugin architecture are out. That design is the defect being replaced.
- Speed is not the pitch and must not become one. The margins are 10% to 30%.

## Verification, rerun any time

```console
$ cargo test          # 44 tests: 12 unit, 8 API, 22 end to end, 2 doctests
$ cargo clippy --all-targets -- -D warnings
$ cargo build --release
$ bash scripts/verify_formats.sh ./target/release/imgsmith
$ python3 scripts/verify_real_world.py ./target/release/imgsmith
$ bash scripts/measure_downloads.sh
```

The Homebrew tap was installed for real on 25 Oct 2026, as a non-root user on this
machine: `brew install marceanlabs/imgsmith/imgsmith`, then `brew test imgsmith` and a
conversion through the installed binary. That is the whole path a Mac or Linux user
takes.

Benchmark scripts live in `/root/.hermes/cache/scratch/bench/` (`bench.sh`, `batch.sh`,
`real.sh`) and compare against ImageMagick 7.1.2 installed on this machine.

## Identity

The public account was renamed from `wlkr73` to `lumenyr` on 25 Oct 2026. Commits after
that date are authored `lumenyr <86650480+lumenyr@users.noreply.github.com>`; earlier
commits keep the old name, because rewriting published history would break every clone
and every link for no gain. Both crates still list `wlkr73` as the crates.io owner until
that username is changed at crates.io, which is a separate namespace from GitHub.
