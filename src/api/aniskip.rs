// =============== Imports ================
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{f64, io::Write, os::unix::net::UnixStream};

#[derive(Deserialize, Debug)]
pub struct SkipTimesResponse {
    pub found: bool,
    pub results: Vec<SkipResult>,
}

#[derive(Deserialize, Debug)]
pub struct SkipResult {
    #[serde(rename = "skipType")]
    pub skip_type: String,

    pub interval: SkipInterval,
}

#[derive(Deserialize, Debug)]
pub struct SkipInterval {
    #[serde(rename = "startTime")]
    pub start_time: f64,

    #[serde(rename = "endTime")]
    pub end_time: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Skip {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SkipData {
    pub op: Skip,
    pub ed: Skip,
    pub recap: Skip,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Anime {
    pub episode: u32,
    pub mal_id: i32,
    pub skip_times: SkipData,
}

// Function to get AniSkip data from the API
pub async fn get_ani_skip_data(client: &Client, anime_mal_id: i32, episode: u32) -> Result<String> {
    let base_url = "https://api.aniskip.com/v2/skip-times";
    let url = format!(
        "{}/{}/{}?types=op&types=ed&types=recap&episodeLength=0",
        base_url, anime_mal_id, episode
    );

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to send request to AniSkip API: {}", url))?;

    if response.status().is_success() {
        response
            .text()
            .await
            .with_context(|| "Failed to read response body from AniSkip API")
    } else {
        Err(anyhow::anyhow!(
            "AniSkip API returned an error with status: {}",
            response.status()
        ))
    }
}

// Function to round time to specified precision
pub fn round_time(time_value: f64, precision: usize) -> f64 {
    let multiplier = f64::powf(10.0, precision as f64);
    (time_value * multiplier).round() / multiplier
}

// Function to parse AniSkip API response and update Anime struct
pub fn parse_ani_skip_response(
    response_text: &str,
    anime: &mut Anime,
    time_precision: usize,
) -> Result<()> {
    if response_text.is_empty() {
        return Err(anyhow::anyhow!("Response text is empty"));
    }

    let data: SkipTimesResponse = serde_json::from_str(response_text)
        .with_context(|| "Failed to parse JSON response from AniSkip API")?;

    if !data.found || data.results.is_empty() {
        log::warn!("No skip times found. Data: {:#?}", anime);
        return Err(anyhow::anyhow!(
            "No skip times found in AniSkip API response"
        ));
    }

    for result in data.results {
        let interval = result.interval;
        let skip = Skip {
            start: round_time(interval.start_time, time_precision),
            end: round_time(interval.end_time, time_precision),
        };

        match result.skip_type.as_str() {
            "op" => anime.skip_times.op = skip,
            "ed" => anime.skip_times.ed = skip,
            "recap" => anime.skip_times.recap = skip,
            _ => {}
        }
    }

    log::debug!("Skip times fetched: {:#?}", anime.skip_times);

    Ok(())
}

// Fetch and parse AniSkip data
pub async fn get_and_parse_ani_skip_data(
    client: &Client,
    anime_mal_id: i32,
    episode: u32,
    time_precision: usize,
    anime: &mut Anime,
) -> Result<()> {
    let response_text = get_ani_skip_data(client, anime_mal_id, episode).await?;
    parse_ani_skip_response(&response_text, anime, time_precision)
}

// Send skip times to MPV
pub fn send_skip_times_to_mpv(anime: &Anime) -> Result<()> {
    let chapter_list = vec![
        json!({
            "title": "Title card",
            "time": 0.0
        }),
        json!({
            "title": "Recap",
            "time": anime.skip_times.recap.start
        }),
        json!({
            "title": "Pre-Opening",
            "time": anime.skip_times.recap.end
        }),
        json!({
            "title": "Opening",
            "time": anime.skip_times.op.start
        }),
        json!({
            "title": "Main",
            "time": anime.skip_times.op.end
        }),
        json!({
            "title": "Credits",
            "time": anime.skip_times.ed.start
        }),
        json!({
            "title": "Post-Credits",
            "time": anime.skip_times.ed.end
        }),
    ];


    let mut stream = UnixStream::connect("/tmp/yato-mpvsocket")
        .with_context(|| "Failed to connect to MPV socket")?;

    let cmd = json!({
        "command": ["set_property", "chapter-list", chapter_list]
    });

    let json = serde_json::to_string(&cmd)
        .with_context(|| "Failed to serialize chapter list command to JSON")?;
    writeln!(stream, "{}", json)
        .with_context(|| "Failed to write chapter list command to MPV socket")?;

    log::info!("Sent skip times to MPV");

    Ok(())
}
