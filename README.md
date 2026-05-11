![CeeDee Ripper](resources/CeeDee-Ripper.png)

# CeeDee Ripper

CeeDee Ripper is a Linux desktop app for extracting audio CDs to FLAC, MP3, WAV, or Ogg Vorbis.

The default build compiles both the egui and GTK4/Libadwaita interfaces. The saved `ui_backend` config key chooses which one launches, and the repo default is egui.

The display name is **CeeDee Ripper**. The package, binary, desktop ID, and Rust crate name use **ceedee-ripper**.

## Features

- Detects inserted audio CDs.
- Looks up album and track metadata from MusicBrainz.
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

Building from source also requires Rust, `pkg-config`, `libdiscid` development headers, GStreamer development headers, GTK4 development headers, and Libadwaita development headers. Use `--no-default-features --features egui-ui` for an egui-only build without GTK dependencies.

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

Default build with both UIs enabled:

```bash
cargo run
```

Select a UI in a universal build:

```bash
cargo run -- --ui egui
cargo run -- --ui gtk
cargo run -- --features egui-ui
cargo run -- --features gtk-ui
ceedee-ripper --features egui-ui
ceedee-ripper --features gtk-ui
```

Cargo consumes `--features` before `--` as build-time feature flags. The
installed app only sees arguments after `--`, so `cargo run --features gtk-ui`
changes Cargo's build feature set but does not select GTK at runtime. Use
`cargo run -- --features gtk-ui` or `cargo run -- --ui gtk` when testing the
runtime selector through Cargo.

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

When run from this checkout, settings are stored at:

```text
config/config.toml
```

Installed builds use:

```text
~/.config/ceedee-ripper/config.toml
```

Set `CEEDEE_RIPPER_CONFIG=/path/to/config.toml` to force a specific config file.

Useful keys:

```toml
device = "/dev/sr0"
encoder = "flac"
metadata_source = "musicbrainz"
album_art_size_preference = "auto"
album_art_download_behavior = "preview-only"
ui_backend = "egui"
```

In a universal build, `--ui egui` or `--ui gtk` selects the interface for the current launch and saves it as `ui_backend`. Installed binaries also accept `--features egui-ui` and `--features gtk-ui` as runtime aliases for the same selector. The Settings page and View menu also save this setting; restart the app to apply it.

## Packaging

Local packaging notes are in [docs/PACKAGING.md](docs/PACKAGING.md).

This project is being prepared for future repository and package uploads, but packaging commands in this checkout should remain local unless publishing is explicitly requested.
