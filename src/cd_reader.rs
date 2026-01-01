use std::error::Error;
use std::io;
use std::path::Path;
use std::process::Command;
use std::fs::File;
use std::os::fd::AsRawFd;
use crate::config::Config;
use discid::DiscId;
use serde_json::Value;
use libc;

#[derive(Debug, Clone)]
pub struct CdInfo {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub disc_id: String,
}

pub struct CdReader;

impl CdReader {
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

    pub fn detect() -> Result<CdInfo, Box<dyn Error>> {
        let device = Self::get_active_device_path();

        // Try raw TOC via ioctl first
        let track_count = match Self::read_toc_raw(&device) {
            Ok(n) if n > 0 => n,
            Ok(_) => {
                // zero tracks - try fallbacks
                Self::fallback_track_count(&device).ok_or_else(|| {
                    format!("No audio tracks detected on {} and fallbacks failed", device)
                })?
            }
            Err(err) => {
                // Permission or device-specific failure; try fallbacks (cdparanoia -Q)
                if let Some(n) = Self::fallback_track_count(&device) {
                    n
                } else {
                    let mut msg = format!(
                        "Failed to read TOC from {} ({}). ",
                        device,
                        err
                    );
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

        // Attempt metadata lookup based on config
        let cfg = Config::load();
        match cfg.metadata_source.as_str() {
            "musicbrainz" => {
                if let Some(info) = Self::fetch_musicbrainz_metadata(&device) {
                    cd_info = info;
                }
            }
            "cddb" => {
                if let Some(info) = Self::fetch_cddb_metadata(&device) {
                    cd_info = info;
                }
            }
            _ => {}
        }

        Ok(cd_info)
    }
    fn read_toc_raw(device: &str) -> Result<usize, io::Error> {
        // ioctl constants from linux/cdrom.h
        const CDROMREADTOCHDR: libc::c_ulong = 0x5305;
        #[repr(C)]
        struct CdromTocHdr { cdth_trk0: libc::c_uchar, cdth_trk1: libc::c_uchar }

        let f = File::open(device)?;
        let fd = f.as_raw_fd();
        let mut hdr = CdromTocHdr { cdth_trk0: 0, cdth_trk1: 0 };
        let ret = unsafe { libc::ioctl(fd, CDROMREADTOCHDR, &mut hdr as *mut _ as *mut libc::c_void) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
        let first = hdr.cdth_trk0 as usize;
        let last = hdr.cdth_trk1 as usize;
        Ok(if last >= first { last - first + 1 } else { 0 })
    }

    fn fallback_track_count(device: &str) -> Option<usize> {
        // Try cdparanoia -Q with block device
        if let Some(n) = Self::track_count_from_cdparanoia(device, None) {
            return Some(n);
        }
        // Try with generic SCSI mapped device (-g)
        if let Some(sg) = Self::find_generic_scsi_for_block(device) {
            if let Some(n) = Self::track_count_from_cdparanoia(device, Some(&sg)) {
                return Some(n);
            }
        }
        // As a last resort, some versions of cd-discid output the number of tracks as the second field
        if let Ok(o) = Command::new("cd-discid").arg(device).output() {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout);
                let mut it = s.split_whitespace();
                // Skip first token (disc id), next token may be number of tracks on some builds
                let _ = it.next();
                if let Some(tok) = it.next() {
                    if let Ok(n) = tok.parse::<usize>() { if n > 0 { return Some(n); } }
                }
            }
        }
        None
    }

    fn track_count_from_cdparanoia(device: &str, sg_dev: Option<&str>) -> Option<usize> {
        let mut cmd = Command::new("cdparanoia");
        cmd.arg("-Q");
        match sg_dev {
            Some(sg) => { cmd.arg("-g").arg(sg); },
            None => { cmd.arg("-d").arg(device); }
        }
        let out = cmd.output().ok()?;
        if !out.status.success() { return None; }
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
                    if s.contains('.') { count += 1; }
                }
                _ => {}
            }
        }
        if count > 0 { Some(count) } else { None }
    }

    pub fn find_generic_scsi_for_block(block_dev: &str) -> Option<String> {
        // Expect paths like /dev/sr0
        let name = Path::new(block_dev).file_name()?.to_string_lossy().to_string();
        let sys_block = Path::new("/sys/class/block").join(&name).join("device");
        let target = std::fs::read_link(&sys_block).ok()?; // symlink to SCSI device, e.g., ../../devices/pci.../hostX/targetX:X:X/X:X:X:X

        // Iterate scsi_generic entries and match their device symlink to the same target
        let sg_root = Path::new("/sys/class/scsi_generic");
        let entries = std::fs::read_dir(sg_root).ok()?;
        for entry in entries.flatten() {
            let sg_name = entry.file_name();
            let sg_path = entry.path().join("device");
            if let Ok(link) = std::fs::read_link(&sg_path) {
                if link == target {
                    let dev_path = format!("/dev/{}", sg_name.to_string_lossy());
                    if Path::new(&dev_path).exists() { return Some(dev_path); }
                }
            }
        }
        None
    }

    fn fetch_musicbrainz_metadata(_device: &str) -> Option<CdInfo> {
        // Read disc via libdiscid
        let disc = DiscId::read(None).ok()?;
        let mbid = disc.id();
        // Query MusicBrainz WS2 for discid
        let url = format!("https://musicbrainz.org/ws/2/discid/{}?inc=artists+recordings+release-groups&fmt=json", mbid);
        let resp = ureq::get(&url)
            .set("User-Agent", "ceedee-ripper/0.1 (https://example.invalid)")
            .call()
            .ok()?;
        let json: Value = resp.into_json().ok()?;
        let releases = json.get("releases")?.as_array()?;
        let first = releases.first()?;
        let album = first.get("title")?.as_str()?.to_string();
        let artist = first
            .get("artist-credit")
            .and_then(|ac| ac.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|v| v.get("name").and_then(|n| n.as_str()))
            .unwrap_or("Unknown Artist")
            .to_string();
        let media = first.get("media").and_then(|m| m.as_array()).and_then(|arr| arr.get(0));
        let tracks_v = media
            .and_then(|m| m.get("tracks"))
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        let mut tracks = Vec::new();
        for (i, t) in tracks_v.iter().enumerate() {
            let title_str = t.get("title")
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
        Some(CdInfo { title: album, artist, tracks, disc_id: mbid.to_string() })
    }

    fn fetch_cddb_metadata(device: &str) -> Option<CdInfo> {
        // Use cd-discid output to build a CDDB query
        let out = Command::new("cd-discid").arg(device).output().ok()?;
        if !out.status.success() { return None; }
        let s = String::from_utf8_lossy(&out.stdout);
        let mut toks = s.split_whitespace();
        let disc_id = toks.next()?.to_string();
        let ntracks: usize = toks.next()?.parse().ok()?;
        let mut offsets: Vec<usize> = Vec::with_capacity(ntracks);
        for _ in 0..ntracks { if let Some(tok) = toks.next() { if let Ok(v) = tok.parse::<usize>() { offsets.push(v); } } }
        let length_secs: usize = toks.next()?.parse().ok()?;
        if offsets.len() != ntracks { return None; }

        let mut url = format!(
            "http://gnudb.gnudb.org/cddb/cddb.cgi?cmd=cddb+query+{}+{}",
            disc_id, ntracks
        );
        for off in &offsets { url.push_str(&format!("+{}", off)); }
        url.push_str(&format!("+{}&hello=anon+localhost+ceedee-ripper+0.1&proto=6", length_secs));

        let resp = ureq::get(&url).call().ok()?;
        let body = resp.into_string().ok()?;
        // Expect lines like: 200 category title id
        let first_line = body.lines().next()?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.is_empty() { return None; }
        let code = parts[0];
        if code != "200" && code != "210" && code != "211" { return None; }
        // 200 <category> <title with spaces> <id> — we need category and id
        // Simplify: take category as second token and id as last token
        if parts.len() < 4 { return None; }
        let category = parts[1];
        let cddb_id = parts.last().copied().unwrap_or("");

        let read_url = format!(
            "http://gnudb.gnudb.org/cddb/cddb.cgi?cmd=cddb+read+{}+{}&hello=anon+localhost+ceedee-ripper+0.1&proto=6",
            category, cddb_id
        );
        let read_resp = ureq::get(&read_url).call().ok()?;
        let data = read_resp.into_string().ok()?;
        // Parse DTITLE and TTITLEi entries
        let mut album = String::from("Unknown Album");
        let mut artist = String::from("Unknown Artist");
        let mut tracks: Vec<String> = Vec::new();
        for line in data.lines() {
            if let Some(rest) = line.strip_prefix("DTITLE=") {
                // Format: Artist / Album
                if let Some((a, t)) = rest.split_once(" / ") {
                    artist = a.to_string();
                    album = t.to_string();
                } else {
                    album = rest.to_string();
                }
            } else if let Some(rest) = line.strip_prefix("TTITLE") {
                // TTITLE0=Track Name
                if let Some(eqpos) = rest.find('=') {
                    let title = &rest[eqpos+1..];
                    tracks.push(title.to_string());
                }
            } else if line.trim() == "." {
                break;
            }
        }
        if tracks.len() != ntracks {
            // Pad missing tracks with placeholders
            while tracks.len() < ntracks { tracks.push(format!("Track {}", tracks.len()+1)); }
        }
        Some(CdInfo { title: album, artist, tracks, disc_id: disc_id })
    }
    // Disc ID retrieval is handled directly within `detect()` using libdiscid.
    
    // Metadata lookup (CDDB/MusicBrainz) not implemented in simplified mode.
    
    // Default info with dynamic count is used when CDDB lookup is unavailable.

    fn create_default_info_with_count(disc_id: &str, track_count: usize) -> CdInfo {
        let tracks: Vec<String> = (1..=track_count)
            .map(|i| format!("Track {}", i))
            .collect();

        CdInfo {
            title: "Unknown Album".to_string(),
            artist: "Unknown Artist".to_string(),
            tracks,
            disc_id: disc_id.to_string(),
        }
    }

    // Track count helpers removed; libdiscid provides reliable TOC.
}