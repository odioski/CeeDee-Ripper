use musicbrainz_rs::entity::coverart::Coverart;
use musicbrainz_rs::entity::disc::Disc;
use musicbrainz_rs::prelude::*;

/// Holds metadata fetched from MusicBrainz.
#[derive(Debug, Clone)]
pub struct MusicBrainzInfo {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<String>,
    pub cover_art_url: Option<String>,
}

/// Fetches release metadata and cover art from MusicBrainz using a CD disc ID.
///
/// This function connects to the MusicBrainz API to find a release matching the
/// provided disc ID. If a release is found, it then queries the Cover Art
/// Archive for associated cover art.
///
/// # Arguments
///
/// * `disc_id` - The disc ID calculated from the CD's table of contents.
///
/// # Returns
///
/// A `Result` containing `MusicBrainzInfo` on success, or a boxed error
/// if the lookup fails or no release is found.
pub async fn fetch_musicbrainz_metadata(
    disc_id: &str,
) -> Result<MusicBrainzInfo, Box<dyn std::error::Error>> {
    // It's good practice to set a proper user-agent for API requests.
    // musicbrainz_rs uses a default, but for a real application, you should
    // set one that identifies your app, like:
    // CeeDeeRipper/1.0.0 ( https://example.com/CeeDeeRipper )
    let client = MusicBrainzClient::new();

    // First, perform a discid lookup to find matching releases.
    // We include "releases" and "recordings" to get album and track data.
    let disc: Disc = client
        .fetch()
        .disc_id(disc_id)
        .with("releases+recordings")
        .execute()
        .await?;

    // A disc ID can be associated with multiple releases. We'll pick the first one.
    let release = disc
        .releases
        .and_then(|releases| releases.into_iter().next())
        .ok_or("No releases found for this disc ID")?;

    // With the release MBID, query the Cover Art Archive.
    let cover_art_url = fetch_cover_art(&client, &release.id).await;

    // Assemble the information into our struct.
    let artist = release
        .artist_credit
        .as_ref()
        .map(|credits| {
            credits
                .iter()
                .map(|credit| credit.name.clone())
                .collect::<Vec<_>>()
                .join(" & ")
        })
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let tracks = release
        .media
        .as_ref()
        .and_then(|media| media.first())
        .and_then(|medium| medium.tracks.as_ref())
        .map(|tracks| {
            tracks
                .iter()
                .filter_map(|track| track.title.clone())
                .collect()
        })
        .unwrap_or_else(Vec::new);

    Ok(MusicBrainzInfo {
        title: release.title.unwrap_or_else(|| "Unknown Title".to_string()),
        artist,
        tracks,
        cover_art_url,
    })
}

/// Fetches the front cover art URL for a given release MBID.
async fn fetch_cover_art(client: &MusicBrainzClient, release_mbid: &str) -> Option<String> {
    // The `coverart` fetch is a separate query to the Cover Art Archive API.
    let coverart_result: Result<Coverart, _> =
        client.fetch().coverart(release_mbid).execute().await;

    if let Ok(cover_art) = coverart_result {
        // We look for the "Front" image.
        cover_art
            .images
            .into_iter()
            .find(|img| img.front.unwrap_or(false))
            .map(|img| img.image)
    } else {
        None
    }
}