#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
BUILD_ROOT="$ROOT/target/ubuntu/source"

cd "$ROOT"

if ! command -v dpkg-buildpackage >/dev/null 2>&1; then
  echo "dpkg-buildpackage is required to build the Ubuntu .deb package." >&2
  exit 1
fi

rm -rf "$BUILD_ROOT"
mkdir -p "$BUILD_ROOT"

tar \
  --exclude='.git' \
  --exclude='target' \
  --exclude='packaging/ubuntu/debian' \
  -cf - \
  . | tar -xf - -C "$BUILD_ROOT"

cp -R packaging/ubuntu/debian "$BUILD_ROOT/debian"

cd "$BUILD_ROOT"
dpkg-buildpackage -us -uc -b

echo "Ubuntu package artifacts written to $ROOT/target/ubuntu"
