# Build instructions to target Linux or Windows Systems.

These commands set up the Rust toolchain to build for the specific operating system.

## Installation

### Snap

```bash
# Build the snap
snapcraft

# Install from the built snap package
snap install --dangerous ./ceedee-ripper_*.snap
```

### From Source (Cargo)

First, ensure you've installed the dependencies (see README.md [Dependencies](#dependencies) section).

```bash
# Build
cargo build --release

# Run directly
cargo run --release

# Or install to ~/.cargo/bin
cargo install --path .
ceedee-ripper
```

### System-Wide Installation

After building with `cargo build --release`, install to `/usr/local/bin`:

```bash
# Build first
cargo build --release

# Install binary
sudo install -Dm755 target/release/CeeDee-Ripper /usr/local/bin/ceedee-ripper

# Install desktop entry
sudo install -Dm644 resources/ceedee-ripper.desktop \
  /usr/share/applications/ceedee-ripper.desktop

# Install icon
sudo install -Dm644 resources/images/ceedee-ripper.png \
  /usr/share/icons/hicolor/256x256/apps/ceedee-ripper.png

# Update icon cache (optional)
sudo gtk-update-icon-cache /usr/share/icons/hicolor/

# Now run from anywhere
ceedee-ripper
```

## Plain Build - Linux

Use cargo check && cargo run to launch CeeDee-Ripper for testing.
If you make changes, use cargo build until you clear any errors or problems. Then use cargo run to launch.

## Linux System
Standard Linux build (usually 64-bit glibc).

Add the target

    rustup target add x86_64-unknown-linux-gnu

Build
    
    cargo build --release --target x86_64-unknown-linux-gnu

Install

    install -Dm755 target/release/CeeDee-Ripper "$CRAFT_PART_INSTALL/bin/CeeDee-Ripper"
      
    install -Dm644 resources/images/ceedee-ripper.png \
    
        "$CRAFT_PART_INSTALL/usr/share/icons/hicolor/256x256/apps/ceedee-ripper.png"
    
    install -Dm644 resources/ceedee-ripper.desktop \
     
        "$CRAFT_PART_INSTALL/usr/share/applications/ceedee-ripper.desktop"

## Windows System
There are two common targets for Windows.

Option A: Native Windows (MSVC) Best if building on Windows. Requires C++ build tools installed.

Add the target
    
    rustup target add x86_64-pc-windows-msvc

Build

    cargo build --release --target x86_64-pc-windows-msvc

Option B: Cross-Compile (GNU) Best if building for Windows from Linux. (Requires mingw-w64 package installed on Linux).

Add the target

    rustup target add x86_64-pc-windows-gnu

Build

    cargo build --release --target x86_64-pc-windows-gnu

# Device Target Designation (Runtime)

These commands tell CeeDee-Ripper which physical drive to use if the default detection fails.

Linux System
Using the export command in your shell (Bash/Zsh).

    export CD_DEVICE=/dev/sr0

Windows Systems
Using the environment variable setter appropriate for your terminal.

Powershell

    $env:CD_DEVICE="D:"

Command Prompt (cmd):

    REM Target the D: drive
    set CD_DEVICE=D:

## Snap Build and Release (Linux)

Use these commands from the project root to build and publish a snap package.

Prerequisites (Ubuntu/Debian):

    sudo apt update
    sudo apt install -y snapd snapcraft
    sudo snap install core

Build the snap:

    snapcraft clean
    snapcraft

Local install test (without publishing):

    sudo snap install --dangerous ./*.snap

Remove local test install:

    sudo snap remove ceedee-ripper

Upload and release to Snap Store:

    snapcraft login
    snapcraft upload --release=stable ./*.snap

Notes:

- Snapcraft builds release artifacts for publishing; debug builds are only for local development.
- Bump the `version` field in `snapcraft.yaml` before uploading a new release.
