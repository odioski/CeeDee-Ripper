use crate::cd_reader::CdInfo;
use crate::config::Config;
use std::error::Error;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use gstreamer as gst;
use gstreamer::prelude::*;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub enum RipMessage {
    Progress(f64, String),
    TrackComplete(usize),
    Success,
    Error(String),
}

pub struct Ripper {
    config: Config,
    output_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    current_child: Arc<Mutex<Option<Child>>>,
    sender: UnboundedSender<RipMessage>,
    runtime: Arc<Runtime>,
}

impl Ripper {
    pub fn new(config: Config, output_dir: PathBuf, sender: UnboundedSender<RipMessage>) -> Self {
        let runtime = Arc::new(Runtime::new().unwrap());
        Self {
            config,
            output_dir,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            current_child: Arc::new(Mutex::new(None)),
            sender,
            runtime,
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.current_child.lock() {
            if let Some(child) = guard.as_mut() {
                let _ = child.kill();
            }
            guard.take();
        }
    }

    pub fn rip(&self, cd_info: &CdInfo) {
        let album_dir = self.output_dir.join(&cd_info.title);
        if let Err(e) = std::fs::create_dir_all(&album_dir) {
            self.sender
                .send(RipMessage::Error(format!(
                    "Failed to create directory: {}",
                    e
                )))
                .unwrap();
            return;
        }

        if cd_info.tracks.is_empty() {
            self.sender
                .send(RipMessage::Error(
                    "No tracks found to rip. Please detect again.".into(),
                ))
                .unwrap();
            return;
        }

        let total_tracks = cd_info.tracks.len();
        for (i, track_name) in cd_info.tracks.iter().enumerate() {
            let track_num = i + 1;
            if self.cancel_flag.load(Ordering::SeqCst) {
                self.sender
                    .send(RipMessage::Error("Ripping cancelled".into()))
                    .unwrap();
                return;
            }

            let progress = (track_num as f64) / (total_tracks as f64);
            let message = format!("Ripping track {} of {}...", track_num, total_tracks);
            self.sender
                .send(RipMessage::Progress(progress, message))
                .unwrap();

            let track_name_clone = track_name.clone();
            let album_dir_clone = album_dir.clone();
            let runtime = self.runtime.clone();
            let res =
                runtime.block_on(self.rip_track(track_num, &track_name_clone, &album_dir_clone));

            if let Err(e) = res {
                self.sender
                    .send(RipMessage::Error(format!(
                        "Failed to rip track {}: {}",
                        track_num, e
                    )))
                    .unwrap();
                return;
            }
            self.sender
                .send(RipMessage::TrackComplete(track_num))
                .unwrap();
        }

        self.sender.send(RipMessage::Success).unwrap();
    }

    async fn rip_track(
        &self,
        track_num: usize,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        let wav_file = output_dir.join(format!("track{:02}.wav", track_num));
        
        // Prefer library-based ripping via GStreamer cdparanoia element
        if let Err(e) = self.rip_track_via_gstreamer(track_num, &wav_file) {
            // Fallback: rip track with cdparanoia CLI, explicitly set device
            let child = Command::new("cdparanoia")
                .arg("-d")
                .arg(&self.config.device)
                .arg(format!("{}", track_num))
                .arg(&wav_file)
                .spawn()?;
            {
                let mut guard = self.current_child.lock().unwrap();
                *guard = Some(child);
            }
            // Poll process and respond to cancel
            loop {
                if self.cancel_flag.load(Ordering::SeqCst) {
                    self.cancel();
                    return Err("Ripping cancelled".into());
                }
                let completed = {
                    let mut guard = self.current_child.lock().unwrap();
                    if let Some(c) = guard.as_mut() {
                        if let Ok(Some(status)) = c.try_wait() {
                            let ok = status.success();
                            guard.take();
                            Some(ok)
                        } else { None }
                    } else { Some(false) }
                };
                if let Some(ok) = completed {
                    if ok {
                        break;
                    } else {
                        // Try cooked ioctl interface as a last resort
                        let child2 = Command::new("cdparanoia")
                            .arg("-k")
                            .arg(&self.config.device)
                            .arg(format!("{}", track_num))
                            .arg(&wav_file)
                            .spawn()?;
                        {
                            let mut guard = self.current_child.lock().unwrap();
                            *guard = Some(child2);
                        }
                        loop {
                            if self.cancel_flag.load(Ordering::SeqCst) {
                                self.cancel();
                                return Err("Ripping cancelled".into());
                            }
                            let completed2 = {
                                let mut guard = self.current_child.lock().unwrap();
                                if let Some(c2) = guard.as_mut() {
                                    if let Ok(Some(status2)) = c2.try_wait() {
                                        let ok2 = status2.success();
                                        guard.take();
                                        Some(ok2)
                                    } else { None }
                                } else { Some(false) }
                            };
                            if let Some(ok2) = completed2 {
                                if ok2 {
                                    break;
                                } else {
                                    // Try generic SCSI interface (-g) mapping sr -> sg
                                    if let Some(sg_dev) = crate::cd_reader::CdReader::find_generic_scsi_for_block(&self.config.device) {
                                        let child3 = Command::new("cdparanoia")
                                            .arg("-g")
                                            .arg(sg_dev)
                                            .arg(format!("{}", track_num))
                                            .arg(&wav_file)
                                            .spawn()?;
                                        {
                                            let mut guard = self.current_child.lock().unwrap();
                                            *guard = Some(child3);
                                        }
                                        loop {
                                            if self.cancel_flag.load(Ordering::SeqCst) {
                                                self.cancel();
                                                return Err("Ripping cancelled".into());
                                            }
                                            let completed3 = {
                                                let mut guard = self.current_child.lock().unwrap();
                                                if let Some(c3) = guard.as_mut() {
                                                    if let Ok(Some(status3)) = c3.try_wait() {
                                                        let ok3 = status3.success();
                                                        guard.take();
                                                        Some(ok3)
                                                    } else { None }
                                                } else { Some(false) }
                                            };
                                            if let Some(ok3) = completed3 {
                                                if ok3 { break; } else { return Err(format!("Failed to rip track {} (lib error: {})", track_num, e).into()); }
                                            }
                                            std::thread::sleep(Duration::from_millis(200));
                                        }
                                        break;
                                    } else {
                                        return Err(format!("Failed to rip track {} (lib error: {})", track_num, e).into());
                                    }
                                }
                            }
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }

        // Encode based on format
        let output_file = match self.config.encoder.as_str() {
            "flac" => self.encode_flac(&wav_file, track_name, output_dir)?,
            "mp3" => self.encode_mp3(&wav_file, track_name, output_dir)?,
            "ogg" => self.encode_ogg(&wav_file, track_name, output_dir)?,
            "wav" => {
                // Keep WAV but rename to track name for consistency
                let dest = output_dir.join(format!("{}.wav", track_name));
                if dest != wav_file {
                    // Ignore error if rename fails; keep original file name
                    let _ = std::fs::rename(&wav_file, &dest);
                    dest
                } else {
                    wav_file.clone()
                }
            }
            _ => wav_file.clone(),
        };

        // Remove WAV if we encoded to something else
        if output_file != wav_file {
            std::fs::remove_file(&wav_file)?;
        }

        Ok(())
    }

    fn encode_ogg(
        &self,
        input: &PathBuf,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let output = output_dir.join(format!("{}.ogg", track_name));

        // Use oggenc from vorbis-tools; quality from config (0-10)
        let status = Command::new("oggenc")
            .arg("-Q")
            .arg("-q")
            .arg(&self.config.quality)
            .arg("-o")
            .arg(&output)
            .arg(input)
            .status()?;

        if !status.success() {
            return Err("OGG encoding failed".into());
        }

        Ok(output)
    }

    fn rip_track_via_gstreamer(&self, track_num: usize, wav_file: &PathBuf) -> Result<(), Box<dyn Error>> {
        // Build a pipeline: cdparanoia device=<dev> track=<n> ! wavenc ! filesink location=<path>
        let pipe_str = format!(
            "cdparanoia device={} track={} ! wavenc ! filesink location={}",
            self.config.device,
            track_num,
            wav_file.display()
        );

        let element = gst::parse::launch(&pipe_str)?;
        let pipeline = element.dynamic_cast::<gst::Pipeline>()
            .map_err(|_| "Failed to create GStreamer pipeline")?;

        pipeline.set_state(gst::State::Playing)?;
        let bus = pipeline.bus().ok_or("Failed to get GStreamer bus")?;

        // Block until EOS or Error; check cancel periodically
        loop {
            if self.cancel_flag.load(Ordering::SeqCst) {
                let _ = pipeline.set_state(gst::State::Null);
                return Err("Ripping cancelled".into());
            }
            match bus.timed_pop(gst::ClockTime::from_mseconds(250)) {
                Some(msg) => match msg.view() {
                    gst::MessageView::Eos(_) => break,
                    gst::MessageView::Error(err) => {
                        let _ = pipeline.set_state(gst::State::Null);
                        return Err(format!("GStreamer error: {}", err.error()).into());
                    }
                    _ => {}
                },
                None => break,
            }
        }

        pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    fn encode_flac(
        &self,
        input: &PathBuf,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let output = output_dir.join(format!("{}.flac", track_name));
        
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

    fn encode_mp3(
        &self,
        input: &PathBuf,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let output = output_dir.join(format!("{}.mp3", track_name));
        
        let status = Command::new("lame")
            .arg("-b")
            .arg(&self.config.bitrate)
            .arg(input)
            .arg(&output)
            .status()?;

        if !status.success() {
            return Err("MP3 encoding failed".into());
        }

        Ok(output)
    }
}