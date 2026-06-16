#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
BUILD_ROOT="$ROOT/target/ubuntu/source"

cd "$ROOT"

if ! command -v dpkg-buildpackage >/dev/null 2>&1; then
  echo "dpkg-buildpackage is required to build the Debian/Ubuntu .deb package." >&2
  exit 1
fi

rm -rf "$BUILD_ROOT"
mkdir -p "$BUILD_ROOT"

tar \
  --exclude='.git' \
  --exclude='target' \
  -cf - \
  . | tar -xf - -C "$BUILD_ROOT"

cp -R packaging/ubuntu/debian "$BUILD_ROOT/debian"

cd "$BUILD_ROOT"
dpkg-buildpackage -us -uc -b

echo "Debian/Ubuntu package artifacts written to $ROOT/target/ubuntu"
echo ""
echo "Upload/contrib:"
echo "  Test the .deb on a clean matching Debian or Ubuntu GNOME desktop before publishing."
echo "  Do not upload local .deb artifacts to a PPA; PPAs build from signed source uploads."
echo "  See Distribution Instructions.md for GitHub Release and Launchpad/PPA steps."
