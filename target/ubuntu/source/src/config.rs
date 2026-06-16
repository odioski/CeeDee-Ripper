use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub encoder: String,
    pub bitrate: String,
    pub quality: String,
    pub device: String,
    pub ui_backend: String,                  // egui | gtk
    pub metadata_source: String,             // musicbrainz
    pub album_art_size_preference: String,   // auto | small | large | original
    pub album_art_download_behavior: String, // preview-only | save-with-rip
}

impl Default for Config {
    fn default() -> Self {
        Self {
            encoder: "flac".to_string(),
            bitrate: "320".to_string(),
            quality: "8".to_string(),
            device: "/dev/sr0".to_string(),
            ui_backend: "egui".to_string(),
            // Default to MusicBrainz so metadata auto-engages on first run
            metadata_source: "musicbrainz".to_string(),
            album_art_size_preference: "auto".to_string(),
            album_art_download_behavior: "preview-only".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            let cfg = Self::default();
            let _ = cfg.save();
            cfg
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self).unwrap();
        fs::write(config_path, content)
    }

    fn config_path() -> PathBuf {
        if let Ok(path) = std::env::var("CEEDEE_RIPPER_CONFIG") {
            return PathBuf::from(path);
        }

        if let Ok(cwd) = std::env::current_dir() {
            let repo_config = cwd.join("config").join("config.toml");
            if repo_config.exists() {
                return repo_config;
            }
        }

        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ceedee-ripper")
            .join("config.toml")
    }
}
