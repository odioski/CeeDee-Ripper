#!/usr/bin/env bash
set -euo pipefail

# Use os-release rather than whichever foreign package manager happens to be installed.
# Slax ships both Debian- and Slackware-based editions.
detect_family() {
  local ID='' ID_LIKE=''
  local release_file=${1:-/etc/os-release}
  if [[ -r $release_file ]]; then
    # shellcheck disable=SC1090
    . "$release_file"
  fi
  case " $ID $ID_LIKE " in
    *' fedora '*|*' rhel '*) printf 'fedora\n' ;;
    *' debian '*|*' ubuntu '*|*' kali '*) printf 'debian\n' ;;
    *' slackware '*) printf 'slackware\n' ;;
    *' slax '*)
      if have slackpkg; then printf 'slackware\n'
      elif have apt-get; then printf 'debian\n'
      else return 1; fi ;;
    *' arch '*) printf 'arch\n' ;;
    *' opensuse '*|*' opensuse-tumbleweed '*|*' opensuse-leap '*|*' suse '*) printf 'suse\n' ;;
    *) return 1 ;;
  esac
}

as_root() {
  if (( EUID == 0 )); then
    "$@"
  elif have sudo; then
    sudo "$@"
  else
    echo "Run this script as root or install sudo." >&2
    return 1
  fi
}

have() { command -v "$1" >/dev/null 2>&1; }

dnf_command() {
  if have dnf; then
    printf 'dnf\n'
  elif have dnf5; then
    printf 'dnf5\n'
  fi
}

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
    if ! rpm -q "$package" >/dev/null 2>&1 && ! rpm -q --whatprovides "$package" >/dev/null 2>&1; then
      printf '%s\n' "$package"
    fi
  done
}

main() {
  local family dnf_cmd
  local -a debian_packages pacman_packages dnf_deps dnf_libs dnf_packages missing_packages
  family=$(detect_family) || {
    echo "Unsupported distribution. Please install dependencies manually." >&2
    return 1
  }
  if [[ $family == debian ]]; then
    echo "Detected apt (Debian/Ubuntu/Kali). Checking packages..."
    # -dev packages pull in release-appropriate runtime packages (including t64).
    debian_packages=(
      build-essential
      cmake
      cmake-curses-gui
      cmake-qt-gui
      cmake-format
      cmake-extras
      extra-cmake-modules
      ninja-build
      make
      ccache
      clang
      clang-tools
      clang-tidy
      clang-format
      cppcheck
      doxygen
      graphviz
      cargo
      rustc
      rust-src
      pkg-config
      dpkg-dev
      debhelper
      desktop-file-utils
      appstream
      flatpak-builder
      libclang-dev
      libcairo2-dev
      libpango1.0-dev
      libgdk-pixbuf-2.0-dev
      libgdk-pixbuf-xlib-2.0-dev
      libglib2.0-dev
      libgraphene-1.0-dev
      libgtk-4-dev
      libgstreamer1.0-dev
      libgstreamer-plugins-base1.0-dev
      libadwaita-1-dev
      libdiscid-dev
      libgpgme-dev
      libgcrypt20-dev
      libcurl4-openssl-dev
      curl
      squashfs-tools
      zsync
      gstreamer1.0-plugins-base
      gstreamer1.0-plugins-good
      gstreamer1.0-plugins-ugly
      cdparanoia
      cd-discid
      eject
      flac
      lame
      vorbis-tools
    )
    mapfile -t missing_packages < <(missing_debian_packages "${debian_packages[@]}")

    if (( ${#missing_packages[@]} == 0 )); then
      echo "All Debian/Ubuntu/Kali packages are already installed."
    else
      echo "Installing missing Debian/Ubuntu/Kali packages:"
      printf '  %s\n' "${missing_packages[@]}"
      as_root apt-get update
      as_root apt-get install -y "${missing_packages[@]}"
    fi
    echo "Done."
  elif [[ $family == arch ]]; then
    echo "Detected pacman (Arch). Checking packages..."
    pacman_packages=(
      base-devel
      devtools
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
      as_root pacman -S --needed "${missing_packages[@]}"
    fi
    echo "Done."
  elif [[ $family == fedora ]]; then
    dnf_cmd=$(dnf_command)
    [[ -n $dnf_cmd ]] || { echo "Fedora requires dnf or dnf5." >&2; return 1; }
    echo "Detected dnf (Fedora/RHEL). Checking packages..."
    # Fedora supplies lame and the freely distributable ugly plugin subset.
    # Additional codecs require a separately configured repository such as RPM Fusion.
    # Fedora equivalents for Debian Build-Depends:
    #   debhelper-compat (= 13) -> debhelper
    #   rustc -> rust
    #   pkg-config -> pkgconf-pkg-config
    #   libclang-dev -> clang-devel
    #   libdiscid-dev -> libdiscid-devel
    #   libglib2.0-dev -> glib2-devel
    #   libgstreamer1.0-dev -> gstreamer1-devel
    #   libgstreamer-plugins-base1.0-dev -> gstreamer1-plugins-base-devel
    #   libgtk-4-dev -> gtk4-devel
    #   libadwaita-1-dev -> libadwaita-devel
    #   libgdk-pixbuf-2.0-dev -> gdk-pixbuf2-devel
    #   libgdk-pixbuf-xlib-2.0-dev -> gdk-pixbuf2-xlib-devel
    #   libcairo2-dev -> cairo-devel
    #   libpango1.0-dev -> pango-devel
    #   libgraphene-1.0-dev -> graphene-devel
    #   libgpgme-dev -> gpgme-devel
    #   libgcrypt20-dev -> libgcrypt-devel
    #   libcurl4-openssl-dev -> libcurl-devel
    #   libgstreamer1.0-0 -> gstreamer1
    #   libgstreamer-plugins-base1.0-0 -> gstreamer1-plugins-base
    #   libadwaita-1-0 -> libadwaita
    #   libdiscid0 -> libdiscid
    dnf_deps=(
      gcc
      gcc-c++
      make
      cmake
      cmake-gui
      cmake-extras
      cmakelang
      extra-cmake-modules
      ninja-build
      ccache
      cargo
      rust
      rust-src
      dpkg-dev
      debhelper
      rpm-build
      rpmdevtools
      pkgconf-pkg-config
      clang
      clang-devel
      clang-tools-extra
      cppcheck
      doxygen
      graphviz
      desktop-file-utils
      appstream
      libappstream-glib
      flatpak-builder
      squashfs-tools
      zsync
      curl
    )
    dnf_libs=(
      glib2-devel
      glib2
      cairo-devel
      cairo
      pango-devel
      pango
      gdk-pixbuf2-devel
      gdk-pixbuf2
      gdk-pixbuf2-xlib-devel
      gdk-pixbuf2-xlib
      graphene-devel
      graphene
      gtk4-devel
      gtk4
      gstreamer1-devel
      gstreamer1-plugins-base-devel
      gstreamer1-plugins-base
      gstreamer1-plugins-good
      gstreamer1-plugins-ugly-free
      libadwaita-devel
      libadwaita
      'pkgconfig(libdiscid)'
      libdiscid-devel
      libdiscid
      gpgme-devel
      gpgme
      libgcrypt-devel
      libgcrypt
      libcurl-devel
      libcurl
      cdparanoia
      cd-discid
      util-linux
      flac
      lame
      vorbis-tools
    )
    dnf_packages=("${dnf_deps[@]}" "${dnf_libs[@]}")
    mapfile -t missing_packages < <(missing_dnf_packages "${dnf_packages[@]}")

    if (( ${#missing_packages[@]} == 0 )); then
      echo "All Fedora/RHEL packages are already installed."
    else
      echo "Installing missing Fedora/RHEL packages:"
      printf '  %s\n' "${missing_packages[@]}"
      as_root "$dnf_cmd" install -y "${missing_packages[@]}"
    fi
    echo "Done."
  elif [[ $family == suse ]]; then
    # Note: 'lame' and 'cd-discid' may require the Packman repository on openSUSE:
    #   sudo zypper ar -cfp 90 https://ftp.gwdg.de/pub/linux/misc/packman/suse/openSUSE_Tumbleweed/ packman
    #   sudo zypper dup --from packman --allow-vendor-change
    echo "Detected zypper (openSUSE). Installing packages..."
    as_root zypper install -y \
      gcc \
      make \
      rpm-build \
      rpmlint \
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
    as_root zypper install -y rust-src || \
      echo "rust-src was not available from configured openSUSE repositories; continuing."
    echo "Done."
  elif [[ $family == slackware ]]; then
    install_slackware
  else
    echo "Unsupported package manager. Please install dependencies manually." >&2
    exit 1
  fi
}

# Slackware combines headers, libraries and tools in the same package: llvm
# provides clang and its tools, rust provides cargo, and cmake provides its GUIs.
# Extra packages must be supplied by configured slackpkg+ repositories (as on
# Slackware Slax) or installed separately. Never silently skip missing packages.
slackware_installed() {
  local package=$1 entry name directory
  shift
  # Optional directories allow verification against fixture package databases.
  if (( $# == 0 )); then set -- /var/lib/pkgtools/packages /var/log/packages; fi
  for directory in "$@"; do
    for entry in "$directory/$package"-*; do
      [[ -f $entry ]] || continue
      name=${entry##*/}
      name=${name%-*}; name=${name%-*}; name=${name%-*}
      [[ $name == "$package" ]] && return 0
    done
  done
  return 1
}

install_slackware() {
  local package
  local -a missing=() unresolved=()
  local -a packages=(
    gcc gcc-g++ binutils glibc make cmake cmake-extras cmakelang
    extra-cmake-modules ninja ccache llvm cppcheck doxygen graphviz
    rust rust-src pkg-config dpkg debhelper rpm desktop-file-utils
    AppStream appstream-glib flatpak flatpak-builder squashfs-tools curl zsync
    glib2 cairo pango gdk-pixbuf2 gdk-pixbuf2-xlib graphene gtk4
    gstreamer gst-plugins-base gst-plugins-good gst-plugins-ugly
    libadwaita libdiscid gpgme libgcrypt cdparanoia cd-discid
    util-linux flac lame vorbis-tools
  )
  have slackpkg || { echo "Slackware/Slax requires slackpkg." >&2; return 1; }
  for package in "${packages[@]}"; do
    slackware_installed "$package" || missing+=("$package")
  done
  if (( ${#missing[@]} == 0 )); then
    echo "All Slackware/Slax packages are already installed."
    return 0
  fi
  echo "Installing Slackware/Slax packages from configured repositories."
  echo "slackpkg does not resolve dependencies; a full Slackware installation is recommended."
  as_root slackpkg update
  as_root slackpkg -batch=on -default_answer=y install "${missing[@]}"
  # slackpkg can succeed even if a requested package is absent from its mirrors.
  for package in "${missing[@]}"; do
    slackware_installed "$package" || unresolved+=("$package")
  done
  if (( ${#unresolved[@]} )); then
    echo "Dependencies still missing from the installed Slackware packages:" >&2
    printf '  %s\n' "${unresolved[@]}" >&2
    echo "Configure matching slackpkg+ repositories or build/install these packages and their dependencies." >&2
    echo "Use repositories for your Slackware release; do not mix stable and current." >&2
    return 1
  fi
  echo "Done."
}

# Record actual package identities, including dependencies pulled in by the
# solver. Versions are excluded so upgrades do not acquire ownership.
installed_packages() {
  case "$1" in
    debian)
      dpkg-query -W -f='${db:Status-Abbrev} ${binary:Package}\n' |
        awk 'substr($0, 2, 1) ~ /[iUFWtH]/ {print $2}' ;;
    fedora|suse) rpm -qa --qf '%{NAME}.%{ARCH}\n' ;;
    arch) pacman -Qq ;;
    slackware)
      local directory entry name
      for directory in /var/lib/pkgtools/packages /var/log/packages; do
        for entry in "$directory"/*; do
          [[ -f $entry ]] || continue
          name=${entry##*/}
          name=${name%-*}; name=${name%-*}; name=${name%-*}
          printf '%s\n' "$name"
        done
      done ;;
    *) return 1 ;;
  esac | LC_ALL=C sort -u
}

remove_packages() {
  local family=$1 manager answer
  shift
  echo "Packages recorded by this installer:"
  printf '  %s\n' "$@"
  echo "Review the removal transaction: other software may now depend on these packages."
  case "$family" in
    debian) apt-get -o APT::Get::AutomaticRemove=false -o APT::Get::Assume-Yes=false remove "$@" ;;
    fedora)
      manager=$(dnf_command)
      [[ -n $manager ]] || return 1
      "$manager" --setopt=clean_requirements_on_remove=False --setopt=assumeyes=False remove "$@" ;;
    arch) pacman -R "$@" ;;
    suse) zypper remove "$@" ;;
    slackware)
      read -r -p 'Remove these packages? [y/N] ' answer
      [[ $answer == y || $answer == Y ]] || return 1
      removepkg "$@" ;;
  esac
}

# Run in a subshell so the EXIT trap also records partially failed installations.
# The caller holds the lock and runs as root; the state directory is root-owned.
tracked_action() (
  local family=$1 action=$2 state=$3 work ledger
  local -a packages=()
  export LC_ALL=C
  ledger=$state/$family.packages
  work=$(mktemp -d "$state/transaction.XXXXXX")
  touch "$ledger"
  installed_packages "$family" > "$work/before"

  # shellcheck disable=SC2329 # Invoked by the EXIT trap.
  finish_tracking() {
    local status=$? record_status=0
    trap - EXIT
    installed_packages "$family" > "$work/after" || record_status=$?
    if (( record_status == 0 )); then
      if [[ $action == install ]]; then
        comm -13 "$work/before" "$work/after" > "$work/added"
      else
        : > "$work/added"
      fi
      sort -u "$ledger" "$work/added" > "$work/owned"
      comm -12 "$work/owned" "$work/after" > "$work/next"
      mv "$work/next" "$ledger"
      rm -rf -- "$work"
    else
      echo "Could not update installation record; transaction snapshots retained at $work" >&2
      status=$record_status
    fi
    exit "$status"
  }
  trap finish_tracking EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  if [[ $action == install ]]; then
    main
  else
    mapfile -t packages < <(comm -12 "$ledger" "$work/before")
    if (( ${#packages[@]} == 0 )); then
      echo "No recorded installed packages to remove. Earlier untracked installations cannot be removed automatically."
      return 0
    fi
    remove_packages "$family" "${packages[@]}"
  fi
)

cli() {
  local action=install family
  if (( $# > 1 )); then
    echo "Usage: $0 [--remove|-R|--help|-h]" >&2
    return 2
  fi
  case "${1:-}" in
    '') ;;
    --remove|-R) action=remove ;;
    --help|-h)
      echo "Usage: $0 [--remove|-R|--help|-h]"
      echo "Without options: install dependencies and record newly installed packages."
      echo "--remove, -R: remove recorded packages, with package-manager confirmation."
      return 0 ;;
    *) echo "Unknown option: $1" >&2; return 2 ;;
  esac
  family=$(detect_family) || { echo "Unsupported distribution." >&2; return 1; }
  if (( EUID != 0 )); then
    if ! have sudo; then
      echo "Run this script as root or install sudo." >&2
      return 1
    fi
    sudo bash "${BASH_SOURCE[0]}" "$@"
    return
  fi
  # Shared across users and checkouts; never infer ownership from the dependency list.
  local state=/var/lib/ceedee-ripper/install-deps
  umask 077
  mkdir -p "$state"
  exec 9> "$state/lock"
  flock -n 9 || { echo "Another dependency installer is running." >&2; return 1; }
  tracked_action "$family" "$action" "$state"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
  cli "$@"
fi
