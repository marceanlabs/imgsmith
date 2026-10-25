#!/usr/bin/env python3
"""Check orientation handling against real camera files and an independent decoder.

Downloads 21 public sample photos (ianare/exif-samples) covering all eight EXIF
orientations plus several camera vendors, then:

  1. runs `imgsmith --auto-orient` and `magick -auto-orient` on the same file
  2. requires identical output dimensions
  3. requires an 8x8 colour grid to match within a small tolerance, which fails
     loudly if the image was turned the wrong way rather than merely resized

Usage:  python3 scripts/verify_real_world.py [path-to-imgsmith]
Requires: ImageMagick (`magick`) and network access for the first run.
"""
import os
import re
import subprocess
import sys
import tempfile
import urllib.request

SMI = sys.argv[1] if len(sys.argv) > 1 else "target/release/imgsmith"
BASE = "https://raw.githubusercontent.com/ianare/exif-samples/master/jpg"
FILES = (
    [f"orientation/landscape_{i}.jpg" for i in range(1, 9)]
    + [f"orientation/portrait_{i}.jpg" for i in range(1, 9)]
    + [
        "Canon_40D.jpg",
        "Nikon_D70.jpg",
        "Panasonic_DMC-FZ30.jpg",
        "mobile/HMD_Nokia_8.3_5G.jpg",
        "Canon_PowerShot_S40.jpg",
    ]
)
TOLERANCE = 12


def sh(*args):
    return subprocess.run(args, capture_output=True, text=True)


def fetch(work, name):
    path = os.path.join(work, name.replace("/", "_"))
    if not os.path.exists(path):
        req = urllib.request.Request(f"{BASE}/{name}", headers={"User-Agent": "imgsmith"})
        with urllib.request.urlopen(req, timeout=60) as r, open(path, "wb") as f:
            f.write(r.read())
    return path


def dims(path):
    return sh("magick", "identify", "-format", "%wx%h", path).stdout.strip()


def grid(path):
    out = sh("magick", path, "-resize", "8x8!", "-depth", "8", "txt:-")
    values = []
    for line in out.stdout.splitlines()[1:]:
        m = re.search(r"\((\d+),(\d+),(\d+)", line)
        if m:
            values.append(tuple(int(v) for v in m.groups()))
    return values


def main():
    failures = 0
    with tempfile.TemporaryDirectory() as work:
        print(f"{'file':38} {'auto-oriented':>20}  check")
        for name in FILES:
            src = fetch(work, name)
            ours = os.path.join(work, "ours.png")
            ref = os.path.join(work, "ref.png")
            run = sh(SMI, src, "-o", ours, "--auto-orient", "--strip", "--force", "-q")
            if run.returncode != 0:
                print(f"{name:38} {'-':>20}  FAILED: {run.stderr.strip()[:60]}")
                failures += 1
                continue
            sh("magick", src, "-auto-orient", ref)
            a, b = grid(ours), grid(ref)
            diff = 999 if len(a) != 64 or len(b) != 64 else max(
                abs(x[i] - y[i]) for x, y in zip(a, b) for i in range(3)
            )
            same = dims(ours) == dims(ref) and diff <= TOLERANCE
            if not same:
                failures += 1
            print(
                f"{name:38} {dims(ours) + ' vs ' + dims(ref):>20}  "
                f"{'ok' if same else 'MISMATCH'} (max channel diff {diff})"
            )
    total = len(FILES)
    print(f"\n{total - failures} of {total} real camera files match ImageMagick")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
