#!/bin/bash
# The promise, enforced: for a fixed set of inputs and flags, the output bytes are
# fixed too. A hash that moves is a change to the contract, not a detail.
#
#   scripts/golden.sh            check against the manifest
#   scripts/golden.sh --update   rewrite the manifest (then bump the version and
#                                say why in the commit, because someone's asset
#                                hashes just changed)
#
# Runs against the release binary, which is what people install, and on every
# operating system in CI: that is how a cross-platform promise gets tested rather
# than assumed.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

BIN=target/release/imgsmith
MANIFEST=tests/golden/manifest.txt
WORK=$(mktemp -d "${TMPDIR:-/tmp}/imgsmith-golden.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

cargo build --release --quiet || { echo "build failed" >&2; exit 1; }

# Each case is: name, then the arguments with %OUT% standing for the output file.
run_case() {
  local name=$1; shift
  local out="$WORK/$name"
  local args=()
  for a in "$@"; do
    case $a in
      *%OUT%*) args+=("${a//%OUT%/$out}") ;;
      *) args+=("$a") ;;
    esac
  done
  "$BIN" "${args[@]}" >"$WORK/$name.stdout" 2>"$WORK/$name.stderr"
  local status=$?
  if [ $status -ne 0 ]; then
    echo "FAIL $name: exit $status" >&2
    sed 's/^/    /' "$WORK/$name.stderr" >&2
    return 1
  fi
  # The case names its own extension, so find whatever it actually wrote.
  local written
  written=$(find "$WORK" -maxdepth 1 -name "$name.*" ! -name '*.stdout' ! -name '*.stderr' | head -1)
  if [ -z "$written" ]; then
    echo "FAIL $name: nothing was written" >&2
    return 1
  fi
  local sum size
  sum=$(sha256sum "$written" | cut -d' ' -f1)
  size=$(stat -c %s "$written")
  printf '%s %s %s\n' "$name" "$sum" "$size"
}

cases() {
  local F=tests/fixtures
  # Format conversion
  run_case jpeg_to_png "$F/plain.jpg" --format png -o "%OUT%.png"
  run_case png_resize "$F/plain.jpg" --format png --width 16 -o "%OUT%.png"
  # The operations the tool exists for
  run_case jpeg_resize "$F/tagged.jpg" --width 32 -o "%OUT%.jpg"
  run_case jpeg_thumbnail "$F/tagged.jpg" --thumb 24 -o "%OUT%.jpg"
  run_case jpeg_crop "$F/tagged.jpg" --crop 32x24+8+8 -o "%OUT%.jpg"
  run_case jpeg_rotate "$F/tagged.jpg" --rotate 90 -o "%OUT%.jpg"
  run_case jpeg_flip "$F/tagged.jpg" --flip h -o "%OUT%.jpg"
  run_case jpeg_strip "$F/tagged.jpg" --strip -o "%OUT%.jpg"
  run_case jpeg_auto_orient "$F/tagged.jpg" --auto-orient -o "%OUT%.jpg"
  # Encoders
  run_case webp_lossless "$F/plain.jpg" --format webp -o "%OUT%.webp"
  run_case jpeg_quality "$F/plain.jpg" --quality 60 -o "%OUT%.jpg"
}

if [ "${1:-}" = "--update" ]; then
  mkdir -p "$(dirname "$MANIFEST")"
  {
    echo "# Output bytes promised by this version. Regenerate with scripts/golden.sh --update."
    echo "# Format: case sha256 size"
    cases
  } > "$MANIFEST"
  echo "manifest written: $(grep -c -v '^#' "$MANIFEST") cases"
  exit 0
fi

[ -f "$MANIFEST" ] || { echo "no manifest at $MANIFEST" >&2; exit 1; }

actual=$(cases) || { echo "a case failed to run" >&2; exit 1; }
expected=$(grep -v '^#' "$MANIFEST")

if [ "$actual" = "$expected" ]; then
  echo "golden: $(echo "$actual" | wc -l) cases match byte for byte"
  exit 0
fi

echo "the output bytes changed:" >&2
diff <(echo "$expected") <(echo "$actual") | sed 's/^/  /' >&2
cat >&2 <<'EOF'

If this was intended, run scripts/golden.sh --update, bump the major version and
say in the commit which outputs moved and why. If it was not intended, this is a
bug in an encoder and the diff above is the evidence.
EOF
exit 1
