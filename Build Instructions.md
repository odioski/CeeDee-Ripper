# Build instructions to target Linux or Windows Systems.

These commands set up the Rust toolchain to build for the specific operating system.

## Linux System
Standard Linux build (usually 64-bit glibc).

Add the target

    rustup target add x86_64-unknown-linux-gnu

Build
    
    cargo build --release --target x86_64-unknown-linux-gnu

## Windows System

There are two common targets for Windows.

### Option A: Native Windows (MSVC)
Best if building on Windows. Requires C++ build tools installed.

Add the target:

    rustup target add x86_64-pc-windows-msvc

Build:

    cargo build --release --target x86_64-pc-windows-msvc

#### Installing Dependencies with MSYS2 (PowerShell)

If you are using MSYS2 for dependencies (GTK4, GStreamer, etc.), you must install them using pacman. You will also need compilers and libc for building Rust projects. Run the following in PowerShell to launch MSYS2 and install all required packages:

    Start-Process -Wait -NoNewWindow msys2 -ArgumentList '-c "pacman -S --needed mingw-w64-x86_64-gcc mingw-w64-x86_64-libwinpthread-git mingw-w64-x86_64-gtk4 mingw-w64-x86_64-libadwaita mingw-w64-x86_64-glib2 mingw-w64-x86_64-gstreamer mingw-w64-x86_64-gst-plugins-base mingw-w64-x86_64-gst-plugins-good mingw-w64-x86_64-libdiscid mingw-w64-x86_64-cdparanoia mingw-w64-x86_64-flac mingw-w64-x86_64-lame mingw-w64-x86_64-vorbis-tools mingw-w64-x86_64-clang mingw-w64-x86_64-gcc mingw-w64-x86_64-crt-git mingw-w64-x86_64-headers-git mingw-w64-x86_64-libwinpthread-git"'

Alternatively, open the "MSYS2 MinGW 64-bit" terminal from your Start menu and run:

    pacman -S --needed  mingw-w64-x86_64-gcc mingw-w64-x86_64-libwinpthread-git mingw-w64-x86_64-gtk4 mingw-w64-x86_64-libadwaita mingw-w64-x86_64-glib2 mingw-w64-x86_64-gstreamer mingw-w64-x86_64-gst-plugins-base mingw-w64-x86_64-gst-plugins-good mingw-w64-x86_64-libdiscid mingw-w64-x86_64-cdparanoia mingw-w64-x86_64-flac mingw-w64-x86_64-lame mingw-w64-x86_64-vorbis-tools mingw-w64-x86_64-clang mingw-w64-x86_64-gcc mingw-w64-x86_64-crt-git mingw-w64-x86_64-headers-git mingw-w64-x86_64-libwinpthread-git

### Option B: Cross-Compile (GNU)
Best if building for Windows from Linux. (Requires mingw-w64 package installed on Linux).

Add the target:

    rustup target add x86_64-pc-windows-gnu

Build:

    cargo build --release --target x86_64-pc-windows-gnu

# Device Target Designation (Runtime)

These commands tell CeeDeeRipper which physical drive to use if the default detection fails.

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
