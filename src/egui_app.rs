use crate::cd_reader::{CdInfo, CdReader};
use crate::config::Config;
use crate::ripper::{RipMessage, Ripper};
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Rip,
    Settings,
}

struct CeeDeeRipperEguiApp {
    active_tab: AppTab,
    cd_info: Option<CdInfo>,
    metadata_source_index: usize,
    format_index: usize,
    album_art_size_index: usize,
    album_art_download_index: usize,
    output_dir: String,
    selected_tracks: Vec<bool>,
    progress_fraction: f64,
    progress_label: String,
    is_ripping: bool,
    ripper: Option<Arc<Ripper>>,
    rip_receiver: Option<UnboundedReceiver<RipMessage>>,
    album_cover_texture: Option<egui::TextureHandle>,
    album_cover_source_url: Option<String>,
    album_cover_error: Option<String>,
    next_auto_detect_at: Instant,
    status: String,
}

impl Default for CeeDeeRipperEguiApp {
    // ## Builds the initial egui app state from saved config and defaults. 1
    fn default() -> Self {
        let cfg = Config::load();
        let metadata_source_index = match cfg.metadata_source.as_str() {
            "musicbrainz" => 1,
            "cddb" => 2,
            _ => 0,
        };
        let format_index = match cfg.encoder.as_str() {
            "mp3" => 1,
            "wav" => 2,
            "ogg" => 3,
            _ => 0,
        };
        let album_art_size_index = match cfg.album_art_size_preference.as_str() {
            "small" => 1,
            "large" => 2,
            "original" => 3,
            _ => 0,
        };
        let album_art_download_index = match cfg.album_art_download_behavior.as_str() {
            "save-with-rip" => 1,
            _ => 0,
        };
        let output_dir = dirs::audio_dir()
            .or_else(|| dirs::home_dir().map(|home| home.join("Music")))
            .unwrap_or_else(|| PathBuf::from("Music"))
            .to_string_lossy()
            .to_string();

        Self {
            active_tab: AppTab::Rip,
            cd_info: None,
            metadata_source_index,
            format_index,
            album_art_size_index,
            album_art_download_index,
            output_dir,
            selected_tracks: Vec::new(),
            progress_fraction: 0.0,
            progress_label: String::new(),
            is_ripping: false,
            ripper: None,
            rip_receiver: None,
            album_cover_texture: None,
            album_cover_source_url: None,
            album_cover_error: None,
            next_auto_detect_at: Instant::now(),
            status: String::new(),
        }
    }
}

// ## Launches the native eframe window for the egui UI. 2
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([980.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CeeDee Ripper",
        options,
        Box::new(|_| Ok(Box::<CeeDeeRipperEguiApp>::default())),
    )
}

impl eframe::App for CeeDeeRipperEguiApp {
    // ## Matches the egui clear color to the active visual background. 3
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.window_fill().to_normalized_gamma_f32()
    }

    // ## Renders the top-level egui app frame and active tab. 4
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_ripper_messages();

        if self.is_ripping {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        ui.spacing_mut().item_spacing.y = 8.0;

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, AppTab::Rip, "Rip");
            ui.selectable_value(&mut self.active_tab, AppTab::Settings, "Settings");
        });
        ui.separator();

        match self.active_tab {
            AppTab::Rip => self.render_rip_tab(ui),
            AppTab::Settings => self.render_settings_tab(ui),
        }
    }
}

impl CeeDeeRipperEguiApp {
    const AUTO_DETECT_RETRY_INTERVAL: Duration = Duration::from_secs(2);

    // ## Renders the ripping tab controls, disc metadata, track list, and album art. 5
    fn render_rip_tab(&mut self, ui: &mut egui::Ui) {
        self.maybe_auto_detect_cd(ui.ctx());

        ui.heading("CeeDee Ripper");
        ui.label("egui interface");
        ui.label(format!("Device: {}", CdReader::active_device_path()));

        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("format")
                .selected_text(match self.format_index {
                    1 => "MP3",
                    2 => "WAV",
                    3 => "OGG",
                    _ => "FLAC",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.format_index, 0, "FLAC");
                    ui.selectable_value(&mut self.format_index, 1, "MP3");
                    ui.selectable_value(&mut self.format_index, 2, "WAV");
                    ui.selectable_value(&mut self.format_index, 3, "OGG");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Metadata:");
            egui::ComboBox::from_id_salt("metadata_source")
                .selected_text(match self.metadata_source_index {
                    1 => "MusicBrainz",
                    2 => "CDDB",
                    _ => "None",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.metadata_source_index, 0, "None");
                    ui.selectable_value(&mut self.metadata_source_index, 1, "MusicBrainz");
                    ui.selectable_value(&mut self.metadata_source_index, 2, "CDDB");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Output:");
            ui.text_edit_singleline(&mut self.output_dir);
        });

        ui.horizontal(|ui| {
            let can_detect = !self.is_ripping;
            if ui
                .add_enabled(can_detect, egui::Button::new("Detect CD"))
                .clicked()
            {
                self.detect_cd();
            }

            if ui
                .add_enabled(!self.is_ripping, egui::Button::new("Save Settings"))
                .clicked()
            {
                self.save_ui_settings();
            }

            if ui
                .add_enabled(!self.is_ripping, egui::Button::new("Eject"))
                .clicked()
            {
                self.eject_disc();
            }

            let rip_label = if self.is_ripping {
                "Stop Ripping"
            } else {
                "Start Ripping"
            };

            if ui.button(rip_label).clicked() {
                if self.is_ripping {
                    self.stop_ripping();
                } else {
                    self.start_ripping();
                }
            }
        });

        if self.is_ripping {
            ui.add(egui::ProgressBar::new(self.progress_fraction as f32).show_percentage());
            if !self.progress_label.is_empty() {
                ui.label(&self.progress_label);
            }
        }

        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }

        let cover_url = self.current_album_cover_url();
        self.ensure_album_cover(ui.ctx(), cover_url.as_deref());

        if let Some(cd_info) = &self.cd_info {
            let tracks = cd_info.tracks.clone();
            let track_count = tracks.len();
            ui.separator();
            ui.label(format!("Album: {}", cd_info.title));
            ui.label(format!("Artist: {}", cd_info.artist));
            ui.label(format!("Disc ID: {}", cd_info.disc_id));

            let content_size = egui::vec2(ui.available_width(), ui.available_height().max(1.0));
            ui.allocate_ui_with_layout(
                content_size,
                egui::Layout::left_to_right(egui::Align::TOP),
                |ui| {
                    let gap = 14.0;
                    let total_width = ui.available_width();
                    let content_height = ui.available_height().max(1.0);
                    let track_column_width =
                        self.current_track_list_width_for_layout(track_count, total_width);
                    let cover_column_width = (total_width - track_column_width - gap).max(1.0);
                    let cover_preview_size = self.current_cover_preview_size_for_layout(
                        track_count,
                        content_height,
                        cover_column_width,
                    );

                    ui.allocate_ui_with_layout(
                        egui::vec2(track_column_width, content_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let track_font_size = Self::track_list_font_size(
                                ui,
                                track_count,
                                track_column_width,
                                content_height,
                            );
                            ui.spacing_mut().item_spacing.y = track_font_size * 0.42;
                            ui.set_width(track_column_width);

                            egui::ScrollArea::vertical()
                                .id_salt("tracks_scroll")
                                .max_height(content_height)
                                .show(ui, |ui| {
                                    ui.set_width(track_column_width);
                                    for (index, track) in tracks.iter().enumerate() {
                                        let track_text = egui::RichText::new(format!(
                                            "{:02}. {}",
                                            index + 1,
                                            track
                                        ))
                                        .size(track_font_size);
                                        let selected = self.selected_tracks.get_mut(index);
                                        if let Some(is_selected) = selected {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.checkbox(is_selected, "");
                                                ui.add(egui::Label::new(track_text).wrap());
                                            });
                                        } else {
                                            ui.add(egui::Label::new(track_text).wrap());
                                        }
                                    }
                                });
                        },
                    );

                    ui.add_space(gap);

                    ui.allocate_ui_with_layout(
                        egui::vec2(cover_column_width, content_height),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            if let Some(texture) = &self.album_cover_texture {
                                ui.add(
                                    egui::Image::new(texture).fit_to_exact_size(cover_preview_size),
                                );
                            } else if let Some(err) = &self.album_cover_error {
                                ui.label(err);
                            } else {
                                ui.label("No album cover available");
                            }
                        },
                    );
                },
            );
        }
    }

    // ## Renders settings for metadata source and album art preferences. 6
    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.label("Album art preferences");

        let previous_size_index = self.album_art_size_index;

        ui.horizontal(|ui| {
            ui.label("Preferred size:");
            egui::ComboBox::from_id_salt("album_art_size")
                .selected_text(match self.album_art_size_index {
                    1 => "Small",
                    2 => "Large",
                    3 => "Original",
                    _ => "Auto",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.album_art_size_index, 0, "Auto");
                    ui.selectable_value(&mut self.album_art_size_index, 1, "Small");
                    ui.selectable_value(&mut self.album_art_size_index, 2, "Large");
                    ui.selectable_value(&mut self.album_art_size_index, 3, "Original");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Download behavior:");
            egui::ComboBox::from_id_salt("album_art_download")
                .selected_text(match self.album_art_download_index {
                    1 => "Save with rip",
                    _ => "Preview only",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.album_art_download_index, 0, "Preview only");
                    ui.selectable_value(&mut self.album_art_download_index, 1, "Save with rip");
                });
        });

        if previous_size_index != self.album_art_size_index {
            self.reset_album_cover_state();
        }

        ui.separator();
        if let Some(cd_info) = &self.cd_info {
            if cd_info.album_art_options.is_empty() {
                ui.label("No album art variants are available for the current disc.");
            } else {
                ui.label("Available for the current disc:");
                for option in &cd_info.album_art_options {
                    let marker =
                        if Some(option.url.as_str()) == self.current_album_cover_url().as_deref() {
                            " (selected)"
                        } else {
                            ""
                        };
                    ui.label(format!("{}{}", option.label, marker));
                }
            }
        } else {
            ui.label("Detect a CD to inspect the available album art sizes.");
        }

        ui.separator();
        ui.label("MusicBrainz mode pulls text metadata from MusicBrainz and artwork from the Cover Art Archive. CDDB metadata currently has no album art source in this app.");

        if ui.button("Save Settings").clicked() {
            self.save_ui_settings();
        }
    }

    // ## Detects the current CD and populates disc, track, and album art state. 7
    fn detect_cd(&mut self) {
        let metadata_source = self.metadata_source_key();
        match CdReader::detect_with_metadata_source(metadata_source) {
            Ok(cd_info) => {
                self.status = format!("Detected {} tracks", cd_info.tracks.len());
                self.selected_tracks = vec![true; cd_info.tracks.len()];
                self.reset_album_cover_state();
                self.cd_info = Some(cd_info);
            }
            Err(err) => {
                self.status = format!("Failed to detect CD: {err}");
                self.selected_tracks.clear();
                self.reset_album_cover_state();
                self.cd_info = None;
            }
        }
    }

    // ## Retries CD detection on a timer while no disc is loaded. 8
    fn maybe_auto_detect_cd(&mut self, ctx: &egui::Context) {
        if self.cd_info.is_some() || self.is_ripping {
            return;
        }

        let now = Instant::now();
        if now < self.next_auto_detect_at {
            ctx.request_repaint_after(self.next_auto_detect_at - now);
            return;
        }

        self.next_auto_detect_at = now + Self::AUTO_DETECT_RETRY_INTERVAL;
        self.detect_cd();

        if self.cd_info.is_none() {
            ctx.request_repaint_after(Self::AUTO_DETECT_RETRY_INTERVAL);
        }
    }

    // ## Loads or clears the preview texture for the selected album cover URL. 9
    fn ensure_album_cover(&mut self, ctx: &egui::Context, cover_url: Option<&str>) {
        let Some(url) = cover_url else {
            self.album_cover_texture = None;
            self.album_cover_source_url = None;
            self.album_cover_error = None;
            return;
        };

        if self.album_cover_source_url.as_deref() == Some(url)
            && (self.album_cover_texture.is_some() || self.album_cover_error.is_some())
        {
            return;
        }

        self.album_cover_source_url = Some(url.to_string());
        self.album_cover_texture = None;
        self.album_cover_error = None;

        match Self::download_album_cover_texture(ctx, url) {
            Ok(texture) => {
                self.album_cover_texture = Some(texture);
            }
            Err(err) => {
                self.album_cover_error = Some(err);
            }
        }
    }

    // ## Downloads and decodes album cover bytes into an egui texture. 10
    fn download_album_cover_texture(
        ctx: &egui::Context,
        url: &str,
    ) -> Result<egui::TextureHandle, String> {
        let response = ureq::get(url)
            .call()
            .map_err(|err| format!("Failed to fetch album cover: {err}"))?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|err| format!("Failed to read album cover bytes: {err}"))?;

        let image = image::load_from_memory(&bytes)
            .map_err(|err| format!("Failed to decode album cover: {err}"))?
            .to_rgba8();

        let size = [image.width() as usize, image.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());

        Ok(ctx.load_texture("album-cover", color_image, egui::TextureOptions::LINEAR))
    }

    // ## Persists UI-controlled settings to the config file. 11
    fn save_ui_settings(&mut self) {
        let mut cfg = Config::load();
        cfg.encoder = self.encoder_key().to_string();
        cfg.metadata_source = self.metadata_source_key().to_string();
        cfg.album_art_size_preference = self.album_art_size_key().to_string();
        cfg.album_art_download_behavior = self.album_art_download_key().to_string();
        match cfg.save() {
            Ok(()) => {
                self.reset_album_cover_state();
                self.status = "Settings saved.".to_string();
            }
            Err(err) => self.status = format!("Failed to save settings: {err}"),
        }
    }

    // ## Validates selections and starts the ripper worker for the selected tracks. 12
    fn start_ripping(&mut self) {
        if self.cd_info.is_none() {
            self.status = "No CD detected.".to_string();
            return;
        }

        let selected_count = self.selected_tracks.iter().copied().filter(|v| *v).count();
        if selected_count == 0 {
            self.status = "No tracks selected. Please select at least one track.".to_string();
            return;
        }

        let output_dir = PathBuf::from(self.output_dir.trim());
        if output_dir.as_os_str().is_empty() {
            self.status = "Output directory is empty.".to_string();
            return;
        }

        let mut cfg = Config::load();
        cfg.encoder = self.encoder_key().to_string();
        cfg.metadata_source = self.metadata_source_key().to_string();
        cfg.album_art_size_preference = self.album_art_size_key().to_string();
        cfg.album_art_download_behavior = self.album_art_download_key().to_string();
        let _ = cfg.save();

        let mut selected_cd = self.cd_info.clone().unwrap();
        selected_cd.album_cover_url = self.current_album_cover_url();
        selected_cd.tracks = selected_cd
            .tracks
            .into_iter()
            .zip(self.selected_tracks.iter().copied())
            .filter_map(|(track, include)| include.then_some(track))
            .collect();

        let (sender, receiver) = mpsc::unbounded_channel();
        let ripper = Arc::new(Ripper::new(cfg, output_dir, sender));

        self.progress_fraction = 0.0;
        self.progress_label = "Starting...".to_string();
        self.status = "Ripping started.".to_string();
        self.is_ripping = true;
        self.ripper = Some(ripper.clone());
        self.rip_receiver = Some(receiver);

        std::thread::spawn(move || {
            ripper.rip(&selected_cd);
        });
    }

    // ## Cancels the active ripping job and resets ripping progress state. 13
    fn stop_ripping(&mut self) {
        if let Some(ripper) = &self.ripper {
            ripper.cancel();
        }
        self.finish_ripping_state();
        self.progress_fraction = 0.0;
        self.progress_label.clear();
        self.status = "Ripping stopped.".to_string();
    }

    // ## Ejects the disc and clears loaded disc state after a successful eject. 14
    fn eject_disc(&mut self) {
        match Command::new("eject").status() {
            Ok(status) if status.success() => {
                self.status = "Disc ejected.".to_string();
                self.cd_info = None;
                self.selected_tracks.clear();
                self.reset_album_cover_state();
                self.next_auto_detect_at = Instant::now() + Self::AUTO_DETECT_RETRY_INTERVAL;
            }
            Ok(_) => {
                self.status =
                    "Eject command failed. Ensure 'eject' is installed and permissions allow it."
                        .to_string();
            }
            Err(err) => {
                self.status = format!("Could not run 'eject': {err}");
            }
        }
    }

    // ## Drains pending ripper messages from the background worker channel. 15
    fn poll_ripper_messages(&mut self) {
        let mut disconnected = false;
        let mut messages = Vec::new();

        if let Some(receiver) = self.rip_receiver.as_mut() {
            loop {
                match receiver.try_recv() {
                    Ok(msg) => messages.push(msg),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        for msg in messages {
            self.handle_rip_message(msg);
        }

        if disconnected {
            self.rip_receiver = None;
            if self.is_ripping {
                self.finish_ripping_state();
            }
        }
    }

    // ## Applies one ripper progress, success, or error message to the UI state. 16
    fn handle_rip_message(&mut self, msg: RipMessage) {
        match msg {
            RipMessage::Progress(fraction, message) => {
                self.progress_fraction = fraction;
                self.progress_label = message;
            }
            RipMessage::TrackComplete(track_num) => {
                self.status = format!("Finished track {}", track_num);
            }
            RipMessage::Success => {
                self.finish_ripping_state();
                self.progress_fraction = 1.0;
                self.progress_label = "Completed".to_string();
                self.status = "CD ripped successfully!".to_string();
            }
            RipMessage::Error(err) => {
                self.finish_ripping_state();
                self.status = format!("Ripping failed: {err}");
            }
        }
    }

    // ## Clears worker handles when ripping ends or is canceled. 17
    fn finish_ripping_state(&mut self) {
        self.is_ripping = false;
        self.ripper = None;
        self.rip_receiver = None;
    }

    // ## Clears cached album cover texture, source URL, and error state. 18
    fn reset_album_cover_state(&mut self) {
        self.album_cover_texture = None;
        self.album_cover_source_url = None;
        self.album_cover_error = None;
    }

    // ## Chooses the album art preview scale percentage from preference and track count. 19
    fn cover_size_percent(&self, track_count: usize) -> f32 {
        match self.album_art_size_index {
            1 => 0.54,
            2 => 0.86,
            3 => 0.94,
            _ => {
                if track_count >= 18 {
                    0.62
                } else if track_count >= 12 {
                    0.70
                } else {
                    0.78
                }
            }
        }
    }

    // ## Chooses the track-list width percentage from the current track count. 20
    fn track_list_width_percent(track_count: usize) -> f32 {
        if track_count >= 18 {
            0.36
        } else if track_count >= 12 {
            0.32
        } else {
            0.30
        }
    }

    // ## Calculates the live track-list width for the current lower-pane size. 21
    fn current_track_list_width_for_layout(&self, track_count: usize, available_width: f32) -> f32 {
        let target_width = available_width * Self::track_list_width_percent(track_count);
        target_width.clamp(available_width.min(280.0), available_width.min(430.0))
    }

    // ## Calculates a responsive track-list font size from width, height, and track count. 22
    fn track_list_font_size(
        ui: &egui::Ui,
        track_count: usize,
        track_column_width: f32,
        content_height: f32,
    ) -> f32 {
        let body_size = egui::TextStyle::Body.resolve(ui.style()).size;
        let preferred_scale = 1.14;
        let width_scale = (track_column_width / 360.0).clamp(0.88, 1.16);
        let row_budget = content_height / track_count.max(1) as f32;
        let height_scale = (row_budget / (body_size * 1.72)).clamp(0.86, 1.12);
        let track_count_scale = (16.0 / track_count.max(1) as f32)
            .powf(0.08)
            .clamp(0.94, 1.06);

        body_size * preferred_scale * width_scale * height_scale * track_count_scale
    }

    // ## Calculates the album cover preview size for the current layout slot. 23
    fn current_cover_preview_size_for_layout(
        &self,
        track_count: usize,
        lower_content_height: f32,
        right_column_width: f32,
    ) -> egui::Vec2 {
        let side_budget = lower_content_height.min(right_column_width);
        let side = (side_budget * self.cover_size_percent(track_count))
            .clamp(120.0, 620.0)
            .min(side_budget);
        let max_size = egui::vec2(side, side);

        let Some(texture) = &self.album_cover_texture else {
            return max_size;
        };

        let mut size = texture.size_vec2();
        let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
        size *= scale;
        size
    }

    // ## Resolves the currently preferred album cover URL from loaded CD metadata. 24
    fn current_album_cover_url(&self) -> Option<String> {
        self.cd_info
            .as_ref()
            .and_then(|cd_info| cd_info.preferred_album_cover_url(self.album_art_size_key()))
            .map(ToOwned::to_owned)
    }

    // ## Maps the metadata source UI index to its config key. 25
    fn metadata_source_key(&self) -> &'static str {
        match self.metadata_source_index {
            1 => "musicbrainz",
            2 => "cddb",
            _ => "none",
        }
    }

    // ## Maps the encoder UI index to its config key. 26
    fn encoder_key(&self) -> &'static str {
        match self.format_index {
            1 => "mp3",
            2 => "wav",
            3 => "ogg",
            _ => "flac",
        }
    }

    // ## Maps the album art size UI index to its config key. 27
    fn album_art_size_key(&self) -> &'static str {
        match self.album_art_size_index {
            1 => "small",
            2 => "large",
            3 => "original",
            _ => "auto",
        }
    }

    // ## Maps the album art download UI index to its config key. 28
    fn album_art_download_key(&self) -> &'static str {
        match self.album_art_download_index {
            1 => "save-with-rip",
            _ => "preview-only",
        }
    }
}
