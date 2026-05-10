![CeeDee Ripper](../resources/CeeDee-Ripper.png)

# Packaging Notes

These notes are for local validation before any future repository or package upload. Do not push, publish, upload, or trigger remote packaging from this checkout unless explicitly requested.

## Local Build Matrix

Official package builds use the default egui UI. The default Cargo feature set is `egui-ui`, so packaging recipes that run plain `cargo build --release` produce egui artifacts. The GTK4/Libadwaita interface remains available for explicit builds with `--no-default-features --features gtk-ui`.

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

Package recipes may ship the default egui binary or produce a GTK variant by
changing the build command to `cargo build --release --no-default-features
--features gtk-ui`. Keep runtime dependencies matched to the UI variant being
shipped.

## Debian/Ubuntu

Local metadata/package check:

```bash
cargo deb --no-strip
```

The Debian package metadata is defined in `Cargo.toml` under `[package.metadata.deb]`. It installs:

- `usr/bin/ceedee-ripper`
- `usr/share/applications/ceedee-ripper.desktop`
- `usr/share/icons/hicolor/256x256/apps/io.github.odioski.ceedee_ripper.png`
- `usr/share/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml`

Before a real package release, verify runtime dependencies on a clean Ubuntu install and decide whether package-level dependency declarations should be pinned more tightly.

## Desktop And AppStream

Validate locally:

```bash
desktop-file-validate resources/ceedee-ripper.desktop
appstreamcli validate --pedantic resources/metainfo/io.github.odioski.ceedee_ripper.metainfo.xml
```

The current desktop file remains `ceedee-ripper.desktop`, while the AppStream ID and icon name use `io.github.odioski.ceedee_ripper`.

## Recipe Targets

Initial recipe files live under `packaging/`:

- `packaging/flatpak/` for Flathub preparation
- `packaging/ubuntu/debian/` for Ubuntu source package/PPA preparation
- `packaging/appimage/` for direct GitHub release artifacts
- `packaging/aur/` for Arch User Repository preparation
- `packaging/fedora/` for Fedora/COPR RPM preparation

These recipes assume the GitHub release tag `v1.1.0` as the upstream source anchor.

## Future Repos

Treat GitHub, crates.io, Flathub, Apt/PPA, AUR, COPR, and similar destinations as release targets only. Keep preparation local until a release/publishing step is explicitly authorized.
