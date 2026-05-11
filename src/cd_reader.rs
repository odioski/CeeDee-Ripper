use crate::config::Config;
use discid::DiscId;
use libc;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AlbumArtOption {
    pub key: String,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct CdInfo {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub disc_id: String,
    pub album_cover_url: Option<String>,
    pub album_art_options: Vec<AlbumArtOption>,
    pub metadata_error: Option<String>,
}

impl CdInfo {
    pub fn preferred_album_cover_url(&self, preference: &str) -> Option<&str> {
        let preferred_keys: &[&str] = match preference {
            "small" => &["small", "large", "original"],
            "large" => &["large", "original", "small"],
            "original" => &["original", "large", "small"],
            _ => &["large", "original", "small"],
        };

        for key in preferred_keys {
            if let Some(option) = self
                .album_art_options
                .iter()
                .find(|option| option.key == *key)
            {
                return Some(option.url.as_str());
            }
        }

        self.album_cover_url.as_deref()
    }
}

pub struct CdReader;

impl CdReader {
    #[cfg(feature = "egui-ui")]
    pub fn active_device_path() -> String {
        Self::get_active_device_path()
    }

    fn get_active_device_path() -> String {
        // Highest priority: environment override
        if let Ok(dev) = std::env::var("CD_DEVICE") {
            if Path::new(&dev).exists() {
                return dev;
            }
        }

        // Next: configuration value
        let cfg = Config::load();
        if Path::new(&cfg.device).exists() {
            return cfg.device;
        }

        // Fallback: common device paths
        let candidates = ["/dev/cdrom", "/dev/sr0", "/dev/sr1"];
        for device in candidates {
            if Path::new(device).exists() {
                return device.to_string();
            }
        }
        "/dev/sr0".to_string()
    }

    #[cfg(feature = "gtk-ui")]
    pub fn detect() -> Result<CdInfo, Box<dyn Error>> {
        let cfg = Config::load();
        Self::detect_impl(&cfg.metadata_source)
    }

    #[cfg(feature = "egui-ui")]
    pub fn detect_with_metadata_source(metadata_source: &str) -> Result<CdInfo, Box<dyn Error>> {
        Self::detect_impl(metadata_source)
    }

    fn detect_impl(metadata_source: &str) -> Result<CdInfo, Box<dyn Error>> {
        let device = Self::get_active_device_path();

        // Try raw TOC via ioctl first
        let track_count = match Self::read_toc_raw(&device) {
            Ok(n) if n > 0 => n,
            Ok(_) => {
                // zero tracks - try fallbacks
                Self::fallback_track_count(&device).ok_or_else(|| {
                    format!(
                        "No audio tracks detected on {} and fallbacks failed",
                        device
                    )
                })?
            }
            Err(err) => {
                // Permission or device-specific failure; try fallbacks (cdparanoia -Q)
                if let Some(n) = Self::fallback_track_count(&device) {
                    n
                } else {
                    let mut msg = format!("Failed to read TOC from {} ({}). ", device, err);
                    if matches!(err.raw_os_error(), Some(libc::EACCES) | Some(libc::EPERM)) {
                        msg.push_str(
                            "You may need to add your user to the 'cdrom' group and re-login: sudo usermod -aG cdrom $USER. ",
                        );
                    }
                    msg.push_str("Tried cdparanoia query as fallback but it also failed.");
                    return Err(msg.into());
                }
            }
        };
        if track_count == 0 {
            return Err("No audio tracks detected".into());
        }

        // Build baseline info
        let mut cd_info = Self::create_default_info_with_count("", track_count);

        // MusicBrainz is the single supported metadata source.
        if metadata_source == "musicbrainz" {
            match Self::fetch_musicbrainz_metadata(&device) {
                Ok(info) => cd_info = info,
                Err(err) => cd_info.metadata_error = Some(err),
            }
        }

        Ok(cd_info)
    }
    fn read_toc_raw(device: &str) -> Result<usize, io::Error> {
        // ioctl constants from linux/cdrom.h
        const CDROMREADTOCHDR: libc::Ioctl = 0x5305;
        #[repr(C)]
        struct CdromTocHdr {
            cdth_trk0: libc::c_uchar,
            cdth_trk1: libc::c_uchar,
        }

        let f = File::open(device)?;
        let fd = f.as_raw_fd();
        let mut hdr = CdromTocHdr {
            cdth_trk0: 0,
            cdth_trk1: 0,
        };
        let ret =
            unsafe { libc::ioctl(fd, CDROMREADTOCHDR, &mut hdr as *mut _ as *mut libc::c_void) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
        let first = hdr.cdth_trk0 as usize;
        let last = hdr.cdth_trk1 as usize;
        Ok(if last >= first { last - first + 1 } else { 0 })
    }

    fn fallback_track_count(device: &str) -> Option<usize> {
        if let Some(n) = Self::track_count_from_cdparanoia(device) {
            return Some(n);
        }
        // As a last resort, some versions of cd-discid output the number of tracks as the second field
        if let Ok(o) = Command::new("cd-discid").arg(device).output() {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout);
                let mut it = s.split_whitespace();
                // Skip first token (disc id), next token may be number of tracks on some builds
                let _ = it.next();
                if let Some(tok) = it.next() {
                    if let Ok(n) = tok.parse::<usize>() {
                        if n > 0 {
                            return Some(n);
                        }
                    }
                }
            }
        }
        None
    }

    fn track_count_from_cdparanoia(device: &str) -> Option<usize> {
        let mut cmd = Command::new("cdparanoia");
        cmd.arg("-Q").arg("-d").arg(device);
        let out = cmd.output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        Self::parse_cdparanoia_q_for_track_count(&text)
    }

    fn parse_cdparanoia_q_for_track_count(output: &str) -> Option<usize> {
        let mut count = 0usize;
        for line in output.lines() {
            let s = line.trim_start();
            // Lines like "  1.  0:02.00 ..." — count lines that start with a number and a dot
            let mut chars = s.chars();
            match chars.next() {
                Some(c) if c.is_ascii_digit() => {
                    if s.contains('.') {
                        count += 1;
                    }
                }
                _ => {}
            }
        }
        if count > 0 {
            Some(count)
        } else {
            None
        }
    }

    fn fetch_musicbrainz_metadata(device: &str) -> Result<CdInfo, String> {
        // Read disc via libdiscid using the same block device selected for ripping.
        let disc = DiscId::read(Some(device))
            .map_err(|err| format!("MusicBrainz Disc ID lookup failed: {err}"))?;
        let mbid = disc.id();
        // Query MusicBrainz WS2 for discid
        let url = format!(
            "https://musicbrainz.org/ws/2/discid/{}?inc=artists+recordings+release-groups&fmt=json",
            mbid
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();
        let resp = agent
            .get(&url)
            .set("User-Agent", "ceedee-ripper/0.1 (https://example.invalid)")
            .call()
            .map_err(|err| format!("MusicBrainz request failed: {err}"))?;
        let json: Value = resp
            .into_json()
            .map_err(|err| format!("MusicBrainz response was not valid JSON: {err}"))?;
        let releases = json
            .get("releases")
            .and_then(|releases| releases.as_array())
            .ok_or_else(|| "MusicBrainz response did not include releases".to_string())?;
        let first = releases
            .first()
            .ok_or_else(|| format!("MusicBrainz found no releases for disc ID {mbid}"))?;

        // Fetch cover art from Cover Art Archive
        let mut album_cover_url = None;
        let mut album_art_options = Vec::new();
        if let Some(release_mbid) = first.get("id").and_then(|id| id.as_str()) {
            let cover_art_url = format!("https://coverartarchive.org/release/{}", release_mbid);
            if let Ok(cover_resp) = agent.get(&cover_art_url).call() {
                if let Ok(cover_json) = cover_resp.into_json::<Value>() {
                    if let Some(images) = cover_json.get("images").and_then(|i| i.as_array()) {
                        let front_image = images.iter().find(|img| {
                            img.get("front").and_then(|v| v.as_bool()).unwrap_or(false)
                        });
                        if let Some(img) = front_image {
                            Self::push_album_art_option(
                                &mut album_art_options,
                                "small",
                                "Small thumbnail",
                                img.get("thumbnails").and_then(|t| t.get("small")),
                            );
                            Self::push_album_art_option(
                                &mut album_art_options,
                                "large",
                                "Large thumbnail",
                                img.get("thumbnails").and_then(|t| t.get("large")),
                            );
                            Self::push_album_art_option(
                                &mut album_art_options,
                                "original",
                                "Original image",
                                img.get("image"),
                            );
                        }

                        let preferred_size = Config::load().album_art_size_preference;
                        album_cover_url =
                            Self::preferred_album_art_url(&album_art_options, &preferred_size)
                                .map(ToOwned::to_owned);
                    }
                }
            }
        }

        let album = first
            .get("title")
            .and_then(|title| title.as_str())
            .ok_or_else(|| "MusicBrainz release did not include an album title".to_string())?
            .to_string();
        let artist = first
            .get("artist-credit")
            .and_then(|ac| ac.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|v| v.get("name").and_then(|n| n.as_str()))
            .unwrap_or("Unknown Artist")
            .to_string();
        let media = first
            .get("media")
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.get(0));
        let tracks_v = media
            .and_then(|m| m.get("tracks"))
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        let mut tracks = Vec::new();
        for (i, t) in tracks_v.iter().enumerate() {
            let title_str = t
                .get("title")
                .or_else(|| t.get("recording").and_then(|r| r.get("title")))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Track {}", i + 1));
            tracks.push(title_str);
        }
        if tracks.is_empty() {
            // Fallback: generate placeholders based on disc track count
            let count = disc.last_track_num() as usize;
            tracks = (1..=count).map(|i| format!("Track {}", i)).collect();
        }
        Ok(CdInfo {
            title: album,
            artist,
            tracks,
            disc_id: mbid.to_string(),
            album_cover_url,
            album_art_options,
            metadata_error: None,
        })
    }

    fn create_default_info_with_count(disc_id: &str, track_count: usize) -> CdInfo {
        let tracks: Vec<String> = (1..=track_count).map(|i| format!("Track {}", i)).collect();

        CdInfo {
            title: "Unknown Album".to_string(),
            artist: "Unknown Artist".to_string(),
            tracks,
            disc_id: disc_id.to_string(),
            album_cover_url: None,
            album_art_options: Vec::new(),
            metadata_error: None,
        }
    }

    fn push_album_art_option(
        options: &mut Vec<AlbumArtOption>,
        key: &str,
        label: &str,
        value: Option<&Value>,
    ) {
        let Some(url) = value.and_then(|value| value.as_str()) else {
            return;
        };

        if options.iter().any(|option| option.url == url) {
            return;
        }

        options.push(AlbumArtOption {
            key: key.to_string(),
            label: label.to_string(),
            url: url.to_string(),
        });
    }

    fn preferred_album_art_url<'a>(
        options: &'a [AlbumArtOption],
        preference: &str,
    ) -> Option<&'a str> {
        let preferred_keys: &[&str] = match preference {
            "small" => &["small", "large", "original"],
            "large" => &["large", "original", "small"],
            "original" => &["original", "large", "small"],
            _ => &["large", "original", "small"],
        };

        for key in preferred_keys {
            if let Some(option) = options.iter().find(|option| option.key == *key) {
                return Some(option.url.as_str());
            }
        }

        None
    }

    // Track count helpers removed; libdiscid provides reliable TOC.
}
