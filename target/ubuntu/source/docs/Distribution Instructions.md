# Distribution Instructions

These notes describe how to publish CeeDee Ripper artifacts after local builds are complete. Do not upload, push, submit, or publish anything from this checkout unless that release action has been explicitly approved.

## Preflight

1. Confirm the release version and tag match the package recipes.
   - Current recipe target: `1.1.0`
   - Expected upstream tag: `v1.1.0`
2. Confirm the working tree contains only intentional packaging changes.
   ```bash
   git status --short --branch
   ```
3. Confirm installable local artifacts exist under `target/`.
   ```bash
   find target -type f \( -name '*.deb' -o -name '*.AppImage' -o -name '*.pkg.tar*' -o -name '*.rpm' -o -name '*.flatpak' \) -print
   ```
4. Test install each artifact on a matching target system before publishing it.
5. Keep generated artifacts out of the repo root.

## GitHub Release

Use GitHub Releases as the upstream artifact home for direct downloads and as the source anchor for distro recipes.

1. Create and push the release tag only after final approval.
   ```bash
   git tag -a v1.1.0 -m "CeeDee Ripper 1.1.0"
   git push origin v1.1.0
   ```
2. Create a GitHub release for `v1.1.0`.
3. Upload direct-download artifacts, typically:
   - `target/appimage/ceedee-ripper-1.1.0-x86_64.AppImage`
   - `target/debian/ceedee-ripper_1.1.0-1_amd64.deb`
4. Include release notes with supported Linux targets, runtime dependencies, and checksums.

## AppImage

The AppImage is a direct release artifact, not a distro repository submission.

1. Build locally:
   ```bash
   packaging/appimage/build-appimage.sh
   ```
2. Test on clean Linux systems.
   ```bash
   chmod +x target/appimage/ceedee-ripper-1.1.0-x86_64.AppImage
   ./target/appimage/ceedee-ripper-1.1.0-x86_64.AppImage
   ```
3. Upload the final AppImage to the GitHub release.
4. Optionally publish checksums beside it.

## Ubuntu PPA

Use Launchpad PPA for Ubuntu source package publishing. Do not upload the local `.deb` directly to a PPA; PPAs build from source uploads.

1. Ensure Debian packaging under `packaging/debian/` is final.
2. Build a source package on an Ubuntu build host with the target series available.
3. Sign the source package with the Launchpad-approved GPG key.
4. Upload using `dput` to the configured PPA.
5. Monitor Launchpad builders for each Ubuntu series.
6. Test the resulting PPA package on a clean Ubuntu system.

## AUR

Use the AUR Git repository for Arch users. The AUR should receive packaging files, not the built `.pkg.tar.*` artifact.

1. Update `packaging/aur/PKGBUILD`.
2. Regenerate `.SRCINFO` on an Arch system:
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```
3. Test the package on Arch:
   ```bash
   makepkg -si
   ```
4. Clone the AUR package repository.
5. Copy in `PKGBUILD` and `.SRCINFO`.
6. Commit and push to the AUR Git remote.

Local test artifact, when built, is only for local installation testing:
`target/aur-build/ceedee-ripper-1.1.0-1-x86_64.pkg.tar.gz`

## Fedora COPR

Use COPR for Fedora/RPM publishing before considering an official Fedora package review.

1. Ensure `packaging/fedora/ceedee-ripper.spec` is final.
2. Build a source RPM:
   ```bash
   rpmbuild -bs packaging/fedora/ceedee-ripper.spec
   ```
3. Submit the SRPM to COPR using the web UI or `copr-cli`.
4. Monitor builds for the selected Fedora targets.
5. Test the resulting RPM on clean Fedora systems.

Local binary RPMs built under `target/fedora/rpmbuild/RPMS/` are for local testing only.

## Flatpak / Flathub

Flatpak should be handled after the other artifacts. For a local test bundle, a manifest can build from the local checkout. For Flathub, the manifest should use stable, reproducible upstream sources and vendored Cargo sources.

1. Finish the Flatpak manifest under `packaging/flatpak/`.
2. Ensure native dependencies not present in the Freedesktop SDK are declared as Flatpak modules.
3. Generate vendored Cargo sources from `Cargo.lock` using the Flatpak Cargo source generator.
4. Build locally:
   ```bash
   flatpak-builder --force-clean --repo=target/flatpak-repo target/flatpak-build packaging/flatpak/io.github.odioski.ceedee_ripper.yml
   ```
5. Bundle locally for transfer/testing:
   ```bash
   flatpak build-bundle target/flatpak-repo target/ceedee-ripper-1.1.0-x86_64.flatpak io.github.odioski.ceedee_ripper
   ```
6. Test install:
   ```bash
   flatpak install --user target/ceedee-ripper-1.1.0-x86_64.flatpak
   flatpak run io.github.odioski.ceedee_ripper
   ```
7. For Flathub submission, open a pull request against the Flathub manifest repository after the app ID, desktop file, icon, metainfo, screenshots, permissions, and generated sources are Flathub-ready.

## Checksums

Generate checksums for all final release artifacts:

```bash
sha256sum target/appimage/*.AppImage target/debian/*.deb target/aur-build/*.pkg.tar* target/fedora/rpmbuild/RPMS/**/*.rpm target/*.flatpak
```

If a glob does not match because that artifact was not built, omit it from the checksum command.
