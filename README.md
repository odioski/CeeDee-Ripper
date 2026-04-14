# CeeDee Ripper — Advanced

A GTK4/Libadwaita-based audio CD ripper for Linux.

# ![Screenshot_20260209_022455](https://github.com/user-attachments/assets/14ba465d-656c-4e89-84a0-5e367b922f3e)


## Dependencies

This app uses external system tools and libraries to detect CDs, read tracks, and encode audio. Install these before running:

- cdparanoia: secure digital audio extraction (CLI fallback)
- cd-discid: disc ID for track/TOC calculations
- flac: FLAC encoder (when encoder = "flac")
- lame: MP3 encoder (when encoder = "mp3")
 - vorbis-tools: OGG/Vorbis encoder (`oggenc`, when encoder = "ogg")

Library-based ripping uses GStreamer:
- GStreamer core and plugins: `base` and `good` sets (provides `cdparanoia` source and `wavenc`)

Additionally, you need GTK4, Libadwaita, and libdiscid development packages to build from source.

## Quick Install (Linux)

Use the helper script to install dependencies on common distros:

```bash
# From the project root
bash scripts/install-deps.sh
```

The script supports `apt` (Debian/Ubuntu), `pacman` (Arch), and `dnf` (Fedora/RHEL). If a package is unavailable, the script prints a hint to enable the appropriate repository (e.g., RPM Fusion for `lame` on Fedora).

## Manual Install

- Debian/Ubuntu:

```bash
sudo apt-get update
 sudo apt-get install -y cdparanoia cd-discid flac lame \
  vorbis-tools libgtk-4-dev libadwaita-1-dev libdiscid-dev \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good
```

- Arch Linux:

```bash
 sudo pacman -S --needed cdparanoia cd-discid flac lame \
  vorbis-tools gtk4 libadwaita libdiscid gst-plugins-base gst-plugins-good
```

- Fedora (may require RPM Fusion for `lame`):

```bash
 sudo dnf install -y cdparanoia cd-discid flac lame \
  vorbis-tools gtk4-devel libadwaita-devel libdiscid-devel \
  gstreamer1-plugins-base gstreamer1-plugins-good || echo "If 'lame' is missing, enable RPM Fusion."
```

## Device Configuration

By default the app uses `/dev/sr0`. You can override the device:

- Config file: `~/.config/ceedee-ripper/config.toml`

```toml
device = "/dev/sr0"
encoder = "flac"   # flac, mp3, wav, etc.
bitrate = "320"     # for mp3
quality = "8"       # for flac/ogg (0-10 for ogg)
cddb_enabled = true
```

- Environment variable (takes precedence):

```bash
export CD_DEVICE=/dev/sr0
```

## Permissions

Your user should have permission to read from the CD device:

```bash
# On Ubuntu/Debian, add user to the 'cdrom' group
sudo usermod -aG cdrom "$USER"
# Log out and back in (or reboot) for group change to apply.
```

You can test detection/ripping manually:

```bash
cdparanoia -d /dev/sr0 -Q
cd-discid /dev/sr0
# GStreamer pipeline (library-based ripping) example:
gst-launch-1.0 cdparanoia device=/dev/sr0 track=1 ! wavenc ! filesink location=track01.wav
```

## Build & Run

```bash
cargo build
cargo run
```

## Troubleshooting

- "No CD detected on /dev/...": Ensure an audio CD is inserted, device path is correct, and permissions allow access.
- Missing encoders: Use `flac` or `lame` per your chosen output format.
- Fedora `lame` not found: Enable RPM Fusion (free) repos, then install `lame`.
