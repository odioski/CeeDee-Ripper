mod app;
mod cd_reader;
mod config;
mod ripper;
mod window;

use app::CeeDeeRipperApp;

fn main() {
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

    // Create and run app
    let app = CeeDeeRipperApp::new();
    app.run();
}