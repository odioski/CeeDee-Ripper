#!/usr/bin/env bash
set -euo pipefail

PKGS=(cdparanoia cd-discid flac lame vorbis-tools eject)
# Build deps for GUI and disc detection via libdiscid
DEV_PKGS_DEB=(libgtk-4-dev libadwaita-1-dev libdiscid-dev)
DEV_PKGS_ARCH=(gtk4 libadwaita libdiscid)
DEV_PKGS_DNF=(gtk4-devel libadwaita-devel libdiscid-devel)

# GStreamer plugins needed for cdparanoia element and WAV encoding
GST_PKGS_DEB=(gstreamer1.0-plugins-base gstreamer1.0-plugins-good)
GST_PKGS_ARCH=(gst-plugins-base gst-plugins-good)
GST_PKGS_DNF=(gstreamer1-plugins-base gstreamer1-plugins-good)

have() { command -v "$1" >/dev/null 2>&1; }

if have apt-get; then
  echo "Detected apt (Debian/Ubuntu). Installing packages..."
  sudo apt-get update
  sudo apt-get install -y "${PKGS[@]}" "${DEV_PKGS_DEB[@]}" "${GST_PKGS_DEB[@]}"
  echo "Done."
elif have pacman; then
  echo "Detected pacman (Arch). Installing packages..."
  sudo pacman -S --needed "${PKGS[@]}" "${DEV_PKGS_ARCH[@]}" "${GST_PKGS_ARCH[@]}"
  echo "Done."
elif have dnf; then
  echo "Detected dnf (Fedora/RHEL). Installing packages..."
  # Note: 'lame' may require RPM Fusion on Fedora
  if ! sudo dnf install -y "${PKGS[@]}" "${DEV_PKGS_DNF[@]}" "${GST_PKGS_DNF[@]}"; then
    cat << 'EOF'
Some packages could not be installed. If 'lame' is missing on Fedora,
you may need to enable RPM Fusion (free):
  sudo dnf install https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
Then re-run this script.
EOF
  fi
  echo "Done (with notes)."
else
  cat << 'EOF'
Unsupported package manager.
Please install these packages manually:
  cdparanoia cd-discid flac lame
And build dependencies (GTK4/Libadwaita dev libs) for your distro.
EOF
  exit 1
fi
