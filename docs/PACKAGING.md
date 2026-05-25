![CeeDee Ripper](../resources/CeeDee-Ripper.png)

# Packaging Notes

These notes are for local validation before any future repository or package upload. Do not push, publish, upload, or trigger remote packaging from this checkout unless explicitly requested.

## Local Build Matrix

Default builds compile both egui and GTK4/Libadwaita into one binary. The launched interface is selected at runtime with `--ui`, the app-level `--feature` alias, or the saved `ui_backend` config key.

Universal UI binary:

```bash
cargo check --feature "gtk-ui egui-ui"
cargo build --release --feature "gtk-ui egui-ui"
```

In universal builds, `ceedee-ripper --ui egui` and `ceedee-ripper --ui gtk`
select the interface for that launch and save the choice. Installed binaries
also accept `ceedee-ripper --feature egui-ui` and
`ceedee-ripper --feature gtk-ui` as runtime aliases for the same selector.
Without a selector, the app uses `ui_backend` from
`~/.config/ceedee-ripper/config.toml`, falling back to egui when available. The
Settings page and View menu write the same setting and require a restart.

When testing through Cargo, put runtime selectors after `--`, for example
`cargo run --feature "gtk-ui egui-ui" -- --feature gtk-ui`. A command like
`cargo run --feature gtk-ui` only changes Cargo's build feature; it does not
pass `--feature gtk-ui` to the running app.

For now, keep universal-build packaging work scoped to `.deb` and AppImage
artifacts. Snap and Flatpak remain deferred until their runtime issues are
handled separately.

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

Initial priority recipe files live under `packaging/`:

- `packaging/ubuntu/debian/` for Ubuntu source package/PPA preparation
- `packaging/appimage/` for direct GitHub release artifacts

Current priority is Debian `.deb` packages and AppImage artifacts. Snap and
Flatpak are intentionally out of scope for this universal-UI packaging pass.

These recipes assume the GitHub release tag `v1.1.0` as the upstream source anchor.

## Future Repos

Treat GitHub, crates.io, Flathub, Apt/PPA, AUR, COPR, and similar destinations as release targets only. Keep preparation local until a release/publishing step is explicitly authorized.
