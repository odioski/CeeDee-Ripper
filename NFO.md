# CeeDee Ripper – Rust Architecture Breakdown

## Overview

CeeDee Ripper is a GTK4/libadwaita CD ripping application written in Rust. It extracts audio tracks from CDs, fetches metadata from online databases, and encodes to multiple formats.

---

## Module Structure

```
src/
├── main.rs       # Entry point: initializes GStreamer, loads resources, starts app
├── app.rs        # Application wrapper around libadwaita::Application
├── window.rs     # Main window UI logic + GTK subclass implementation
├── cd_reader.rs  # CD detection, TOC reading, metadata fetching (MusicBrainz/CDDB)
├── config.rs     # Configuration persistence (~/.config/ceedee-ripper/config.toml)
└── ripper.rs     # Track extraction (GStreamer/cdparanoia) + encoding (FLAC/MP3/OGG/WAV)
```

---

## How The Pieces Fit

### 1. `main.rs` – The Entry Point

```rust
fn main() {
    gstreamer::init()?;                     // Initialize GStreamer for audio pipelines
    gio::resources_register(...);           // Embed compiled UI resources at build-time
    let app = CeeDeeRipperApp::new();
    app.run();
}
```

**Key Rust Concepts:**
- **`include_bytes!`** – Embeds the `.gresource` binary at compile time
- **`glib::Bytes::from_static`** – Zero-copy static byte slice wrapper for GLib
- Module declarations (`mod app; mod window;`) wire up the crate

---

### 2. `app.rs` – Application Lifecycle

```rust
pub struct CeeDeeRipperApp {
    app: libadwaita::Application,
}
```

**Purpose:** Wraps the GTK application, handles the `activate` signal to create the main window.

**Key Rust Concepts:**
- **Struct wrapping** – Encapsulates GTK types in a Rust struct
- **`connect_activate`** – GTK signal connection using a callback function
- **Builder pattern** – `Application::builder().application_id(APP_ID).build()`

---

### 3. `window.rs` – UI Logic & GTK Subclassing

This is the most complex file because it implements GTK's subclassing pattern in Rust.

#### Outer Wrapper (Public API)

```rust
glib::wrapper! {
    pub struct CeeDeeRipperWindow(ObjectSubclass<imp::CeeDeeRipperWindow>)
        @extends libadwaita::ApplicationWindow, gtk::ApplicationWindow, ...
        @implements gio::ActionGroup, gio::ActionMap, ...
}
```

**What This Macro Does:**
- Creates a safe Rust wrapper around the internal `imp::CeeDeeRipperWindow`
- Declares the GObject inheritance chain (`@extends`)
- Declares implemented interfaces (`@implements`)

#### Inner Implementation (`mod imp`)

```rust
#[derive(gtk::CompositeTemplate, Default)]
#[template(resource = "/org/ceedeeripper/CeeDeeRipper/ui/window.ui")]
pub struct CeeDeeRipperWindow {
    #[template_child]
    pub detect_button: TemplateChild<gtk::Button>,
    // ... more template children
    pub state: RefCell<AppState>,
}
```

**Key Rust Concepts:**
- **`#[derive(CompositeTemplate)]`** – Proc macro that binds UI XML to Rust fields
- **`#[template_child]`** – Marks fields that come from the `.ui` template
- **`RefCell<AppState>`** – Interior mutability for GTK's single-threaded model
- **`ObjectSubclass` trait impl** – Required by GObject for subclassing

#### The GObject Trait Chain

```rust
impl ObjectSubclass for CeeDeeRipperWindow { ... }  // Type registration
impl ObjectImpl for CeeDeeRipperWindow { ... }      // Object lifecycle (constructed)
impl WidgetImpl for CeeDeeRipperWindow { ... }      // Widget overrides
impl WindowImpl for CeeDeeRipperWindow { ... }      // Window overrides  
impl ApplicationWindowImpl for CeeDeeRipperWindow { ... }
impl AdwApplicationWindowImpl for CeeDeeRipperWindow { ... }
```

Each trait corresponds to a GTK class in the inheritance chain.

#### Signal Callbacks Pattern

```rust
let window_weak = self.downgrade();  // Weak reference to avoid ref cycles
imp.detect_button.connect_clicked(move |_| {
    if let Some(window) = window_weak.upgrade() {
        window.on_detect_clicked();
    }
});
```

**Why `downgrade()`?** 
GTK buttons hold references to their callbacks. If the callback held a strong reference to the window, you'd have a reference cycle → memory leak. `downgrade()` creates a weak reference that can be upgraded when needed.

---

### 4. `cd_reader.rs` – CD Detection & Metadata

#### Core Structure

```rust
pub struct CdInfo {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub disc_id: String,
}

pub struct CdReader;  // Unit struct – no state, just associated functions
```

#### Detection Flow

```
CdReader::detect()
    ├── get_active_device_path()     # Check env, config, fallbacks
    ├── read_toc_raw(&device)        # Direct ioctl to /dev/sr0
    │   └── [fails] → fallback_track_count()  # cdparanoia -Q
    └── fetch metadata based on config:
        ├── "musicbrainz" → fetch_musicbrainz_metadata()
        ├── "cddb"        → fetch_cddb_metadata()
        └── "none"        → create_default_info_with_count()
```

#### Raw ioctl – Talking Directly to the Kernel

```rust
fn read_toc_raw(device: &str) -> Result<usize, io::Error> {
    const CDROMREADTOCHDR: libc::c_ulong = 0x5305;  // From linux/cdrom.h
    
    #[repr(C)]  // C-compatible memory layout
    struct CdromTocHdr { 
        cdth_trk0: libc::c_uchar, 
        cdth_trk1: libc::c_uchar 
    }

    let f = File::open(device)?;
    let fd = f.as_raw_fd();  // Get raw file descriptor
    
    unsafe { 
        libc::ioctl(fd, CDROMREADTOCHDR, &mut hdr as *mut _) 
    };
}
```

**Key Rust Concepts:**
- **`#[repr(C)]`** – Guarantees struct layout matches C ABI
- **`unsafe { ioctl(...) }`** – FFI calls require unsafe block
- **`as_raw_fd()`** – Converts Rust File to raw integer fd
- **Raw pointers** – `&mut hdr as *mut _` creates a C-style pointer

#### MusicBrainz Metadata Fetch

```rust
fn fetch_musicbrainz_metadata(_device: &str) -> Option<CdInfo> {
    let disc = DiscId::read(None).ok()?;           // libdiscid FFI binding
    let url = format!("...{}", disc.id());
    let resp = ureq::get(&url).call().ok()?;       // HTTP GET
    let json: serde_json::Value = resp.into_json().ok()?;
    // Parse JSON...
}
```

**Key Rust Concepts:**
- **`?` operator on `Option`** – Early return `None` if any step fails
- **`ureq`** – Blocking HTTP client (works in sync context)
- **`serde_json::Value`** – Dynamic JSON (when schema isn't fixed)

---

### 5. `ripper.rs` – Audio Extraction & Encoding

#### Structure with Concurrency Primitives

```rust
pub struct Ripper {
    config: Config,
    output_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,           // Thread-safe cancellation
    current_child: Arc<Mutex<Option<Child>>>,  // Currently running process
}
```

**Key Rust Concepts:**
- **`Arc<T>`** – Atomic Reference Counted pointer, enables shared ownership across threads
- **`AtomicBool`** – Lock-free boolean for signaling between threads
- **`Mutex<Option<Child>>`** – Protects the spawned subprocess handle

#### Async Ripping with GStreamer

```rust
pub async fn rip(&self, cd_info: &CdInfo) -> Result<(), Box<dyn Error>> {
    for (i, track_name) in cd_info.tracks.iter().enumerate() {
        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err("Ripping cancelled".into());
        }
        self.rip_track(track_num, track_name, &album_dir).await?;
    }
}
```

**Key Rust Concepts:**
- **`async fn`** – Returns a `Future`, doesn't block
- **`Ordering::SeqCst`** – Memory ordering for atomic operations (strongest guarantee)
- **`.await`** – Suspends until the future completes

#### GStreamer Pipeline

```rust
fn rip_track_via_gstreamer(&self, track_num: usize, wav_file: &PathBuf) -> Result<...> {
    let pipe_str = format!(
        "cdparanoia device={} track={} ! wavenc ! filesink location={}",
        self.config.device, track_num, wav_file.display()
    );
    
    let pipeline = gst::parse::launch(&pipe_str)?
        .dynamic_cast::<gst::Pipeline>()?;
    
    pipeline.set_state(gst::State::Playing)?;
    
    // Event loop
    loop {
        match bus.timed_pop(gst::ClockTime::from_mseconds(250)) {
            Some(msg) => match msg.view() {
                gst::MessageView::Eos(_) => break,
                gst::MessageView::Error(err) => return Err(...),
                _ => {}
            },
            None => break,
        }
    }
}
```

**Pipeline Syntax:** `source ! filter ! sink` (GStreamer's DSL)

**Key Rust Concepts:**
- **`dynamic_cast::<T>()`** – GObject-style type casting (can fail)
- **Pattern matching on message types** – Rust's `match` handles GStreamer events
- **Timed polling** – Non-blocking wait allows cancellation checks

#### Encoding via External Tools

```rust
fn encode_flac(&self, input: &PathBuf, ...) -> Result<PathBuf, Box<dyn Error>> {
    let status = Command::new("flac")
        .arg("-8")
        .arg(input)
        .arg("-o")
        .arg(&output)
        .status()?;

    if !status.success() {
        return Err("FLAC encoding failed".into());
    }
    Ok(output)
}
```

Encoders: `flac`, `lame` (MP3), `oggenc` (OGG)

---

### 6. `config.rs` – Persistent Configuration

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub encoder: String,
    pub bitrate: String,
    pub device: String,
    pub metadata_source: String,
}

impl Config {
    pub fn load() -> Self {
        let path = dirs::config_dir().join("ceedee-ripper/config.toml");
        toml::from_str(&fs::read_to_string(&path)?)?
    }
    
    pub fn save(&self) -> io::Result<()> {
        fs::write(path, toml::to_string_pretty(self)?)
    }
}
```

**Key Rust Concepts:**
- **`#[derive(Serialize, Deserialize)]`** – Serde macros for TOML serialization
- **`#[serde(default)]`** – Use `Default::default()` for missing fields
- **`dirs::config_dir()`** – XDG-compliant config path (`~/.config/`)

---

### 7. `build.rs` – Build-Time Resource Compilation

```rust
fn main() {
    println!("cargo:rerun-if-changed=resources");
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "ceedee_ripper.gresource",
    );
}
```

**What Happens:**
1. Cargo runs `build.rs` before compiling the crate
2. `glib-compile-resources` bundles UI XML into a binary blob
3. Output goes to `$OUT_DIR/ceedee_ripper.gresource`
4. `main.rs` embeds this with `include_bytes!(concat!(env!("OUT_DIR"), "/..."))`

---

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         User Interaction                            │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  window.rs (CeeDeeRipperWindow)                                     │
│  ├── detect_button.clicked → on_detect_clicked()                    │
│  ├── rip_button.clicked    → on_rip_clicked() / on_stop_clicked()  │
│  └── eject_button.clicked  → on_eject_clicked()                     │
└─────────────────────────────────────────────────────────────────────┘
                    │                               │
                    ▼                               ▼
    ┌───────────────────────────┐     ┌───────────────────────────┐
    │  cd_reader.rs             │     │  ripper.rs                │
    │  ├── detect() → CdInfo    │     │  ├── rip() [async]        │
    │  ├── read_toc_raw()       │     │  ├── rip_track_via_gst()  │
    │  ├── fetch_musicbrainz()  │     │  ├── encode_flac/mp3/ogg()│
    │  └── fetch_cddb()         │     │  └── cancel()             │
    └───────────────────────────┘     └───────────────────────────┘
                    │                               │
                    ▼                               ▼
    ┌───────────────────────────────────────────────────────────────┐
    │  config.rs (Config)                                           │
    │  └── ~/.config/ceedee-ripper/config.toml                      │
    └───────────────────────────────────────────────────────────────┘
```

---

## External Dependencies

| Crate | Purpose |
|-------|---------|
| `gtk4` / `libadwaita` | GTK4 UI framework + GNOME design |
| `glib` / `gio` | GLib event loop, resources, async I/O |
| `gstreamer` | Audio pipeline (cdparanoia element, WAV encoding) |
| `discid` | Rust bindings to `libdiscid` (MusicBrainz disc ID calculation) |
| `ureq` | Blocking HTTP client for metadata APIs |
| `serde` / `toml` | Config serialization |
| `libc` | Raw ioctl syscalls |
| `dirs` | XDG directory paths |

---

## System Tools Required

- `cdparanoia` – CD audio extraction (CLI fallback)
- `flac` – FLAC encoder
- `lame` – MP3 encoder  
- `oggenc` – OGG Vorbis encoder (from `vorbis-tools`)
- `eject` – Disc ejection
- `cd-discid` – Fallback disc ID calculation

---

## Rust Idioms Used

| Pattern | Where Used |
|---------|-----------|
| Interior Mutability (`RefCell`) | `AppState` in window.rs |
| Thread-Safe Shared State (`Arc<AtomicBool>`) | Ripper cancellation |
| Error Propagation (`?` operator) | Throughout all modules |
| Builder Pattern | GTK object construction |
| Option Chaining (`.and_then()`, `.or_else()`) | JSON parsing in cd_reader.rs |
| Weak References (`downgrade/upgrade`) | GTK signal callbacks |
| FFI with `unsafe` | ioctl in cd_reader.rs |
| Async/Await | Ripping in ripper.rs |
| Derive Macros | Serde, GObject |

---

## Build & Run

```bash
# Install dependencies (Debian/Ubuntu)
./scripts/install-deps.sh

# Build
cargo build

# Run
cargo run
```

---

*Generated from source analysis – CeeDee Ripper v0.1.0*
