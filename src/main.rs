mod cd_reader;
mod config;
mod ripper;
mod window;

use gtk4::{gio, glib, prelude::*};
use window::CeeDeeRipperWindow;

fn main() -> glib::ExitCode {
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
