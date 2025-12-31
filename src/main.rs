mod app;
mod cd_reader;
mod config;
mod ripper;
mod window;

use app::CeeDeeRipperApp;
use gdk;
use gio;
use glib;
use gtk::prelude::*;

fn main() {
    env_logger::init();
    // Initialize GStreamer
    if let Err(e) = gstreamer::init() {
        eprintln!("Failed to initialize GStreamer: {}", e);
        return;
    }

    // Register resources
    let resources_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/ceedee_ripper.gresource"));
    gio::resources_register(
        &gio::Resource::from_data(&glib::Bytes::from_static(resources_bytes)).expect("Failed to load resources")
    );

    // Load custom CSS for styling
    let provider = gtk::CssProvider::new();
    provider.load_from_data(".translucent { opacity: 0.8; }");

    // Add the provider to the default display so it's available globally
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Create and run app
    let app = CeeDeeRipperApp::new();
    app.run();
}