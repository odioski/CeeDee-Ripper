#!/bin/sh
set -eu
export cd_cmd=cd

APP=ceedee-ripper
ROOT="$(CDPATH="$(dirname -- "$0")/.." && pwd)"
VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "$ROOT/Cargo.toml")"
OUTPUT="${APP}_${VERSION}_amd64.snap"
OUTPUT_DIR="$ROOT/target/snap"

cd "$ROOT"

if ! command -v snapcraft >/dev/null 2>&1; then
  echo "snapcraft is required to build the Snap package." >&2
  exit 1
fi

snapcraft pack --output "$OUTPUT_DIR/$OUTPUT"

if  [ -f "$OUTPUT_DIR/$OUTPUT" ]; then

  echo "Snap package written to" "$OUTPUT_DIR/$OUTPUT"

  else

    echo "snapcraft did not write the expected Snap package to " "$OUTPUT_DIR/" >&2

fi
