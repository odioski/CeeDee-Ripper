
![CeeDeeRipper-action](https://github.com/user-attachments/assets/2a6f6f28-37f1-408d-bae7-c336c29cdb5f)

- Arch Linux:

```bash
 # Only libdiscid is required at runtime for CD TOC reading
 sudo pacman -S --needed libdiscid
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
