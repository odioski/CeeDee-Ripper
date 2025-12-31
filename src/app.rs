use crate::window::CeeDeeRipperWindow;
use gtk::prelude::*;
use gtk::glib;
use libadwaita::Application;

const APP_ID: &str = "org.ceedeeripper.CeeDeeRipper";

pub struct CeeDeeRipperApp {
    app: Application,
}

impl CeeDeeRipperApp {
    pub fn new() -> Self {
        let app = Application::builder()
            .application_id(APP_ID)
            .build();

        // Minor: ensure activation is wired once and no extra state is needed
        app.connect_activate(Self::on_activate);

        Self { app }
    }

    fn on_activate(app: &Application) {
        let window = CeeDeeRipperWindow::new(app);
        window.present();
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}