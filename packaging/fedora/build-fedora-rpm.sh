#!/bin/sh
set -eu

APP=ceedee-ripper
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "$ROOT/Cargo.toml")"
RPM_ROOT="$ROOT/target/fedora/rpmbuild"
SPEC="$ROOT/packaging/fedora/${APP}.spec"
SOURCE="$RPM_ROOT/SOURCES/${APP}-${VERSION}.tar.gz"

cd "$ROOT"

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "rpmbuild is required to build the Fedora RPM." >&2
  exit 1
fi

mkdir -p \
  "$RPM_ROOT/BUILD" \
  "$RPM_ROOT/BUILDROOT" \
  "$RPM_ROOT/RPMS" \
  "$RPM_ROOT/SOURCES" \
  "$RPM_ROOT/SPECS" \
  "$RPM_ROOT/SRPMS"

tar \
  --exclude='.git' \
  --exclude='target' \
  --transform="s|^|CeeDee-Ripper-${VERSION}/|" \
  -czf "$SOURCE" \
  .

rpmbuild \
  --define "_topdir $RPM_ROOT" \
  -ba "$SPEC"

echo "RPM artifacts written under $RPM_ROOT/RPMS and $RPM_ROOT/SRPMS"
