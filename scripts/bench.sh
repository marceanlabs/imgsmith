#!/bin/bash
# Regenerates the README benchmark table. Every number in the README comes from
# here, so a claim that cannot be reproduced by running this file does not belong
# in the README.
#
#   scripts/bench.sh [source.jpg]     default: a generated 4032x3024 JPEG
#
# Needs ImageMagick 7 (`magick`) for the comparison column. imgsmith itself is
# taken from target/release.
set -u
cd "$(dirname "$0")/.." || exit 1
SMI=${SMI:-target/release/imgsmith}
[ -x "$SMI" ] || { echo "build it first: cargo build --release" >&2; exit 1; }
command -v magick >/dev/null || { echo "needs ImageMagick 7 for the comparison column" >&2; exit 1; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

SRC=${1:-}
if [ -z "$SRC" ]; then
  SRC=$TMP/src.jpg
  # -seed makes the source byte identical on every run, so a number measured
  # today can be compared with one measured next month.
  magick -seed 7 -size 4032x3024 plasma:fractal -quality 88 "$SRC"
fi
echo "source: $(basename "$SRC"), $(identify -format '%wx%h' "$SRC"), $(du -h "$SRC" | cut -f1)"
echo "host:   $(nproc) cores, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')"
echo "date:   $(date -u +%Y-%m-%d)"
echo

median3 () {
  local a b c
  a=$(timed "$@"); b=$(timed "$@"); c=$(timed "$@")
  printf '%s\n%s\n%s\n' "$a" "$b" "$c" | sort -n | sed -n '2p'
}

# wall clock in milliseconds, no subshell surprises
timed () {
  local start end
  start=$(date +%s%N)
  "$@" >/dev/null 2>&1
  end=$(date +%s%N)
  echo $(( (end - start) / 1000000 ))
}

t () { printf '%d.%02d s' $(( $1 / 1000 )) $(( ($1 % 1000) / 10 )); }

# The ImageMagick side of the footprint row: the whole dependency closure, as apt
# would install it, not just the package itself.
mapfile -t deps < <(apt-cache depends --recurse --no-recommends --no-suggests \
  --no-conflicts --no-breaks --no-replaces --no-enhances imagemagick 2>/dev/null |
  grep -E '^[a-z0-9]' | sort -u)
pkgs=${#deps[@]}
pkgsize=$(dpkg-query -W -f='${Installed-Size}\n' "${deps[@]}" 2>/dev/null | awk '{s+=$1} END {printf "%d", s/1024}')

echo "| Task | imgsmith | ImageMagick |"
echo "|---|---|---|"

r_smi=$(median3 "$SMI" "$SRC" -o "$TMP/r1.jpg" --width 1920 --quality 85 --force -q)
r_mag=$(median3 magick "$SRC" -resize 1920x -quality 85 "$TMP/r2.jpg")
printf '| Resize to 1920 wide, quality 85 | %s, %s | %s, %s |\n' \
  "$(t "$r_smi")" "$(du -h "$TMP/r1.jpg" | cut -f1)" "$(t "$r_mag")" "$(du -h "$TMP/r2.jpg" | cut -f1)"

th_smi=$(median3 "$SMI" "$SRC" -o "$TMP/t1.jpg" --thumb 400 --quality 85 --force -q)
th_mag=$(median3 magick "$SRC" -thumbnail 400x400 -quality 85 "$TMP/t2.jpg")
printf '| Thumbnail 400 | %s | %s |\n' "$(t "$th_smi")" "$(t "$th_mag")"

p_smi=$(median3 "$SMI" "$SRC" -o "$TMP/c1.png" --force -q)
p_mag=$(median3 magick "$SRC" "$TMP/c2.png")
printf '| JPEG to PNG, 12 MP | %s, %s | %s, %s |\n' \
  "$(t "$p_smi")" "$(du -h "$TMP/c1.png" | cut -f1)" "$(t "$p_mag")" "$(du -h "$TMP/c2.png" | cut -f1)"

# batch: eight copies, four workers against one
mkdir -p "$TMP/in" "$TMP/o4" "$TMP/o1"
for i in $(seq 1 8); do cp "$SRC" "$TMP/in/f$i.jpg"; done
b4=$(median3 "$SMI" "$TMP/in" -o "$TMP/o4" --width 1600 --force -q --jobs 4)
b1=$(median3 "$SMI" "$TMP/in" -o "$TMP/o1" --width 1600 --force -q --jobs 1)
printf '| Eight files to 1600 wide | %s with 4 workers, %s with 1 | not measured |\n' "$(t "$b4")" "$(t "$b1")"

s_smi=$(median3 "$SMI" --version)
s_mag=$(median3 magick --version)
printf '| Program start | %d ms | %d ms |\n' "$s_smi" "$s_mag"

m_smi=$(/usr/bin/time -v "$SMI" "$SRC" -o "$TMP/mem.jpg" --width 1920 --force -q 2>&1 | awk -F': ' '/Maximum resident/ {print int($2/1024)}')
m_mag=$(/usr/bin/time -v magick "$SRC" -resize 1920x "$TMP/mem2.jpg" 2>&1 | awk -F': ' '/Maximum resident/ {print int($2/1024)}')
printf '| Peak memory, 12 MP resize | %s MB | %s MB |\n' "$m_smi" "$m_mag"

printf '| Footprint | %s binary, links libc, libm and libgcc | see apt on a clean system: %s packages, %s MB here |\n' \
  "$(du -h "$SMI" | cut -f1)" "$pkgs" "$pkgsize"
