use crate::cd_reader::{CdInfo, CdReader};
use crate::config::Config;
use crate::ripper::Ripper;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use gtk::{gio, glib};
use std::process::Command;
use glib::MainContext;

glib::wrapper! {
    pub struct CeeDeeRipperWindow(ObjectSubclass<imp::CeeDeeRipperWindow>)
        @extends libadwaita::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl CeeDeeRipperWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    fn setup_callbacks(&self) {
        let imp = self.imp();

        // Detect button
        let window_weak = self.downgrade();
        imp.detect_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.on_detect_clicked();
            }
        });

        // Choose folder button
        let window_weak = self.downgrade();
        imp.choose_folder_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.on_choose_folder_clicked();
            }
        });

        // Single toggle button: Start/Stop
        let window_weak = self.downgrade();
        imp.rip_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                let ripping = window.imp().state.borrow().is_ripping;
                if ripping {
                    window.on_stop_clicked();
                } else {
                    window.on_rip_clicked();
                }
            }
        });

        // Metadata lookup button
        let window_weak = self.downgrade();
        imp.metadata_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.on_metadata_lookup_clicked();
            }
        });

        // No separate Stop button in toggle mode

        // Selector changes are handled in constructed(); no duplicate wiring here

        // Eject button
        let window_weak = self.downgrade();
        imp.eject_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                window.on_eject_clicked();
            }
        });
    }

    fn on_detect_clicked(&self) {
        let imp = self.imp();

        // Apply current metadata source selection to config before detect
        let mut cfg = Config::load();
        let meta_sel = imp.metadata_selector.selected();
        cfg.metadata_source = match meta_sel {
            1 => "musicbrainz".to_string(),
            2 => "cddb".to_string(),
            _ => "none".to_string(),
        };
        let _ = cfg.save();

        match CdReader::detect() {
            Ok(cd_info) => {
                imp.state.borrow_mut().cd_info = Some(cd_info.clone());
                self.display_cd_info(&cd_info);
                imp.rip_button.set_sensitive(true);
            }
            Err(e) => {
                self.show_error(&format!("Failed to detect CD: {}", e));
            }
        }
    }

    // Metadata lookup handled by re-detect with current selection

    fn on_choose_folder_clicked(&self) {
        let dialog = gtk::FileDialog::new();
        dialog.set_title("Choose Output Folder");
        dialog.set_modal(true);

        let window_weak = self.downgrade();
        dialog.select_folder(Some(self), None::<&gio::Cancellable>, move |result| {
            if let Some(window) = window_weak.upgrade() {
                match result {
                    Ok(folder) => {
                        if let Some(path) = folder.path() {
                            window.imp().state.borrow_mut().output_dir = path;
                        } else {
                            window.show_error("Selected folder is not on a local filesystem.");
                        }
                    }
                    Err(err) => {
                        window.show_error(&format!("Folder selection failed: {}", err));
                    }
                }
            }
        });
    }

    fn on_rip_clicked(&self) {
        let imp = self.imp();
        let mut state = imp.state.borrow_mut();

        if let Some(cd_info) = state.cd_info.clone() {
            let mut config = Config::load();
            // Map UI selection to encoder string
            let selected = imp.format_selector.selected();
            let encoder = match selected {
                0 => "flac",
                1 => "mp3",
                2 => "wav",
                3 => "ogg",
                _ => "flac",
            };
            config.encoder = encoder.to_string();
            // Map metadata selector to config
            let meta_sel = imp.metadata_selector.selected();
            config.metadata_source = match meta_sel {
                1 => "musicbrainz".to_string(),
                2 => "cddb".to_string(),
                _ => "none".to_string(),
            };
            // Best-effort persist so user's choice sticks next time
            let _ = config.save();
            let ripper = std::sync::Arc::new(Ripper::new(config, state.output_dir.clone()));
            // Store ripper for cancellation
            state.ripper = Some(ripper.clone());

            state.is_ripping = true;
            let window_weak = self.downgrade();
            drop(state); // Release borrow before calling other methods on self

            MainContext::default().spawn_local(async move {
                if let Some(window) = window_weak.upgrade() {
                    match ripper.rip(&cd_info).await {
                        Ok(_) => window.on_rip_complete(),
                        Err(e) => {
                            window.show_error(&format!("Ripping failed: {}", e));
                            window.reset_ripping_state();
                        }
                    }
                }
            });

            // Update the UI to reflect the ripping state
            self.update_ui_for_ripping_state(true);
        } else {
            self.show_error("No CD detected.");
        }
    }

    fn on_metadata_lookup_clicked(&self) {
        let imp = self.imp();
        // Update config from selector
        let mut cfg = Config::load();
        let meta_sel = imp.metadata_selector.selected();
        cfg.metadata_source = match meta_sel { 1 => "musicbrainz".into(), 2 => "cddb".into(), _ => "none".into() };
        let _ = cfg.save();

        match CdReader::detect() {
            Ok(cd_info) => {
                imp.state.borrow_mut().cd_info = Some(cd_info.clone());
                self.display_cd_info(&cd_info);
                imp.rip_button.set_sensitive(true);
            }
            Err(e) => {
                self.show_error(&format!("Metadata lookup failed: {}", e));
            }
        }
    }

    fn on_stop_clicked(&self) {
        let imp = self.imp();
        {
            let mut st = imp.state.borrow_mut();
            if let Some(r) = st.ripper.take() {
                r.cancel();
            }
        }
        self.reset_ripping_state();
        // Clear per-track selections
        let mut child = imp.track_list.first_child();
        while let Some(row_w) = child {
            let next_row = row_w.next_sibling();
            if let Ok(row) = row_w.downcast::<gtk::ListBoxRow>() {
                if let Some(hb_w) = row.child() {
                    if let Ok(hbox) = hb_w.downcast::<gtk::Box>() {
                        let mut inner = hbox.first_child();
                        while let Some(w) = inner {
                            let next = w.next_sibling();
                            if let Ok(cb) = w.clone().downcast::<gtk::CheckButton>() {
                                cb.set_active(false);
                            }
                            inner = next;
                        }
                    }
                }
            }
            child = next_row;
        }
        // Note: Actual cancellation of ripping should be implemented in Ripper.
    }

    fn on_eject_clicked(&self) {
        match Command::new("eject").status() {
            Ok(status) if status.success() => self.show_success("Disc ejected."),
            Ok(_) => self.show_error("Eject command failed. Ensure 'eject' is installed and you have permission."),
            Err(err) => self.show_error(&format!("Could not run 'eject': {}", err)),
        }
    }

    fn reset_ripping_state(&self) {
        let imp = self.imp();
        {
            let mut st = imp.state.borrow_mut();
            st.is_ripping = false;
            st.ripper = None;
        }
        self.update_ui_for_ripping_state(false);
    }

    fn on_rip_complete(&self) {
        self.reset_ripping_state();
        self.show_success("CD ripped successfully!");
    }

    /// Centralized function to update UI elements based on ripping state.
    fn update_ui_for_ripping_state(&self, is_ripping: bool) {
        let imp = self.imp();

        // Toggle button appearance and progress bar visibility
        imp.progress_box.set_visible(is_ripping);
        if is_ripping {
            imp.rip_button.set_label("Stop");
            imp.rip_button.remove_css_class("suggested-action");
            imp.rip_button.add_css_class("destructive-action");
        } else {
            imp.rip_button.set_label("Start Ripping");
            imp.rip_button.remove_css_class("destructive-action");
            imp.rip_button.add_css_class("suggested-action");
            imp.progress_bar.set_fraction(0.0);
            imp.progress_label.set_label("");
        }

        // Disable other controls during ripping to prevent concurrent operations
        let sensitive = !is_ripping;
        imp.detect_button.set_sensitive(sensitive);
        imp.eject_button.set_sensitive(sensitive);
        imp.format_selector.set_sensitive(sensitive);
        imp.metadata_selector.set_sensitive(sensitive);
        imp.choose_folder_button.set_sensitive(sensitive);
        imp.track_list.set_sensitive(sensitive);

        // The rip button should be sensitive if a CD is detected (i.e., not ripping)
        imp.rip_button.set_sensitive(sensitive && imp.state.borrow().cd_info.is_some());

        // Metadata button sensitivity depends on the selector's choice, but only if not ripping
        let meta_selected = imp.metadata_selector.selected() != 0;
        imp.metadata_button.set_sensitive(sensitive && meta_selected);
    }

    fn display_cd_info(&self, cd_info: &CdInfo) {
        let imp = self.imp();

        imp.status_page.set_visible(false);
        imp.cd_info.set_visible(true);

        imp.cd_title.set_label(&cd_info.title);
        imp.cd_artist.set_label(&cd_info.artist);

        // Clear and populate track list
        while let Some(child) = imp.track_list.first_child() {
            imp.track_list.remove(&child);
        }

        for (i, track) in cd_info.tracks.iter().enumerate() {
            let row = self.create_track_row(i + 1, track);
            imp.track_list.append(&row);
        }
    }

    fn create_track_row(&self, track_num: usize, track_name: &str) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);

        let check = gtk::CheckButton::new();
        check.set_active(true);
        hbox.append(&check);

        let label = gtk::Label::new(Some(&format!("{}. {}", track_num, track_name)));
        label.set_halign(gtk::Align::Start);
        label.set_hexpand(true);
        hbox.append(&label);

        row.set_child(Some(&hbox));
        row
    }

    fn show_error(&self, message: &str) {
        let dialog = gtk::AlertDialog::builder()
            .message("Error")
            .detail(message)
            .default_button(0)
            .build();
        dialog.set_buttons(&["OK"]);
        dialog.choose(Some(self), None::<&gio::Cancellable>, |_| ());
    }

    fn show_success(&self, message: &str) {
        let dialog = gtk::AlertDialog::builder()
            .message("Success")
            .detail(message)
            .default_button(0)
            .build();
        dialog.set_buttons(&["OK"]);
        dialog.choose(Some(self), None::<&gio::Cancellable>, |_| ());
    }
}

mod imp {
    use super::*;
    use std::path::PathBuf;
    use std::cell::RefCell; // FIX: needed for state
    use gtk::subclass::widget::TemplateChild;
    use super::glib;
    use libadwaita::subclass::prelude::*;

    pub struct AppState {
        pub cd_info: Option<CdInfo>,
        pub output_dir: PathBuf,
        pub is_ripping: bool,
        pub ripper: Option<std::sync::Arc<Ripper>>,
    }

    impl Default for AppState {
        fn default() -> Self {
            Self {
                cd_info: None,
                output_dir: PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                    .join("Music"),
                is_ripping: false,
                ripper: None,
            }
        }
    }

    #[derive(gtk::CompositeTemplate, Default)]
    #[template(resource = "/org/ceedeeripper/CeeDeeRipper/ui/window.ui")]
    pub struct CeeDeeRipperWindow {
        #[template_child]
        pub detect_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub rip_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub eject_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub format_selector: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub metadata_selector: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub metadata_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub choose_folder_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub cd_info: TemplateChild<gtk::Box>,
        #[template_child]
        pub cd_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub cd_artist: TemplateChild<gtk::Label>,
        #[template_child]
        pub status_page: TemplateChild<libadwaita::StatusPage>,
        #[template_child]
        pub progress_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub progress_label: TemplateChild<gtk::Label>,

        pub state: RefCell<AppState>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CeeDeeRipperWindow {
        const NAME: &'static str = "CeeDeeRipperWindow";
        type Type = super::CeeDeeRipperWindow;
        type ParentType = libadwaita::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CeeDeeRipperWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_callbacks();

            // FIX: initialize UI cascade safely
            self.rip_button.set_sensitive(false);
            self.eject_button.set_sensitive(true);
            self.progress_box.set_visible(false);
            self.progress_bar.set_fraction(0.0);
            self.progress_label.set_label("");
            self.status_page.set_visible(true);
            self.cd_info.set_visible(false);

            // Apply transparency to the track list
            self.track_list.add_css_class("translucent");

            // Provide a model for the DropDown to avoid template cascade issues
            let formats = gtk::StringList::new(&["FLAC", "MP3", "WAV", "OGG"]);
            self.format_selector.set_model(Some(&formats));
            self.format_selector.set_selected(0);

            // Initialize metadata selector from saved config
            let cfg = Config::load();
            let meta_index = match cfg.metadata_source.as_str() {
                "musicbrainz" => 1,
                "cddb" => 2,
                _ => 0,
            };
            self.metadata_selector.set_selected(meta_index);
            // Enable lookup button when a source is selected
            self.metadata_button.set_sensitive(meta_index != 0);

            // React to metadata selector changes to persist and enable button
            let selector = self.metadata_selector.clone();
            let btn = self.metadata_button.clone();
            selector.connect_selected_notify(move |sel| {
                let idx = sel.selected();
                btn.set_sensitive(idx != 0);
                let mut cfg = Config::load();
                cfg.metadata_source = match idx {
                    1 => "musicbrainz".to_string(),
                    2 => "cddb".to_string(),
                    _ => "none".to_string(),
                };
                let _ = cfg.save();
            });
            // Sensitivity already set above based on initial selection

            // Auto-detect CD on launch
            let obj_clone = obj.clone();
            glib::idle_add_local(move || {
                obj_clone.on_detect_clicked();
                glib::ControlFlow::Break
            });
        }
    }

    impl WidgetImpl for CeeDeeRipperWindow {}
    impl WindowImpl for CeeDeeRipperWindow {}
    impl ApplicationWindowImpl for CeeDeeRipperWindow {}
    impl AdwApplicationWindowImpl for CeeDeeRipperWindow {}
}