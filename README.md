# CeeDee Ripper

![CeeDee Ripper](resources/CeeDee-Ripper.png)

CeeDee Ripper is a Linux desktop app for extracting audio CDs to common music
formats. It detects an inserted disc, looks up album metadata through
MusicBrainz, previews album art, and rips selected tracks to FLAC, MP3, WAV, or
Ogg Vorbis.

The app is written in Rust and currently supports two desktop front ends:

- `egui-ui`
- `gtk-ui` with GTK4 and Libadwaita

Default builds include both front ends, and the active interface can be selected
at runtime.

## Screenshots

![CeeDee Ripper main window](Screenshot_20260522_234929.png)

![CeeDee Ripper ripping view](Screenshot_20260522_235511.png)

## Requirements

CeeDee Ripper needs normal Rust build tools plus the native libraries and
command-line tools used for CD access, metadata, and encoding.

On supported Linux systems, the helper script can install the common packages:

```bash
scripts/install-deps.sh
```

Important runtime tools and libraries include:

- `cdparanoia`
- `cd-discid`
- `eject`
- `flac`
- `lame`
- `vorbis-tools`
- `libdiscid`
- GStreamer base, good, and ugly plugins
- GTK4 and Libadwaita for the GTK interface

Your user may also need permission to read the optical drive. On many Linux
systems that means being in the `cdrom` group and logging in again:

```bash
sudo usermod -aG cdrom "$USER"
```

## Build And Run

Build the normal release binary:

```bash
cargo build --release --features "gtk-ui egui-ui"
```

Run with the default saved interface:

```bash
cargo run --features "gtk-ui egui-ui"
```

Select an interface for a launch:

```bash
cargo run --features "gtk-ui egui-ui" -- --ui egui
cargo run --features "gtk-ui egui-ui" -- --ui gtk
```

The selected UI is saved in the app config. By default the app uses
`~/.config/ceedee-ripper/config.toml`, unless a repo-local config file or
`CEEDEE_RIPPER_CONFIG` is used.

The CD device defaults to `/dev/sr0`. You can override it with the config file
or with:

```bash
CD_DEVICE=/dev/sr1 ceedee-ripper
```

## Packaging

Packaging work lives under `packaging/`, with notes in `docs/PACKAGING.md` and
`Distribution Instructions.md`.

The most straightforward local targets are:

- Debian/Ubuntu `.deb`
- AppImage
- Arch/AUR recipe
- Fedora/RPM recipe

The Debian package metadata is also present in `Cargo.toml`.

## Notes On Flatpak And Snap

Flatpak and Snap builds are present, but they are more complicated than the
native packages for this app.

CeeDee Ripper is not just a self-contained GUI. It needs low-level access to an
optical drive, reads disc table-of-contents data, calls CD helper tools, uses
GStreamer encoders, talks to MusicBrainz, and writes music files to user-visible
locations. Sandboxed package formats make each of those pieces stricter:

- device access to `/dev/cdrom`, `/dev/sr0`, or `/dev/sr1` has to be granted;
- udev, removable media, and drive permissions may differ by distribution;
- command-line helpers and codecs must be staged inside the sandbox;
- GStreamer plugin availability has to match what the app expects;
- network access is needed for metadata and cover art;
- output folders must be exposed deliberately;
- reproducible Rust/Cargo dependency vendoring is required for Flathub-style
  builds.

The Flatpak manifest currently grants broad device and filesystem permissions
for local testing and vendors Cargo sources through `generated-sources.json`.
That is useful for development, but a Flathub-ready version still needs careful
permission review, stable source generation, and validation on clean systems.

The Snap recipe uses strict confinement and plugs such as `optical-drive`,
`removable-media`, `mount-observe`, `network`, `wayland`, and `x11`. That is the
right general shape, but optical-drive access and desktop/media integration can
still require manual connections or target-system testing before it behaves like
a native package.

For now, treat Flatpak and Snap as experimental packaging paths. Native packages
and AppImage are the simpler release artifacts to validate first.

## License

CeeDee Ripper is released under the MIT License. See `LICENSE`.
