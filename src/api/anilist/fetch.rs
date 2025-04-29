// Getting information from Anilist

// =============== Imports ================
use crate::config;
use crate::theme;
use crate::utils;

use log;
use console::{self, Style};
use dialoguer::FuzzySelect;
use reqwest::Client;
use serde_json::{Value, json};
use anyhow::Result;

// Constant variables
const ANILIST_API_URL: &str = "https://graphql.anilist.co";
// const ANILIST_CLIENT_ID: &str = "25501";

// Searches by name, returns anilist id if found (and selected)
pub async fn search(client: &Client, input: String) -> Result<i32> {
    log::info!("Searching AniList for: {}", input);
    let adult = config::load_config().show_adult_content;

    let query_string = r#"
        query ($search: String, $isAdult: Boolean) {
            Page {
                media(search: $search, type: ANIME, isAdult: $isAdult) {
                    id
                    title {
                        romaji
                        english
                    }
                }
            }
        }
    "#;

    let variables = json!({ "search": input, "isAdult": adult });

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let data: serde_json::Value = res.json().await.expect("Failed to parse response");
                let anime_list = data["data"]["Page"]["media"]
                    .as_array()
                    .expect("Expected 'media' to be an array");

                if anime_list.is_empty() {
                    return Err(anyhow::anyhow!("No results found"));
                }

                let options: Vec<String> = anime_list
                    .iter()
                    .map(|anime| {
                        anime["title"]["english"]
                            .as_str()
                            .unwrap_or(anime["title"]["romaji"].as_str().unwrap_or("Unknown Title"))
                            .to_string()
                    })
                    .collect();

                let theme = theme::CustomTheme {};

                let selected_index = FuzzySelect::with_theme(&theme)
                    .with_prompt("Choose an anime:")
                    .items(&options)
                    .default(0)
                    .clear(true)
                    .interact_opt()?;

                utils::clear();
                if let Some(index) = selected_index {
                    log::info!("AniList search completed successfully");
                    Ok(anime_list[index]["id"].as_i64().expect("No ID found") as i32)
                } else {
                    Err(anyhow::anyhow!("No selection was made"))
                }
            } else {
                log::warn!("Failed to search. Status code: {}", res.status());
                Err(anyhow::anyhow!("Failed to search. Status code: {}", res.status()))
            }
        }
        Err(e) => {
            if e.is_timeout() {
                eprintln!(
                    "Check your internet connection. The request to AniList API took too long. Error: {}",
                    e
                );
                log::warn!("Internet connection error: {}", e);
            } else {
                eprintln!("Error sending request: {}", e);
                log::error!("Request to AniList API failed: {}", e);
            }
            Err(anyhow::anyhow!("Error sending request: {}", e))
        }
    }
}

// Gets the information of an anime and prints it out in a pretty way
pub async fn information(client: &Client, anime_id: i32) -> Result<()> {
    let query_string = r#"
        query ($id: Int!) {
            Media(id: $id) {
                id
                title {
                    romaji
                    english
                }
                status
                description
                genres
                episodes
                startDate {
                    year
                    month
                    day
                }
                endDate {
                    year
                    month
                    day
                }
            }
        }
    "#;

    let variables = json!({ "id": anime_id });

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let data: serde_json::Value = res.json().await.expect("Failed to parse response");
                let media = &data["data"]["Media"];

                let title = media["title"]["english"]
                    .as_str()
                    .unwrap_or(media["title"]["romaji"].as_str().unwrap_or("Unknown Title"));

                let description = media["description"]
                    .as_str()
                    .unwrap_or("No description available.")
                    .replace("<br><br>", "\n")
                    .replace("<br>", "\n")
                    .replace("<i>", "{")
                    .replace("</i>", "}");

                let genres = media["genres"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|genre| genre.as_str().unwrap_or("Unknown").to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                let episodes = media["episodes"].as_u64().unwrap_or(0) as u32;

                let start_date = format!(
                    "{}-{:02}-{:02}",
                    media["startDate"]["year"].as_u64().unwrap_or(0),
                    media["startDate"]["month"].as_u64().unwrap_or(0),
                    media["startDate"]["day"].as_u64().unwrap_or(0)
                );

                let end_date = format!(
                    "{}-{:02}-{:02}",
                    media["endDate"]["year"].as_u64().unwrap_or(0),
                    media["endDate"]["month"].as_u64().unwrap_or(0),
                    media["endDate"]["day"].as_u64().unwrap_or(0)
                );

                let status = media["status"].as_str().unwrap_or("Unknown Status");

                println!("{}", console::style("Anime Info").bold().underlined());
                let design = Style::new().bold().italic().color256(247);

                println!("{} {}", design.apply_to("Title:"), title);
                println!("{} {}", design.apply_to("Current status:"), status);
                println!(
                    "\n{} {}\n",
                    design.apply_to("Description:"),
                    console::style(description).italic()
                );
                println!("{} {}", design.apply_to("Genres:"), genres);
                println!("{} {}", design.apply_to("Episodes:"), episodes);
                print!(
                    "{} {},",
                    design.apply_to("Start date:"),
                    if start_date == "0-00-00" {
                        "Unknown".to_string()
                    } else {
                        start_date
                    }
                );
                println!(
                    " {} {}",
                    design.apply_to("End date:"),
                    if end_date == "0-00-00" {
                        "Unknown".to_string()
                    } else {
                        end_date
                    }
                );
            } else {
                eprintln!("Failed to fetch anime info. Status code: {}", res.status());
                log::warn!("Failed to fetch anime info. Status code: {}", res.status());
            }
            Ok(())
        }
        Err(e) => {
            if e.is_timeout() {
                log::warn!("Internet connection error");
            } else {
                log::error!("Fetching info failed: {}", e);
            }
            Err(anyhow::anyhow!("Fetching info failed: {}", e))
        }
    }
}

// Converts anilist id to mal id
pub async fn id_converter(client: &Client, id: i32) -> Result<i32> {
    let query_string = r#"
        query ($id: Int) {
            Media(id: $id) {
                id
                idMal
            }
        }
    "#;

    let variables = serde_json::json!({ "id": id });

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    let response = match response {
        Ok(res) => res,
        Err(e) => {
            log::error!("Request error: {}", e);
            return Err(anyhow::anyhow!("Request error: {}", e))
        }
    };

    if !response.status().is_success() {
        log::error!("Failed to convert id, status: {}", response.status());
        return Err(anyhow::anyhow!("Failed to convert id, status: {}", response.status()))
    }

    let data: Value = match response.json().await {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to parse response: {}", e);
            return Err(anyhow::anyhow!("Failed to parse response: {}", e))
        }
    };

    let mal_id = data["data"]["Media"]["idMal"].as_i64().map(|id| id as i32);

    Ok(mal_id.unwrap())
}

// gets anime name and episode count by id
pub struct AnimeData {
    pub title: String,
    pub episodes: u32,
    pub id: i32,
    pub large_pic: Option<String>,
}

pub async fn data_by_id(client: &Client, id: i32) -> Result<AnimeData> {
    let query_string = r#"
        query ($id: Int!) {
            Media(id: $id) {
                id
                title {
                    romaji
                    english
                }
                episodes
                coverImage {
                    large
                    medium
                }
            }
        }
    "#;

    let variables = json!({ "id": id });

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(res) => match res.json::<serde_json::Value>().await {
            Ok(json) => {
                let titles = &json["data"]["Media"]["title"];
                let covers = &json["data"]["Media"]["coverImage"];
                let title = titles["english"].as_str().unwrap_or_else(|| titles["romaji"].as_str().unwrap_or(""));
                let episodes = json["data"]["Media"]["episodes"].as_u64().unwrap_or(0) as u32;
                let large = covers["large"].as_str().unwrap_or("");

                if title.is_empty() {
                    return Err(anyhow::anyhow!("Failed to parse title data"));
                }
                else {
                    return Ok(
                        AnimeData{
                            id: id,
                            episodes: episodes,
                            title: title.to_string(),
                            large_pic: Some(large.to_string()),
                        }
                    );
                }

            }
            Err(e) => {
                log::error!("JSON parsing error: {}", e);
                Err(anyhow::anyhow!("Failed to parse response JSON: {}", e))
            }
        },
        Err(e) => {
            if e.is_timeout() {
                log::warn!("Internet connection error");
                Err(anyhow::anyhow!("Internet connection error"))
            } else {
                log::error!("Error searching for name: {}", e);
                Err(anyhow::anyhow!("Error searching for name: {}", e))
            }
        }
    }
}

// gets sequel data by id
pub async fn sequel_data(client: &Client, id: i32) -> Result<AnimeData> {
    let query_string = r#"query ($id: Int) {
        Media(id: $id, type: ANIME) {
            title {
                romaji
                english
            }
            relations {
                edges {
                    relationType
                        node {
                            id
                            title {
                                romaji
                                english
                            }
                            episodes
                            coverImage {
                                large
                                medium
                            }
                        type
                    }
                }
            }
        }
    }"#;
    let variables = json!({"id": id});

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": query_string,
            "variables": variables,
        }))
        .send()
        .await;

    match response {
        Ok(res) => match res.json::<serde_json::Value>().await {
            Ok(json) => {
                let edges_array = json["data"]["Media"]["relations"]["edges"]
                    .as_array();
                let edges = match edges_array {
                    Some(arr) => arr,
                    None => {
                        return Err(anyhow::anyhow!("Failed to parse edges array"));
                    }
                };

                for edge in edges {
                    let relation_type = edge["relationType"].as_str().unwrap_or("");
                    if relation_type == "SEQUEL" {
                        let node = &edge["node"]["title"];
                        let title = node["english"].as_str().unwrap_or_else(|| node["romaji"].as_str().unwrap_or("<Unknown Title>"));
                        let sequel_id = edge["node"]["id"].as_i64().unwrap() as i32;
                        let episodes = edge["node"]["episodes"].as_u64().unwrap_or(0) as u32;
                        if !title.is_empty() && sequel_id != 0 {
                            return Ok(AnimeData{
                                title: title.to_string(),
                                episodes: episodes,
                                id: sequel_id,
                                large_pic: None,
                            });
                        } else {
                            return Err(anyhow::anyhow!("Failed to parse sequel data"));
                        }

                    }
                }
                Err(anyhow::anyhow!("No sequel found"))
            }
            Err(e) => {
                log::error!("JSON parsing error: {}", e);
                Err(anyhow::anyhow!("Failed to parse response JSON: {}", e))
            }
        },
        Err(e) => {
            if e.is_timeout() {
                log::warn!("Internet connection error");
                Err(anyhow::anyhow!("Internet connection error"))
            } else {
                log::error!("Error getting relations for name: {}", e);
                Err(anyhow::anyhow!("Error getting relations for name: {}", e))
            }
        }
    }
}
