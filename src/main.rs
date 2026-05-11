mod cd_reader;
mod config;
#[cfg(feature = "egui-ui")]
mod egui_app;
mod ripper;
#[cfg(feature = "gtk-ui")]
mod window;

use config::Config;
use std::fmt;

#[cfg(feature = "gtk-ui")]
use std::{env, path::PathBuf};

#[cfg(feature = "gtk-ui")]
use gtk4::{gio, glib, prelude::*};
#[cfg(feature = "gtk-ui")]
use libadwaita::{ColorScheme, StyleManager};
#[cfg(feature = "gtk-ui")]
use window::CeeDeeRipperWindow;

#[cfg(not(any(feature = "gtk-ui", feature = "egui-ui")))]
compile_error!("Enable at least one UI feature: gtk-ui or egui-ui.");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiBackend {
    Egui,
    Gtk,
}

impl UiBackend {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "egui" | "egui-ui" => Some(Self::Egui),
            "gtk" | "gtk4" | "gtk-ui" => Some(Self::Gtk),
            _ => None,
        }
    }

    fn is_compiled(self) -> bool {
        match self {
            Self::Egui => cfg!(feature = "egui-ui"),
            Self::Gtk => cfg!(feature = "gtk-ui"),
        }
    }

    fn as_config_value(self) -> &'static str {
        match self {
            Self::Egui => "egui",
            Self::Gtk => "gtk",
        }
    }
}

impl fmt::Display for UiBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_config_value())
    }
}

fn default_compiled_ui_backend() -> UiBackend {
    #[cfg(feature = "egui-ui")]
    {
        UiBackend::Egui
    }
    #[cfg(all(not(feature = "egui-ui"), feature = "gtk-ui"))]
    {
        UiBackend::Gtk
    }
}

fn print_compiled_ui_backend() {
    let mut backends = Vec::new();
    #[cfg(feature = "egui-ui")]
    backends.push("egui-ui");
    #[cfg(feature = "gtk-ui")]
    backends.push("gtk-ui");

    println!("Compiled UI backend(s): {}", backends.join(", "));
}

fn print_usage() {
    println!(
        "Usage: ceedee-ripper [--ui egui|gtk]\n       ceedee-ripper [--features egui-ui|gtk-ui]\n\nRuntime selectors choose the UI for this run and save it for future launches when available.\nWith cargo run, pass runtime selectors after --, for example:\n       cargo run --features \"gtk-ui egui-ui\" -- --features gtk-ui"
    );
}

fn selected_ui_backend() -> Result<Option<UiBackend>, String> {
    let mut args = std::env::args().skip(1);
    let mut selected = None;

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            print_usage();
            std::process::exit(0);
        }

        let value = if arg == "--ui" || arg == "--features" {
            args.next()
                .ok_or_else(|| format!("{arg} requires one of: egui, gtk, egui-ui, gtk-ui"))?
        } else if let Some(value) = arg.strip_prefix("--ui=") {
            value.to_string()
        } else if let Some(value) = arg.strip_prefix("--features=") {
            value.to_string()
        } else {
            return Err(format!("Unrecognized argument: {arg}"));
        };

        let backend = UiBackend::parse(&value).ok_or_else(|| {
            format!("Unsupported UI backend '{value}'. Use egui, gtk, egui-ui, or gtk-ui.")
        })?;
        if !backend.is_compiled() {
            return Err(format!(
                "UI backend '{backend}' is not compiled into this binary. Rebuild with --features {}.",
                match backend {
                    UiBackend::Egui => "egui-ui",
                    UiBackend::Gtk => "gtk-ui",
                }
            ));
        }
        selected = Some(backend);
    }

    Ok(selected)
}

fn resolve_ui_backend() -> Result<UiBackend, String> {
    if let Some(backend) = selected_ui_backend()? {
        let mut config = Config::load();
        config.ui_backend = backend.as_config_value().to_string();
        let _ = config.save();
        return Ok(backend);
    }

    let config = Config::load();
    Ok(UiBackend::parse(&config.ui_backend)
        .filter(|backend| backend.is_compiled())
        .unwrap_or_else(default_compiled_ui_backend))
}

#[cfg(feature = "gtk-ui")]
fn has_graphical_display() -> bool {
    if env::var_os("DISPLAY").is_some_and(|display| !display.is_empty()) {
        return true;
    }

    let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") else {
        return false;
    };

    let wayland_display = env::var_os("WAYLAND_DISPLAY")
        .filter(|display| !display.is_empty())
        .unwrap_or_else(|| "wayland-0".into());

    let wayland_socket = {
        let path = PathBuf::from(&wayland_display);
        if path.is_absolute() {
            path
        } else {
            PathBuf::from(runtime_dir).join(path)
        }
    };

    wayland_socket.exists()
}

#[cfg(feature = "gtk-ui")]
fn run_gtk_ui() -> glib::ExitCode {
    print_compiled_ui_backend();

    if !has_graphical_display() {
        eprintln!(
            "CeeDee Ripper requires a graphical X11 or Wayland session. Start it from a desktop session with access to a display."
        );
        return glib::ExitCode::FAILURE;
    }

    if let Err(e) = gtk4::init() {
        eprintln!("Failed to initialize GTK: {}", e);
        return glib::ExitCode::FAILURE;
    }

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(false);
    }

    if let Err(e) = libadwaita::init() {
        eprintln!("Failed to initialize Libadwaita: {}", e);
        return glib::ExitCode::FAILURE;
    }

    StyleManager::default().set_color_scheme(ColorScheme::Default);

    // Initialize GStreamer
    if let Err(e) = gstreamer::init() {
        eprintln!("Failed to initialize GStreamer: {}", e);
        return glib::ExitCode::FAILURE;
    }

    // Create a new application
    let app = libadwaita::Application::builder()
        .application_id("snap.ceedee-ripper.ceedee-ripper")
        .build();

    // Connect to "startup" signal to perform one-time initialization
    app.connect_startup(|_| {
        // Register resources
        let resources_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/ceedee_ripper.gresource"));
        let resource = gio::Resource::from_data(&glib::Bytes::from_static(resources_bytes))
            .expect("Failed to load resources");
        gio::resources_register(&resource);

        // Load CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_resource("/org/ceedeeripper/CeeDee-Ripper/style.css");
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    // Connect to "activate" signal to create and show the main window
    app.connect_activate(|app| {
        CeeDeeRipperWindow::new(app).present();
    });

    // Runtime UI selectors are parsed by CeeDee Ripper before GTK starts.
    // Run GTK with a clean argv so GApplication does not reject app-level
    // options such as `--features gtk-ui`.
    app.run_with_args(&["ceedee-ripper"])
}

#[cfg(feature = "egui-ui")]
fn run_egui_ui() -> eframe::Result<()> {
    print_compiled_ui_backend();

    if let Err(e) = gstreamer::init() {
        eprintln!("Failed to initialize GStreamer: {}", e);
        std::process::exit(1);
    }

    egui_app::run()
}

#[cfg(all(feature = "gtk-ui", feature = "egui-ui"))]
fn main() -> glib::ExitCode {
    match resolve_ui_backend() {
        Ok(UiBackend::Gtk) => run_gtk_ui(),
        Ok(UiBackend::Egui) => match run_egui_ui() {
            Ok(()) => glib::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Failed to run egui UI: {err}");
                glib::ExitCode::FAILURE
            }
        },
        Err(err) => {
            eprintln!("{err}");
            print_usage();
            glib::ExitCode::FAILURE
        }
    }
}

#[cfg(all(feature = "gtk-ui", not(feature = "egui-ui")))]
fn main() -> glib::ExitCode {
    match resolve_ui_backend() {
        Ok(UiBackend::Gtk) => run_gtk_ui(),
        Ok(UiBackend::Egui) => unreachable!("egui-ui is not compiled"),
        Err(err) => {
            eprintln!("{err}");
            print_usage();
            glib::ExitCode::FAILURE
        }
    }
}

#[cfg(all(feature = "egui-ui", not(feature = "gtk-ui")))]
fn main() -> eframe::Result<()> {
    match resolve_ui_backend() {
        Ok(UiBackend::Egui) => run_egui_ui(),
        Ok(UiBackend::Gtk) => unreachable!("gtk-ui is not compiled"),
        Err(err) => {
            eprintln!("{err}");
            print_usage();
            std::process::exit(1);
        }
    }
}
