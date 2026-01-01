use crate::cd_reader::CdInfo;
use crate::config::Config;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};
use std::time::Duration;
use gstreamer as gst;
use gstreamer::prelude::*;
use gtk::prelude::*;
use tokio::task;

pub struct Ripper {
    config: Config,
    output_dir: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    current_child: Arc<Mutex<Option<std::process::Child>>>,
}

impl Ripper {
    pub fn new(config: Config, output_dir: PathBuf) -> Self {
        Self {
            config,
            output_dir,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            current_child: Arc::new(Mutex::new(None)),
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

    pub async fn rip(&self, cd_info: &CdInfo, track_list: Option<&gtk::ListBox>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let album_dir = self.output_dir.join(&cd_info.title);
        std::fs::create_dir_all(&album_dir)?;

        if cd_info.tracks.is_empty() {
            return Err("No tracks found to rip. Please detect again.".into());
        }

        let tracks_to_rip: Vec<(usize, String)> = if let Some(list) = track_list {
            list.selected_rows()
                .iter()
                .map(|row| row.index() as usize)
                .filter_map(|index| {
                    cd_info
                        .tracks
                        .get(index)
                        .map(|name| (index + 1, name.clone()))
                })
                .collect()
        } else {
            cd_info
                .tracks
                .iter()
                .enumerate()
                .map(|(i, name)| (i + 1, name.clone()))
                .collect()
        };

        if tracks_to_rip.is_empty() {
            return Err("No tracks selected to rip.".into());
        }

        for (track_num, track_name) in tracks_to_rip {
            if self.cancel_flag.load(Ordering::SeqCst) {
                return Err("Ripping cancelled".into());
            }
            self.rip_track(track_num, &track_name, &album_dir).await?;
        }

        Ok(())
    }

    async fn rip_track(
        &self,
        track_num: usize,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let wav_file = output_dir.join(format!("track{:02}.wav", track_num));
        
        if self.rip_track_via_gstreamer(track_num, &wav_file).await.is_err() {
            let device = self.config.device.clone();
            let wav_file_clone = wav_file.clone();
            let child_arc = self.current_child.clone();
            let cancel_flag_arc = self.cancel_flag.clone();

            task::spawn_blocking(move || {
                let child = Command::new("cdparanoia")
                    .arg("-d")
                    .arg(&device)
                    .arg(format!("{}", track_num))
                    .arg(&wav_file_clone)
                    .spawn()?;
                
                {
                    let mut guard = child_arc.lock().unwrap();
                    *guard = Some(child);
                }

                loop {
                    if cancel_flag_arc.load(Ordering::SeqCst) {
                        return Err(Box::<dyn Error + Send + Sync>::from("Ripping cancelled"));
                    }
                    let mut guard = child_arc.lock().unwrap();
                    if let Some(c) = guard.as_mut() {
                        if let Ok(Some(status)) = c.try_wait() {
                            return if status.success() {
                                Ok(())
                            } else {
                                Err(format!("cdparanoia failed for track {}", track_num).into())
                            };
                        }
                    } else {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                if cancel_flag_arc.load(Ordering::SeqCst) {
                    Err(Box::from("Ripping cancelled"))
                } else {
                    Ok(())
                }
            }).await??;
        }

        let config_clone = self.config.clone();
        let track_name_clone = track_name.to_string();
        let output_dir_clone = output_dir.clone();
        let wav_file_clone = wav_file.clone();

        task::spawn_blocking(move || {
            let output_file = match config_clone.encoder.as_str() {
                "flac" => config_clone.encode_flac(&wav_file_clone, &track_name_clone, &output_dir_clone)?,
                "mp3" => config_clone.encode_mp3(&wav_file_clone, &track_name_clone, &output_dir_clone)?,
                "ogg" => config_clone.encode_ogg(&wav_file_clone, &track_name_clone, &output_dir_clone)?,
                "wav" => {
                    let dest = output_dir_clone.join(format!("{}.wav", track_name_clone));
                    if dest != wav_file_clone {
                        let _ = std::fs::rename(&wav_file_clone, &dest);
                        dest
                    } else {
                        wav_file_clone.clone()
                    }
                }
                _ => wav_file_clone.clone(),
            };

            if output_file != wav_file_clone {
                std::fs::remove_file(&wav_file_clone)?;
            }
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        }).await??;

        Ok(())
    }

    async fn rip_track_via_gstreamer(&self, track_num: usize, wav_file: &PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
        let device = self.config.device.clone();
        let wav_file_str = wav_file.to_str().ok_or("Invalid path")?.to_string();
        let cancel_flag = self.cancel_flag.clone();

        task::spawn_blocking(move || {
            gst::init()?;
            let pipe_str = format!(
                "cdparanoia device={} track={} ! wavenc ! filesink location={}",
                device,
                track_num,
                wav_file_str
            );

            let pipeline = gst::parse::launch(&pipe_str)?;
            
            pipeline.set_state(gst::State::Playing)?;
            let bus = pipeline.bus().ok_or("Failed to get GStreamer bus")?;

            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                if cancel_flag.load(Ordering::SeqCst) {
                    pipeline.set_state(gst::State::Null)?;
                    return Err(Box::from("Ripping cancelled"));
                }
                match msg.view() {
                    gst::MessageView::Eos(_) => break,
                    gst::MessageView::Error(err) => {
                        pipeline.set_state(gst::State::Null)?;
                        return Err(format!("GStreamer error: {}", err.error()).into());
                    }
                    _ => {}
                }
            }

            pipeline.set_state(gst::State::Null)?;
            Ok(())
        }).await?
    }
}

