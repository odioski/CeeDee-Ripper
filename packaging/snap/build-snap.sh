#!/bin/sh
set -eu

APP=ceedee-ripper
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "$ROOT/Cargo.toml")"
OUTPUT="$ROOT/target/snap/${APP}_${VERSION}_amd64.snap"
OUTPUT_DIR="$ROOT/target/snap"

cd "$ROOT"

if ! command -v snapcraft >/dev/null 2>&1; then
  echo "snapcraft is required to build the Snap package." >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

(
  cd "$ROOT/packaging/snap"
  snapcraft pack --output "$OUTPUT_DIR"
)

GENERATED="$(find "$OUTPUT_DIR" -maxdepth 1 -type f -name "${APP}_${VERSION}_*.snap" -print | head -n 1)"
if [ -z "$GENERATED" ]; then
  echo "snapcraft did not write the expected Snap package to $OUTPUT_DIR." >&2
  exit 1
fi

if [ "$GENERATED" != "$OUTPUT" ]; then
  mv "$GENERATED" "$OUTPUT"
fi

echo "Snap package written to $OUTPUT"
