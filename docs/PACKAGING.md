# Packaging Notes

These notes are for local validation before any future repository or package upload. Do not push, publish, upload, or trigger remote packaging from this checkout unless explicitly requested.

## Local Build Matrix

Default egui UI:

```bash
cargo check
cargo build
```

GTK UI:

```bash
cargo check --no-default-features --features gtk-ui
cargo build --no-default-features --features gtk-ui
```

The `gtk-ui` and `egui-ui` features are intentionally mutually exclusive.

## Debian/Ubuntu

Local metadata/package check:

```bash
cargo deb --no-strip
```

The Debian package metadata is defined in `Cargo.toml` under `[package.metadata.deb]`. It installs:

- `usr/bin/ceedee-ripper`
- `usr/share/applications/ceedee-ripper.desktop`
- `usr/share/icons/hicolor/256x256/apps/ceedee-ripper.png`
- `usr/share/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml`

Before a real package release, verify runtime dependencies on a clean Ubuntu install and decide whether package-level dependency declarations should be pinned more tightly.

## Desktop And AppStream

Validate locally:

```bash
desktop-file-validate resources/ceedee-ripper.desktop
appstreamcli validate --pedantic resources/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml
```

The current desktop file remains `ceedee-ripper.desktop`. For Flathub readiness, expect to revisit reverse-DNS naming across the desktop file, app ID, icons, and metainfo ID.

## Snap

Snap metadata lives in `snapcraft.yaml`. It is aligned with the Cargo package version and MIT license.

Local-only checks:

```bash
python3 -c 'import yaml; yaml.safe_load(open("snapcraft.yaml")); print("snapcraft.yaml: YAML OK")'
snapcraft
snapcraft lint ./ceedee-ripper_*.snap
```

Do not upload to the Snap Store from this checkout unless explicitly requested.

## Future Repos

Treat GitHub, crates.io, Snap Store, Flathub, Apt/PPA, AUR, COPR, and similar destinations as release targets only. Keep preparation local until a release/publishing step is explicitly authorized.
