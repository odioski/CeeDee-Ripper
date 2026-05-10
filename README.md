![CeeDee Ripper](resources/CeeDee-Ripper.png)

# CeeDee Ripper

CeeDee Ripper is a Linux desktop app for extracting audio CDs to FLAC, MP3, WAV, or Ogg Vorbis.

The default build uses the egui interface. A GTK4/Libadwaita interface is also available as the `gtk-ui` Cargo feature.

The display name is **CeeDee Ripper**. The package, binary, desktop ID, and Rust crate name use **ceedee-ripper**.

## Features

- Detects inserted audio CDs.
- Looks up album and track metadata from MusicBrainz or GnuDB/CDDB.
- Lets you choose which tracks to rip.
- Saves album art previews, with an option to save cover art alongside the rip.
- Encodes to FLAC, MP3, WAV, or Ogg Vorbis using local system tools.

## Requirements

CeeDee Ripper needs access to an optical drive and these runtime tools:

- `cdparanoia`
- `cd-discid`
- `eject`
- `flac`
- `lame`
- `vorbis-tools`
- GStreamer base/good plugins, including the cdparanoia source and `wavenc`

Building from source also requires Rust, `pkg-config`, `libdiscid` development headers, and GStreamer development headers. GTK4 and Libadwaita development headers are only required when building with `--no-default-features --features gtk-ui`.

On Debian or Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y cdparanoia cd-discid eject flac lame vorbis-tools \
  libdiscid-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-ugly pkg-config
```

For the optional GTK UI:

```bash
sudo apt-get install -y libgtk-4-dev libadwaita-1-dev
```

## Build And Run

Default egui UI:

```bash
cargo run
```

GTK UI:

```bash
cargo run --no-default-features --features gtk-ui
```

Release build:

```bash
cargo build --release
```

Install locally with Cargo:

```bash
cargo install --path .
```

## Optical Drive Access

By default, CeeDee Ripper tries `CD_DEVICE`, then the saved config value, then common devices such as `/dev/cdrom` and `/dev/sr0`.

To force a device for one run:

```bash
CD_DEVICE=/dev/sr0 cargo run
```

On Debian or Ubuntu, your user may need to be in the `cdrom` group:

```bash
sudo usermod -aG cdrom "$USER"
```

Log out and back in before testing the group change.

## Configuration

Settings are stored at:

```text
~/.config/ceedee-ripper/config.toml
```

Useful keys:

```toml
device = "/dev/sr0"
encoder = "flac"
metadata_source = "musicbrainz"
album_art_size_preference = "auto"
album_art_download_behavior = "preview-only"
```

## Packaging

Local packaging notes are in [docs/PACKAGING.md](docs/PACKAGING.md).

This project is being prepared for future repository and package uploads, but packaging commands in this checkout should remain local unless publishing is explicitly requested.
