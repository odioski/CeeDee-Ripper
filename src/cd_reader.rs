use crate::config::Config;
use discid::DiscId;
use libc;
use serde_json::Value;
use std::error::Error;
use std::fs::OpenOptions;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
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

        let track_count = match Self::read_toc_raw(&device) {
            Ok(count) => count,
            Err(err) => match Self::fallback_track_count(&device) {
                Ok(count) => count,
                Err(fallback) => {
                    let hint = match err.raw_os_error() {
                        Some(libc::EACCES) | Some(libc::EPERM) => {
                            " Check your read permissions or desktop device-access ACLs."
                        }
                        Some(libc::ENOMEDIUM) => {
                            " Insert an audio CD and wait for the drive to become ready."
                        }
                        _ => "",
                    };
                    return Err(format!(
                        "Failed to read audio TOC from {device}: {err}.{hint} Fallbacks: {fallback}"
                    )
                    .into());
                }
            },
        };

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

        // Audio CDs do not expose a filesystem. A blocking block-device open
        // may fail with ENOMEDIUM before we even get to the TOC ioctl.
        // Keep the File alive until every ioctl finishes; it owns/closes the fd.
        let f = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(device)?;
        let fd = f.as_raw_fd();
        let mut hdr = CdromTocHdr {
            cdth_trk0: 0,
            cdth_trk1: 0,
        };
        loop {
            // SAFETY: fd is owned by f and hdr matches linux/cdrom.h's two-byte
            // cdrom_tochdr. The kernel writes only that structure.
            let ret = unsafe { libc::ioctl(fd, CDROMREADTOCHDR, &mut hdr) };
            if ret >= 0 {
                break;
            }
            let err = io::Error::last_os_error();
            if err.kind() != io::ErrorKind::Interrupted {
                return Err(err);
            }
        }
        let count = Self::validate_toc_header(i32::from(hdr.cdth_trk0), i32::from(hdr.cdth_trk1))?;

        Ok(count)
    }

    fn validate_toc_header(first: i32, last: i32) -> io::Result<usize> {
        if first < 1 || last > 99 || last < first {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid TOC track range {first}..{last}"),
            ));
        }
        Ok((last - first + 1) as usize)
    }

    fn fallback_track_count(device: &str) -> Result<usize, String> {
        let paranoia_error = match Self::track_count_from_cdparanoia(device) {
            Ok(count) => return Ok(count),
            Err(err) => err,
        };
        // Keep the original cd-discid fallback for drives that it handles
        // better than cdparanoia, but validate its documented output format.
        let discid_error = match Command::new("cd-discid").arg(device).output() {
            Ok(out) if out.status.success() => {
                if let Some(count) = Self::parse_cd_discid(&String::from_utf8_lossy(&out.stdout)) {
                    return Ok(count);
                }
                "cd-discid returned an invalid TOC".to_string()
            }
            Ok(out) => format!(
                "cd-discid {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(err) => format!("could not run cd-discid: {err}"),
        };
        Err(format!("{paranoia_error}; {discid_error}"))
    }

    fn parse_cd_discid(output: &str) -> Option<usize> {
        // disc ID, track count, one frame offset per track, total seconds.
        let fields: Vec<_> = output.split_whitespace().collect();
        let count = fields.get(1)?.parse::<usize>().ok()?;
        if !(1..=99).contains(&count) || fields.len() != count + 3 {
            return None;
        }
        u32::from_str_radix(fields[0], 16).ok()?;
        for field in &fields[2..] {
            field.parse::<u32>().ok()?;
        }
        Some(count)
    }

    fn track_count_from_cdparanoia(device: &str) -> Result<usize, String> {
        let out = Command::new("cdparanoia")
            .args(["-Q", "-d", device])
            .env("LC_ALL", "C")
            .output()
            .map_err(|err| format!("could not run cdparanoia: {err}"))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() {
            return Err(format!("cdparanoia {}: {}", out.status, stderr.trim()));
        }
        Self::parse_query_streams(&stdout, &stderr)
            .ok_or_else(|| format!("cdparanoia returned no audio track rows: {}", stderr.trim()))
    }

    fn parse_query_streams(stdout: &str, stderr: &str) -> Option<usize> {
        // Query tables normally appear on stderr. Some wrappers use stdout;
        // parse each separately so mirrored output does not double the count.
        Self::parse_cdparanoia_q_for_track_count(stderr)
            .or_else(|| Self::parse_cdparanoia_q_for_track_count(stdout))
    }

    fn parse_cdparanoia_q_for_track_count(output: &str) -> Option<usize> {
        let mut tracks = std::collections::BTreeSet::new();
        for line in output.lines() {
            let mut fields = line.split_whitespace();
            let Some(track) = fields
                .next()
                .and_then(|field| field.strip_suffix('.'))
                .and_then(|field| field.parse::<u8>().ok())
            else {
                continue;
            };
            // A real query row begins with "1. <length in sectors> [mm:ss.ff]".
            // Reject version numbers, progress output, and diagnostic prose.
            let sectors = fields.next().and_then(|field| field.parse::<u32>().ok());
            let time = fields.next().unwrap_or_default();
            if (1..=99).contains(&track)
                && sectors.is_some()
                && time.starts_with('[')
                && time.ends_with(']')
                && time.contains(':')
            {
                tracks.insert(track);
            }
        }
        (!tracks.is_empty()).then_some(tracks.len())
    }

    fn fetch_musicbrainz_metadata(device: &str) -> Result<CdInfo, String> {
        // Read disc via libdiscid using the same block device selected for ripping.
        let disc = DiscId::read(Some(device))
            .map_err(|err| format!("MusicBrainz Disc ID lookup failed: {err}"))?;
        let mbid = disc.id();
        let toc = disc.toc_string().replace(' ', "+");
        // Query MusicBrainz WS2 for discid
        let url = format!(
            "https://musicbrainz.org/ws/2/discid/{}?toc={}&inc=artists+recordings+release-groups&fmt=json",
            mbid, toc
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();
        let resp = agent
            .get(&url)
            .set(
                "User-Agent",
                "ceedee-ripper/1.1.0 (https://github.com/odioski/CeeDee-Ripper)",
            )
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
}

#[cfg(test)]
mod tests {
    use super::CdReader;

    const QUERY: &str = "Table of contents (audio tracks only):\n\
        track        length               begin        copy pre ch\n\
          1.    15000 [03:20.00]        0 [00:00.00]    no   no  2\n\
          2.    22500 [05:00.00]    15000 [03:20.00]    no   no  2\n\
        TOTAL 37500 [08:20.00] (audio only)\n";

    #[test]
    fn query_table_on_stderr_is_read() {
        assert_eq!(CdReader::parse_query_streams("", QUERY), Some(2));
        assert_eq!(CdReader::parse_query_streams(QUERY, ""), Some(2));
        assert_eq!(CdReader::parse_query_streams(QUERY, QUERY), Some(2));
    }

    #[test]
    fn diagnostics_are_not_tracks() {
        assert_eq!(
            CdReader::parse_query_streams("", "10.2 release\n1. drive failed\n"),
            None
        );
        assert_eq!(CdReader::parse_query_streams("", ""), None);
    }

    #[test]
    fn toc_ranges_are_validated() {
        assert_eq!(CdReader::validate_toc_header(1, 12).unwrap(), 12);
        assert_eq!(CdReader::validate_toc_header(3, 5).unwrap(), 3);
        for (first, last) in [(0, 0), (0, 1), (5, 4), (1, 100)] {
            assert!(CdReader::validate_toc_header(first, last).is_err());
        }
    }

    #[test]
    fn cd_discid_output_requires_a_complete_toc() {
        assert_eq!(
            CdReader::parse_cd_discid("abcdef01 2 150 15150 502\n"),
            Some(2)
        );
        assert_eq!(CdReader::parse_cd_discid("abcdef01 2"), None);
        assert_eq!(CdReader::parse_cd_discid("abcdef01 0 0"), None);
        assert_eq!(CdReader::parse_cd_discid("error 2 150 15150 502"), None);
    }

    #[test]
    fn ioctl_failure_preserves_errno() {
        let error = CdReader::read_toc_raw("/dev/null").unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTTY));
    }

    #[test]
    #[ignore = "requires an audio CD in CD_DEVICE (defaults to /dev/sr0)"]
    fn audio_cd_hardware_toc() {
        let device = std::env::var("CD_DEVICE").unwrap_or_else(|_| "/dev/sr0".into());
        let count = CdReader::read_toc_raw(&device).expect("audio CD TOC read failed");
        assert!((1..=99).contains(&count));
        eprintln!("{device}: {count} tracks");
    }
}
