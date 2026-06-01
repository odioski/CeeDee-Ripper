#!/usr/bin/env bash
set -euo pipefail

have() { command -v "$1" >/dev/null 2>&1; }

missing_debian_packages() {
  local package

  for package in "$@"; do
    if ! dpkg-query -W -f='${db:Status-Abbrev}' "$package" 2>/dev/null | grep -q '^ii '; then
      printf '%s\n' "$package"
    fi
  done
}

missing_pacman_packages() {
  local package

  for package in "$@"; do
    if ! pacman -Qq "$package" >/dev/null 2>&1; then
      printf '%s\n' "$package"
    fi
  done
}

missing_dnf_packages() {
  local package

  for package in "$@"; do
    if ! rpm -q "$package" >/dev/null 2>&1; then
      printf '%s\n' "$package"
    fi
  done
}

if have apt-get; then
  echo "Detected apt (Debian/Ubuntu). Checking packages..."
  debian_packages=(
    build-essential \
    cmake \
    cmake-curses-gui \
    cmake-qt-gui \
    cmake-format \
    cmake-extras \
    extra-cmake-modules \
    ninja-build \
    make \
    ccache \
    clang \
    clang-tools \
    clang-tidy \
    clang-format \
    cppcheck \
    doxygen \
    graphviz \
    cargo \
    rustc \
    rust-src \
    pkg-config \
    debhelper \
    desktop-file-utils \
    appstream \
    flatpak-builder \
    libclang-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libgdk-pixbuf-xlib-2.0-dev \
    libglib2.0-dev \
    libgraphene-1.0-dev \
    libgtk-4-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libadwaita-1-dev \
    libdiscid-dev \
    libgpgme-dev \
    libgcrypt20-dev \
    libcurl4-openssl-dev \
    zsync \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-ugly \
    cdparanoia \
    cd-discid \
    eject \
    flac \
    lame \
    vorbis-tools
  )
  mapfile -t missing_packages < <(missing_debian_packages "${debian_packages[@]}")

  if (( ${#missing_packages[@]} == 0 )); then
    echo "All Debian/Ubuntu packages are already installed."
  else
    echo "Installing missing Debian/Ubuntu packages:"
    printf '  %s\n' "${missing_packages[@]}"
    sudo apt-get update
    sudo apt-get install -y "${missing_packages[@]}"
  fi
  echo "Done."
elif have pacman; then
  echo "Detected pacman (Arch). Checking packages..."
  pacman_packages=(
    base-devel
    cargo
    rust
    rust-src
    pkgconf
    clang
    desktop-file-utils
    appstream
    flatpak-builder
    glib2
    cairo
    pango
    gdk-pixbuf2
    graphene
    gtk4
    gstreamer
    gst-plugins-base
    gst-plugins-good
    gst-plugins-ugly
    libadwaita
    libdiscid
    cdparanoia
    cd-discid
    eject
    flac
    lame
    vorbis-tools
  )
  mapfile -t missing_packages < <(missing_pacman_packages "${pacman_packages[@]}")

  if (( ${#missing_packages[@]} == 0 )); then
    echo "All Arch packages are already installed."
  else
    echo "Installing missing Arch packages:"
    printf '  %s\n' "${missing_packages[@]}"
    sudo pacman -S --needed "${missing_packages[@]}"
  fi
  echo "Done."
elif have zypper; then
  # Note: 'lame' and 'cd-discid' may require the Packman repository on openSUSE:
  #   sudo zypper ar -cfp 90 https://ftp.gwdg.de/pub/linux/misc/packman/suse/openSUSE_Tumbleweed/ packman
  #   sudo zypper dup --from packman --allow-vendor-change
  echo "Detected zypper (openSUSE). Installing packages..."
  sudo zypper install -y \
    gcc \
    make \
    cargo \
    rust \
    pkg-config \
    clang-devel \
    desktop-file-utils \
    AppStream \
    flatpak-builder \
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
  sudo zypper install -y rust-src || \
    echo "rust-src was not available from configured openSUSE repositories; continuing."
  echo "Done."
elif have dnf; then
  echo "Detected dnf (Fedora/RHEL). Checking packages..."
  # Note: 'lame' and gstreamer1-plugins-ugly-free may require RPM Fusion on Fedora:
  #   sudo dnf install https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
  # Fedora provides appstream-util in the libappstream-glib package.
  dnf_packages=(
    gcc
    make
    cargo
    rust
    rust-src
    rpm-build
    pkgconf-pkg-config
    clang-devel
    mksquashfs
    desktop-file-utils
    libappstream-glib
    flatpak-builder
    glib2-devel
    cairo-devel
    pango-devel
    gdk-pixbuf2-devel
    graphene-devel
    gtk4-devel
    gstreamer1-devel
    gstreamer1-plugins-base-devel
    gstreamer1-plugins-base
    gstreamer1-plugins-good
    gstreamer1-plugins-ugly-free
    libadwaita-devel
    libdiscid-devel
    libgcrypt-devel
    libcurl-devel
    libgio-devel
    zsync
    curl
    libcurl
    libgpgme
    libgio
    libglib
    cdparanoia
    cd-discid
    eject
    flac
    lame
    vorbis-tools
  )
  mapfile -t missing_packages < <(missing_dnf_packages "${dnf_packages[@]}")

  if (( ${#missing_packages[@]} == 0 )); then
    echo "All Fedora/RHEL packages are already installed."
  else
    echo "Installing missing Fedora/RHEL packages:"
    printf '  %s\n' "${missing_packages[@]}"
    sudo dnf install -y "${missing_packages[@]}"
  fi
  echo "Done."
else
  echo "Unsupported package manager. Please install dependencies manually." >&2
  exit 1
fi
