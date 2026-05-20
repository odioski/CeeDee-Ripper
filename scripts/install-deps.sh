#!/usr/bin/env bash
set -euo pipefail

have() { command -v "$1" >/dev/null 2>&1; }

if have apt-get; then
  echo "Detected apt (Debian/Ubuntu). Installing packages..."
  sudo apt-get update
  sudo apt-get install -y \
    apt-file \
    build-essential \
    pkg-config \
    libgio-2.0-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libgdk-pixbuf2.0-dev \
    libgraphene-1.0-dev \
    libgtk-4-bin \
    libgtk-4-common \
    libgtk-4-dev \
    libgstreamer1.0-dev \
    libadwaita-1-dev \
    libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-ugly \
    libdiscid-dev \
    cdparanoia \
    cd-discid \
    eject \
    flac \
    lame \
    vorbis-tools
  echo "Done."
elif have pacman; then
  echo "Detected pacman (Arch). Installing packages..."
  sudo pacman -S --needed \
    base-devel \
    pkgconf \
    glib2 \
    cairo \
    pango \
    gdk-pixbuf2 \
    graphene \
    gtk4 \
    gstreamer \
    gst-plugins-base \
    gst-plugins-good \
    gst-plugins-ugly \
    libadwaita \
    libdiscid \
    cdparanoia \
    cd-discid \
    eject \
    flac \
    lame \
    vorbis-tools
  echo "Done."
elif have zypper; then
  # Note: 'lame' and 'cd-discid' may require the Packman repository on openSUSE:
  #   sudo zypper ar -cfp 90 https://ftp.gwdg.de/pub/linux/misc/packman/suse/openSUSE_Tumbleweed/ packman
  #   sudo zypper dup --from packman --allow-vendor-change
  echo "Detected zypper (openSUSE). Installing packages..."
  sudo zypper install -y \
    gcc \
    make \
    pkg-config \
    glib2-devel \
    cairo-devel \
    pango-devel \
    gdk-pixbuf-devel \
    libgraphene-devel \
    gtk4-devel \
    gstreamer-devel \
    gstreamer-plugins-base-devel \
    gstreamer-plugins-good \
    gstreamer-plugins-ugly \
    libadwaita-devel \
    libdiscid-devel \
    cdparanoia \
    cd-discid \
    eject \
    flac \
    lame \
    vorbis-tools
  echo "Done."
elif have dnf; then
  echo "Detected dnf (Fedora/RHEL). Installing packages..."
  # Note: 'lame' may require RPM Fusion (free) on Fedora:
  #   sudo dnf install https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
  sudo dnf install -y \
    gcc \
    make \
    pkgconf-pkg-config \
    glib2-devel \
    cairo-devel \
    pango-devel \
    gdk-pixbuf2-devel \
    graphene-devel \
    gtk4-devel \
    gstreamer1-devel \
    gstreamer1-plugins-base-devel \
    gstreamer1-plugins-good \
    gstreamer1-plugins-ugly \
    libadwaita-devel \
    libdiscid-devel \
    cdparanoia \
    cd-discid \
    eject \
    flac \
    lame \
    vorbis-tools
  echo "Done."
else
  echo "Unsupported package manager. Please install dependencies manually." >&2
  echo "See the 'list' file in the project root for the required apt packages." >&2
  exit 1
fi
