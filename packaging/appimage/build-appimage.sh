#!/bin/sh
set -eu

APP=ceedee-ripper
VERSION=1.1.0
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
APPDIR="$ROOT/target/appimage/${APP}.AppDir"

cd "$ROOT"
cargo build --release --locked

rm -rf "$APPDIR"
install -Dm755 target/release/ceedee-ripper "$APPDIR/usr/bin/ceedee-ripper"
install -Dm755 packaging/appimage/AppRun "$APPDIR/AppRun"
install -Dm644 resources/ceedee-ripper.desktop "$APPDIR/${APP}.desktop"
install -Dm644 resources/ceedee-ripper.desktop "$APPDIR/usr/share/applications/${APP}.desktop"
install -Dm644 resources/images/ceedee-ripper.png "$APPDIR/io.github.odioski.ceedee_ripper.png"
install -Dm644 resources/images/ceedee-ripper.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/io.github.odioski.ceedee_ripper.png"
install -Dm644 resources/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml "$APPDIR/usr/share/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml"
install -Dm644 LICENSE "$APPDIR/usr/share/licenses/${APP}/LICENSE"

if command -v appimagetool >/dev/null 2>&1; then
  ARCH=x86_64 VERSION="$VERSION" appimagetool "$APPDIR" "$ROOT/target/appimage/${APP}-${VERSION}-x86_64.AppImage"
else
  echo "AppDir prepared at $APPDIR"
  echo "Install appimagetool to produce target/appimage/${APP}-${VERSION}-x86_64.AppImage"
fi
