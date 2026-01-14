# Build instructions to target Linux or Windows Systemes.

These commands set up the Rust toolchain to build for the specific operating system.

## Linux System
Standard Linux build (usually 64-bit glibc).

Add the target

    rustup target add x86_64-unknown-linux-gnu

Build
    
    cargo build --release --target x86_64-unknown-linux-gnu

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

# 2. Device Target Designation (Runtime)

These commands tell CeeDeeRipper which physical drive to use if the default detection fails.

Linux System
Using the export command in your shell (Bash/Zsh).

### Target the first SCSI optical drive

    export CD_DEVICE=/dev/sr0

Windows System
Using the environment variable setter appropriate for your terminal.

Powershell
# Target the D: drive

    $env:CD_DEVICE="D:"

Command Prompt (cmd):

    REM Target the D: drive
    set CD_DEVICE=D:
