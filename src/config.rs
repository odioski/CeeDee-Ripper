use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub encoder: String,
    pub bitrate: String,
    pub quality: String,
    pub cddb_enabled: bool,
    pub device: String,
    pub metadata_source: String, // none | musicbrainz | cddb
}

impl Default for Config {
    fn default() -> Self {
        Self {
            encoder: "flac".to_string(),
            bitrate: "320".to_string(),
            quality: "8".to_string(),
            cddb_enabled: true,
            device: "/dev/sr0".to_string(),
            metadata_source: "musicbrainz".to_string(),
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
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ceedee-ripper")
            .join("config.toml")
    }

    pub fn encode_ogg(
        &self,
        input: &PathBuf,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let output = output_dir.join(format!("{}.ogg", track_name));
        let status = Command::new("oggenc")
            .arg("-Q")
            .arg("-q")
            .arg(&self.quality)
            .arg("-o")
            .arg(&output)
            .arg(input)
            .status()?;
        if !status.success() {
            return Err("OGG encoding failed".into());
        }
        Ok(output)
    }

    pub fn encode_flac(
        &self,
        input: &PathBuf,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
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

    pub fn encode_mp3(
        &self,
        input: &PathBuf,
        track_name: &str,
        output_dir: &PathBuf,
    ) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let output = output_dir.join(format!("{}.mp3", track_name));
        let status = Command::new("lame")
            .arg("-b")
            .arg(&self.bitrate)
            .arg(input)
            .arg(&output)
            .status()?;
        if !status.success() {
            return Err("MP3 encoding failed".into());
        }
        Ok(output)
    }
}