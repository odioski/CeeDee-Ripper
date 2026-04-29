mod cd_reader;
mod config;
mod ripper;
mod window;

use std::{env, path::PathBuf};

use gtk4::{gio, glib, prelude::*};
use libadwaita::{ColorScheme, StyleManager};
use window::CeeDeeRipperWindow;

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

fn main() -> glib::ExitCode {
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

    // Run the application
    app.run()
}
