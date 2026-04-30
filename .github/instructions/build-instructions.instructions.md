---
description: "Use when editing Build Instructions.md or advising on packaging, release, or build commands for this repository."
applyTo: "**/Build Instructions.md"
---

# Build & Packaging Guardrails

When working on build or packaging guidance in this repository:

- Follow the build workflows already documented: Snap, Cargo source, and system-wide install.
- The canonical Snap build sequence is `snapcraft` then `snap install --dangerous ./ceedee-ripper_*.snap`.
- The canonical Cargo release build is `cargo build --release`; use `cargo check && cargo run` for iterative dev testing.
- For system-wide install the binary path is `/usr/local/bin/ceedee-ripper` and the desktop/icon paths follow the XDG hierarchy already in the file.
- Cross-compile targets: `x86_64-unknown-linux-gnu` (Linux), `x86_64-pc-windows-msvc` (Windows MSVC), or `x86_64-pc-windows-gnu` (cross from Linux via mingw-w64).
- Do not introduce new build steps or targets unless explicitly requested.
- Keep commands minimal, tested, and consistent with what is already in the file.

Authoritative references:

- https://documentation.ubuntu.com/snapcraft/stable/reference/project-file/snapcraft-yaml/
- https://doc.rust-lang.org/cargo/commands/cargo-build.html
- https://rustup.rs/
