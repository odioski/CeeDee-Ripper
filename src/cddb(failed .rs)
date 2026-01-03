use std::error::Error;
use std::process::Command;

/// Holds metadata fetched from a CDDB server.
#[derive(Debug, Clone)]
pub struct CddbInfo {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub disc_id: String,
}

/// Fetches release metadata from a CDDB server (gnudb.gnudb.org).
///
/// This function uses the `cd-discid` command to get the disc's table of
/// contents, then queries a public CDDB server for matching entries.
///
/// # Arguments
///
/// * `device` - The path to the CD-ROM device (e.g., "/dev/sr0").
///
/// # Returns
///
/// A `Result` containing `CddbInfo` on success, or a boxed error if the
/// lookup fails.
pub fn fetch_cddb_metadata(device: &str) -> Result<CddbInfo, Box<dyn Error>> {
    // 1. Get disc information using `cd-discid` command-line tool.
    let output = Command::new("cd-discid").arg(device).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cd-discid failed with status: {}. Stderr: {}",
            output.status, stderr
        )
        .into());
    }

    let s = String::from_utf8_lossy(&output.stdout);
    let mut toks = s.split_whitespace();
    let disc_id = toks.next().ok_or("cd-discid output missing disc_id")?.to_string();
    let ntracks: usize = toks.next().ok_or("cd-discid output missing track count")?.parse()?;
    let mut offsets: Vec<usize> = Vec::with_capacity(ntracks);
    for _ in 0..ntracks {
        offsets.push(toks.next().ok_or("cd-discid output missing offset")?.parse()?);
    }
    let length_secs: usize = toks.next().ok_or("cd-discid output missing disc length")?.parse()?;
    if offsets.len() != ntracks {
        return Err("Failed to parse all track offsets from cd-discid output".into());
    }

    // 2. Query the CDDB server.
    let mut query_url = format!(
        "http://gnudb.gnudb.org/cddb/cddb.cgi?cmd=cddb+query+{}+{}",
        disc_id, ntracks
    );
    for off in &offsets {
        query_url.push_str(&format!("+{}", off));
    }
    query_url.push_str(&format!("+{}&hello=anon+localhost+ceedee-ripper+0.1&proto=6", length_secs));

    let resp = ureq::get(&query_url)
        .set("User-Agent", "ceedee-ripper/0.1 (https://example.invalid)")
        .call()?;
    let body = resp.into_string()?;

    // 3. Parse the query response to get category and ID for the read command.
    let mut lines = body.lines();
    let first_line = lines.next().ok_or("CDDB query response was empty")?;
    let mut query_parts: Vec<&str> = first_line.split_whitespace().collect();
    if query_parts.is_empty() {
        return Err("CDDB query response line is empty".into());
    }
    let code = query_parts[0];

    let (category, cddb_id) = if code == "200" {
        // Exact match: 200 <category> <discid> <title>
        if query_parts.len() < 3 {
            return Err(format!("Malformed CDDB 200 response: {}", first_line).into());
        }
        (query_parts[1], query_parts[2])
    } else if code == "210" || code == "211" {
        // Inexact matches. We'll take the first one.
        // The next line should be the first match: <category> <discid> <title>
        let match_line = lines.next().ok_or("CDDB 21x response has no match lines")?;
        let match_parts: Vec<&str> = match_line.split_whitespace().collect();
        if match_parts.len() < 2 {
            return Err(format!("Malformed CDDB 21x match line: {}", match_line).into());
        }
        (match_parts[0], match_parts[1])
    } else {
        // e.g., 202 No match found, 4xx errors
        return Err(format!("CDDB query failed or no match: {}", first_line).into());
    };

    // 4. Read the full entry from the CDDB server.
    let read_url = format!(
        "http://gnudb.gnudb.org/cddb/cddb.cgi?cmd=cddb+read+{}+{}&hello=anon+localhost+ceedee-ripper+0.1&proto=6",
        category, cddb_id
    );
    let read_resp = ureq::get(&read_url)
        .set("User-Agent", "ceedee-ripper/0.1 (https://example.invalid)")
        .call()?;
    let data = read_resp.into_string()?;

    // 5. Parse the entry data.
    let mut album = String::from("Unknown Album");
    let mut artist = String::from("Unknown Artist");
    let mut tracks: Vec<String> = Vec::with_capacity(ntracks);
    for line in data.lines() {
        if let Some(rest) = line.strip_prefix("DTITLE=") {
            if let Some((a, t)) = rest.split_once(" / ") {
                artist = a.trim().to_string();
                album = t.trim().to_string();
            } else {
                album = rest.trim().to_string();
            }
        } else if let Some(rest) = line.strip_prefix("TTITLE") {
            if let Some(eqpos) = rest.find('=') {
                let title = &rest[eqpos + 1..];
                tracks.push(title.trim().to_string());
            }
        } else if line.trim() == "." {
            break; // End of entry
        }
    }

    // Ensure track list matches expected count, padding if necessary.
    if tracks.len() < ntracks {
        for i in (tracks.len())..ntracks {
            tracks.push(format!("Track {}", i + 1));
        }
    }

    Ok(CddbInfo {
        title: album,
        artist,
        tracks,
        disc_id: disc_id.to_string(),
    })
}