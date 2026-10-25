#!/bin/bash
# Every writable format must be readable again, except AVIF which is write-only.
# Each output is checked by ImageMagick, a different implementation from the
# library this tool is built on.
set -u
SMI="${1:-target/release/imgsmith}"
# The script changes directory, so the binary path has to be absolute first.
case "$SMI" in
  /*) ;;
  *) SMI="$(cd "$(dirname "$SMI")" && pwd)/$(basename "$SMI")" ;;
esac
B=$(mktemp -d)
trap 'rm -rf "$B"' EXIT
cd "$B" || exit 1

magick -size 256x256 plasma:fractal src.png || exit 1
fail=0

for ext in png jpg webp tiff gif bmp tga pnm qoi ff hdr exr avif; do
  out="out.${ext}"
  if ! "$SMI" src.png -o "$out" --force -q 2>err1; then
    printf '%-5s write FAILED: %s\n' "$ext" "$(head -c 90 err1)"
    fail=1
    continue
  fi
  fmt=$(magick identify -format '%m %wx%h' "$out" 2>/dev/null)
  case "$fmt" in
    *256x256) write="ok" ;;
    *) write="UNREADABLE ($fmt)"; fail=1 ;;
  esac

  if [ "$ext" = "avif" ]; then
    read="skipped (write only)"
  elif "$SMI" "$out" -o "back_${ext}.png" --force -q 2>err2; then
    read="ok"
  else
    read="FAILED: $(head -c 90 err2)"
    fail=1
  fi
  printf '%-5s write=%-4s read=%s\n' "$ext" "$write" "$read"
done

# Real-world inputs that are not a plain baseline JPEG or 8-bit PNG.
magick -size 200x150 plasma:fractal base.png
magick base.png -colorspace CMYK cmyk.jpg
magick base.png -interlace Plane progressive.jpg
magick base.png -depth 16 sixteen.png
magick base.png -colorspace Gray gray.jpg
magick base.png -alpha set -channel A -evaluate set 40% +channel alpha.png
printf 'not an image' > corrupted.jpg

for f in cmyk.jpg progressive.jpg sixteen.png gray.jpg alpha.png; do
  if "$SMI" "$f" -o "edge_$f.png" --force -q 2>err3; then
    printf '%-16s ok\n' "$f"
  else
    printf '%-16s FAILED: %s\n' "$f" "$(head -c 90 err3)"
    fail=1
  fi
done

if "$SMI" corrupted.jpg -o nope.png --force -q 2>err4; then
  echo "corrupted.jpg was accepted, which is a bug"
  fail=1
else
  printf '%-16s refused with a clean error\n' corrupted.jpg
fi

exit "$fail"
