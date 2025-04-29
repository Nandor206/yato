// Scraping from AllAnime
// ! Source of the original code: https://github.com/Wraient/curd
// * I just translated it to Rust
#![allow(unused)] // * There are some unused variables in structs that are just needed for deserialization

// =============== Imports ================
use crate::local_save::ProgressDatabase;
use crate::{theme, utils};

use anyhow::{Context, Result};
use dialoguer::FuzzySelect;
use regex::Regex;
use reqwest::{header::{REFERER, USER_AGENT}, Client};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, process};
use m3u8_rs::{parse_playlist_res, Playlist};

// const ALLANIME_BASE: &str = "allanime.day";  //It's not used in the code, because I can't format a constant
const ALLANIME_API: &str = "https://api.allanime.day/api";
const AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";
const ALLANIME_REF: &str = "https://allanime.to";

pub async fn get_link(
    client: &Client,
    language: &str,
    anilist_id: i32,
    episode: u32,
    quality: &str,
    sub_or_dub: &str,
    name: &str,
) -> Result<String> {
    log::info!(
        "Getting link from AllAnime for anilist id: {}, episode: {}",
        anilist_id,
        episode
    );

    // Load progress database
    let mut db = ProgressDatabase::load().with_context(|| "Failed to load progress database")?;

    // Try to reuse scraper_id if it exists
    let scraper_id = db.get_scraper_id(anilist_id, language);
    if scraper_id.is_none() {
        log::info!("Scraper ID not found in database, scraping new one...");
        let id = get_scraper_id(&client, name, sub_or_dub)
            .await
            .with_context(|| format!("Failed to scrape scraper ID for name: {}", name))?;

        db.update_or_add(anilist_id, episode, 0.0, language, &id);
        db.save().ok();
    }
    let scraper_id = db.get_scraper_id(anilist_id, language).unwrap();

    let available_episodes = episodes_list(&client, scraper_id, sub_or_dub).await?;

    if available_episodes.is_empty() {
        eprintln!("No episodes found for the given anime.");
        println!("Please try again later.");
        return Err(anyhow::anyhow!("No episodes found"));
    } else if !available_episodes.contains(&episode.to_string()) {
        eprintln!(
            "Episode {} not found in the list of available episodes.",
            episode
        );
        println!("Please try again later.");
        return Err(anyhow::anyhow!("Episode not found"));
    }

    // Getting the direct video link
    let url = get_video_link(&client, scraper_id, episode, quality, sub_or_dub).await;
    if url.is_err() {
        eprintln!("Failed to get the video link.");
        return Err(url.unwrap_err());
    } else {
        log::info!("Video link found for: {}, episode: {}", name, episode);
    }
    let url = url?;
    Ok(url)
}

// ================ Search for AllAnime ID ================
#[derive(Deserialize)]
struct GraphQLResponse {
    data: SearchData,
}

#[derive(Deserialize)]
struct SearchData {
    shows: ShowsData,
}

#[derive(Deserialize)]
struct ShowsData {
    edges: Vec<AnimeShow>,
}

#[derive(Deserialize)]
struct AnimeShow {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    #[serde(rename = "englishName")]
    english_name: Option<String>,
    #[serde(rename = "availableEpisodes")]
    available_episodes: serde_json::Value,
    #[serde(rename = "__typename")]
    typename: String,
}

async fn get_scraper_id(client: &Client, name: &str, mode: &str) -> Result<String> {
    let query = r#"
        query($search: SearchInput, $limit: Int, $page: Int, $translationType: VaildTranslationTypeEnumType, $countryOrigin: VaildCountryOriginEnumType) {
            shows(search: $search, limit: $limit, page: $page, translationType: $translationType, countryOrigin: $countryOrigin) {
                edges {
                    _id
                    name
                    englishName
                    availableEpisodes
                    __typename
                }
            }
        }
    "#;

    let variables = json!({
        "search": {
            "allowAdult": false,
            "allowUnknown": false,
            "query": name,
        },
        "limit": 40,
        "page": 1,
        "translationType": mode,
        "countryOrigin": "ALL"
    })
    .to_string();

    let query_encoded = urlencoding::encode(&query);
    let variables_encoded = urlencoding::encode(&variables);
    let url = format!(
        "{}?query={}&variables={}",
        ALLANIME_API, query_encoded, variables_encoded
    );

    let response = client
        .get(&url)
        .header(USER_AGENT, AGENT)
        .header(REFERER, ALLANIME_REF)
        .send()
        .await
        .with_context(|| format!("Failed to send request to AllAnime API: {}", url))?;

    let body = response.text().await.expect("Failed to read response");
    let parsed: GraphQLResponse = serde_json::from_str(&body)
        .with_context(|| "Failed to parse JSON response from AllAnime API")?;

    let mut results = vec![];

    for anime in parsed.data.shows.edges {
        let display_name = anime.english_name.clone().unwrap_or(anime.name.clone());
        results.push((anime.id, display_name));
    }

    let titles: Vec<String> = results.iter().map(|(_, title)| title.clone()).collect();

    // Letting the user select the correct anime
    let theme = theme::CustomTheme {};
    let selection = FuzzySelect::with_theme(&theme)
        .with_prompt("Choose the correct anime:")
        .items(&titles)
        .default(0)
        .interact_opt()
        .expect("Failed to select anime");
    utils::clear();

    if selection.is_none() {
        return Err(anyhow::anyhow!("No selection was made"))
    }
    let (anime_id, _anime_name) = results[selection.unwrap()].clone();

    Ok(anime_id)
}

// ================ Fetch episode list ===============

#[derive(Deserialize)]
struct EpisodesResponse {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    show: Show,
}

#[derive(Deserialize)]
struct Show {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "availableEpisodesDetail")]
    available_episodes_detail: HashMap<String, serde_json::Value>,
}

async fn episodes_list(
    client: &Client, 
    allanime_id: &str, 
    mode: &str
) -> Result<Vec<String>> {
    let query =
        r#"query ($showId: String!) {
        show( _id: $showId ) {
            _id availableEpisodesDetail
            }
        }"#;
    let encoded_query = urlencoding::encode(query);
    let variables = format!(r#"{{"showId":"{}"}}"#, allanime_id);
    let encoded_variables = urlencoding::encode(&variables);

    let url = format!(
        "{}?variables={}&query={}",
        ALLANIME_API, encoded_variables, encoded_query
    );

    let response = client
        .get(&url)
        .header(USER_AGENT, AGENT)
        .header(REFERER, ALLANIME_REF)
        .send()
        .await
        .with_context(|| format!("Failed to send request to fetch episode list: {}", url))?
        .text()
        .await?;

    let parsed: EpisodesResponse = serde_json::from_str(&response)
        .with_context(|| "Failed to parse JSON response for episode list")?;

    Ok(extract_episodes(
        &parsed.data.show.available_episodes_detail,
        mode,
    ))
}

fn extract_episodes(
    available_episodes_detail: &HashMap<String, serde_json::Value>,
    mode: &str,
) -> Vec<String> {
    let mut episodes = vec![];

    if let Some(eps) = available_episodes_detail
        .get(mode)
        .and_then(|v| v.as_array())
    {
        for ep in eps {
            if let Some(ep_str) = ep.as_str() {
                episodes.push(ep_str.to_string());
            }
        }
    }

    episodes.sort_by(|a, b| {
        let a_num = a.parse::<f64>().unwrap_or(0.0);
        let b_num = b.parse::<f64>().unwrap_or(0.0);
        a_num.partial_cmp(&b_num).unwrap()
    });

    episodes
}

// ================ Fetch video links ===============

fn decode_provider_id(encoded: &str) -> String {
    let re = Regex::new("..").unwrap();
    let pairs: Vec<String> = re
        .find_iter(encoded)
        .map(|m| m.as_str().to_string())
        .collect();

    let replacements: HashMap<&str, &str> = [
        ("01", "9"),("08", "0"),("05", "="),("0a", "2"),("0b", "3"),
        ("0c", "4"),("07", "?"),("00", "8"),("5c", "d"),("0f", "7"),
        ("5e", "f"),("17", "/"),("54", "l"),("09", "1"),("48", "p"),
        ("4f", "w"),("0e", "6"),("5b", "c"),("5d", "e"),("0d", "5"),
        ("53", "k"),("1e", "&"),("5a", "b"),("59", "a"),("4a", "r"),
        ("4c", "t"),("4e", "v"),("57", "o"),("51", "i"),
    ]
    .iter()
    .cloned()
    .collect();

    let mut decoded = pairs
        .into_iter()
        .map(|pair| replacements.get(&*pair).unwrap_or(&&*pair).to_string())
        .collect::<Vec<_>>()
        .join("");

    decoded = decoded.replace("/clock", "/clock.json");
    decoded
}

async fn extract_links(client: &Client, provider_id: &str) -> Result<Value> {
    let url = format!("https://allanime.day{}", provider_id);
    let res = client
        .get(url)
        .header(REFERER, ALLANIME_REF)
        .header(USER_AGENT, AGENT)
        .send().await.with_context(|| {
            format!("Failed to send request to fetch links from provider ID: {}", provider_id)
        })?;

    let text = res.text().await.with_context(|| "Failed to read response text")?;
    let links_json = serde_json::from_str::<Value>(&text)
        .with_context(|| "Failed to parse JSON response for links")?;

    Ok(links_json)
}

#[derive(Debug, Deserialize)]
struct EpisodeResponse {
    data: EpisodeData,
}

#[derive(Debug, Deserialize)]
struct EpisodeData {
    episode: Episode,
}

#[derive(Debug, Deserialize)]
struct Episode {
    #[serde(rename = "sourceUrls")]
    source_urls: Vec<SourceUrl>,
}

#[derive(Debug, Deserialize)]
struct SourceUrl {
    #[serde(rename = "sourceUrl")]
    source_url: String,
}

async fn get_episode_url(
    client: &Client,
    show_id: &str,
    ep_no: u32,
    translation_type: &str,
) -> Result<Vec<String>> {
    let query = r#"
    query($showId:String!,$translationType:VaildTranslationTypeEnumType!,$episodeString:String!) {
    episode(showId:$showId,translationType:$translationType,episodeString:$episodeString)
        {
        episodeString sourceUrls
        }
    }"#;

    let variables = json!({
        "showId": show_id,
        "translationType": translation_type,
        "episodeString": ep_no.to_string(),
    });

    let url = format!(
        "https://api.allanime.day/api?query={}&variables={}",
        urlencoding::encode(query),
        urlencoding::encode(&variables.to_string())
    );

    let resp = client
        .get(&url)
        .header(USER_AGENT, AGENT)
        .header(REFERER, ALLANIME_REF)
        .send().await
        .with_context(|| format!("Failed to send request to fetch episode URLs: {}", url))?;

    let json_resp: EpisodeResponse = resp.json().await
        .with_context(|| "Failed to parse JSON response for episode URLs")?;
    let mut valid_links = vec![];

    for src in json_resp.data.episode.source_urls {
        if src.source_url.len() > 2 {
            let decoded = decode_provider_id(&src.source_url[2..]);
            if decoded.contains("clock.json") {
                if let Ok(links_json) = extract_links(&client, &decoded).await {
                    if let Some(links) = links_json.get("links").and_then(|v| v.as_array()) {
                        for link in links {
                            if let Some(link_str) = link.get("link").and_then(|l| l.as_str()) {
                                valid_links.push(link_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    if valid_links.is_empty() {
        return Err(anyhow::anyhow!("No valid links found"));
    }

    Ok(valid_links)
}

// ================ Get video link ===============

async fn get_video_link(
    client: &Client,
    show_id: &str,
    ep_no: u32,
    quality: &str,
    translation_type: &str,
) -> Result<String> {
    let  link_priorities: Vec<&str> = vec![
        "sharepoint.com",
        "wixmp.com",
        "dropbox.com",
        "wetransfer.com",
        "gogoanime.com",
        // Add more domains in order of priority
    ];

    let episode_urls = get_episode_url(client, show_id, ep_no, translation_type).await?;

    let video_link = match get_priority_link(link_priorities, episode_urls) {
        Some(link) => link,
        None => return Err(anyhow::anyhow!("No valid video link found"))
    };

    if video_link.contains(".m3u8") {
        if quality == "best" {
            return Ok(video_link); // let mpv do it's part
        } else {
            get_resolution_link(&client, &video_link, quality).await
                .with_context(|| format!("Failed to get resolution link for: {}", video_link))
                .map_err(|e| {
                    log::error!("Error getting resolution link: {:?}", e);
                    anyhow::anyhow!("Failed to get resolution link")
                })
        }
    }
    else {
        Ok(video_link)
    }
    
}

// =============== Get priority link ===============

// Prioritize m3u8 links and then check for other links
fn get_priority_link(priorities: Vec<&str>, links: Vec<String>) -> Option<String> {
    if links.is_empty() {
        return None;
    }
    
    // Create a map for quick lookup of priorities
    // Higher index means higher priority
    let priority_map: HashMap<&str, usize> = priorities
        .iter()
        .enumerate()
        .map(|(i, domain)| (*domain, priorities.len() - i))
        .collect();
    
    // First, check specifically for m3u8 links
    let mut m3u8_links = Vec::new();
    for link in &links {
        if link.contains(".m3u8") {
            m3u8_links.push(link.clone());
        }
    }
    
    // If we found m3u8 links, prioritize them
    if !m3u8_links.is_empty() {
        // Apply domain priorities within m3u8 links
        let mut highest_priority = 0;
        let mut best_link = None;
        
        for link in &m3u8_links {
            for (domain, priority) in &priority_map {
                if link.contains(domain) {
                    if *priority > highest_priority {
                        highest_priority = *priority;
                        best_link = Some(link.clone());
                    }
                    break;
                }
            }
        }
        
        // If we found a priority m3u8 link, return it
        // Otherwise return the first m3u8 link
        return best_link.or_else(|| m3u8_links.first().cloned());
    }
    
    // If no m3u8 links, fall back to regular priority logic
    let mut highest_priority = 0;
    let mut best_link = None;
    
    for link in &links {
        for (domain, priority) in &priority_map {
            if link.contains(domain) {
                if *priority > highest_priority {
                    highest_priority = *priority;
                    best_link = Some(link.clone());
                }
                break;
            }
        }
    }
    
    // If no priority link found, return the first link
    best_link.or_else(|| links.first().cloned())
}

// ================ Get resolution link from m3u8 link ===============

async fn get_resolution_link(client: &Client, m3u8_url: &str, target_resolution: &str) -> Result<String> {
    // Fetch the m3u8 content
    let response = client.get(m3u8_url).send().await?;
    let m3u8_content = response.bytes().await?;
    
    // Parse the playlist
    let playlist = parse_playlist_res(&m3u8_content[..]).map_err(|e| anyhow::anyhow!("Failed to parse m3u8: {:?}", e))?;
    
    match playlist {
        Playlist::MasterPlaylist(master) => {
            // Find the target resolution in the variants
            for variant in master.variants {
                if let Some(resolution) = variant.resolution {
                    if resolution.height == target_resolution.parse::<u64>().unwrap_or(0) {
                        // Return the URI of the selected variant
                        return Ok(variant.uri);
                    }
                }
            }
            // If resolution is not found, return the m3u8 link (let mpv decide)
            return Ok(m3u8_url.to_string());
        },
        m3u8_rs::Playlist::MediaPlaylist(_) => {
            // This is already a media playlist (single resolution)
            // Just return the original URL
            Ok(m3u8_url.to_string())
        }
    }
    
}