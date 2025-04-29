// Scraping from hungarian popular site: AnimeDrive
// The code is made by me completely

// =============== Imports ================
use crate::local_save::ProgressDatabase;

use regex::Regex;
use reqwest::Client;
use anyhow::{Result, Context};

// Get the direct video link by reusing or scraping the scraper_id
pub async fn get_link(
    client: &Client,
    mal_id: i32,
    anilist_id: i32,
    ep: u32,
    quality: &str,
) -> Result<String> {
    log::info!("Fetching link for MAL ID: {}, Episode: {}", mal_id, ep);
    println!("Getting link from AnimeDrive. This might take a while...");
    log::info!(
        "Getting link from AnimeDrive (hun) for anilist id: {}, episode: {}",
        anilist_id,
        ep
    );
    // Load progress database
    let mut db = ProgressDatabase::load()
        .with_context(|| "Failed to load progress database")?;
    let language = "hungarian";

    // Try to reuse scraper_id if it exists
    let scraper_id = db.get_scraper_id(anilist_id, language);
    if scraper_id.is_none() {
        // Scrape new scraper_id and save it
        log::info!("Scraper ID not found in database, scraping new one...");
        let id = get_scraper_id(client, mal_id)
            .await
            .with_context(|| format!("Failed to scrape scraper ID for MAL ID: {}", mal_id))?;

        // Update or add the entry with the correct scraper_id for the language
        db.update_or_add(anilist_id, ep, 0.0, language, &id);
        db.save().ok();
    }
    let scraper_id = db.get_scraper_id(anilist_id, language).unwrap();

    // Construct player URL using cached/scraped ID
    let player_url = format!(
        "https://player.animedrive.hu/player_wee.php?id={}&ep={}",
        scraper_id, ep
    );
    log::info!("Player URL: {}", player_url);

    let html = get_html(client, &player_url)
        .await
        .with_context(|| format!("Failed to fetch HTML from player URL: {}", player_url))?;
    let link = extract_video_link(&html, quality);
    if link.is_err() {
        eprintln!("Failed to extract video link.");
        return Err(link.unwrap_err());
    }
    else {
        log::info!("Successfully fetched link for Episode: {}", ep);
        return Ok(link.unwrap());
    }
}

// Scrapes the scraper_id (AnimeDrive internal ID) using MAL ID
async fn get_scraper_id(client: &Client, mal_id: i32) -> Result<String> {
    log::info!("Scraping scraper ID for MAL ID: {}", mal_id);
    let mal_link = format!("https://myanimelist.net/anime/{}", mal_id);
    log::info!("Searching for AnimeDrive ID with MAL link: {}", mal_link);
    let search_url = format!("https://animedrive.hu/search/?q={}", mal_link);

    let response = client.get(&search_url).send().await
        .with_context(|| format!("Failed to send request to AnimeDrive: {}", search_url))?;
    let final_url = response.url().to_string();

    let id = final_url.split("id=").nth(1).unwrap().split('&').next();

    if id == None {
        eprintln!("Anime not found on AnimeDrive. MAL id: {}", mal_id);
        log::error!("Anime not found on AnimeDrive. MAL id: {}", mal_id);
        return Err(anyhow::anyhow!("Anime not found"));
    }

    log::info!("Successfully scraped scraper ID for MAL ID: {}", mal_id);
    Ok(id.unwrap().to_string())
}

async fn get_html(client: &Client, player_link: &str) -> Result<String> {
    let response = client.get(player_link).send().await
        .with_context(|| format!("Failed to send request to player link: {}", player_link))?;
    Ok(response.text().await?)
}

// Extracts video link with preferred or best quality
fn extract_video_link(js_code: &str, quality: &str) -> Result<String> {
    let re = Regex::new(r#"src:\s*'([^']+)'.*?size:\s*(\d+)"#).unwrap();
    let mut best = None;
    let mut preferred = None;

    for cap in re.captures_iter(js_code) {
        let url = cap.get(1).unwrap().as_str();
        let size_str = cap.get(2).unwrap().as_str();
        let size: u32 = size_str.parse().ok().unwrap();

        if size_str == quality {
            preferred = Some(url.to_string());
        }

        if best.as_ref().map_or(true, |(_, s)| size > *s) {
            best = Some((url.to_string(), size));
        }
    }

    Ok(preferred.or_else(|| best.map(|(url, _)| url)).unwrap_or("".to_string()))
}
