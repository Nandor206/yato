// Getting user related data

// =============== Imports ================
use crate::skip_override;
use crate::theme;
use crate::utils;

use log;
use dialoguer::{FuzzySelect, Input};
use reqwest::Client;
use serde_json::{Value, json};
use std::{fs, process};
use anyhow::{Result, Context};

// Constant variables
const ANILIST_API_URL: &str = "https://graphql.anilist.co";
const ANILIST_CLIENT_ID: &str = "25501";

// Checks if token is valid
pub async fn check_credentials(client: &Client) -> Result<()> {
    log::info!("Checking AniList credentials");
    let auth = get_token();

    let query = r#"
        query {
            Viewer {
                id
                name
            }
        }
    "#;

    let body = serde_json::json!({
        "query": query
    });

    let response = client
        .post(ANILIST_API_URL)
        .header("Authorization", format!("Bearer {}", auth))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .with_context(|| "Failed to send request to AniList API")?;

    if response.status().is_success() {
        let data: Value = response.json().await
            .with_context(|| "Failed to parse JSON response from AniList API")?;
        
        if let Some(user_id) = data["data"]["Viewer"]["id"].as_i64() {
            let client_file = dirs::data_local_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get local data directory"))?
                .join("yato/anilist_user_id");
            
            // Write the user ID to the file
            fs::write(&client_file, user_id.to_string())
                .with_context(|| format!("Failed to write client ID to file: {:?}", client_file))?;
            
            log::info!("AniList credentials verified successfully");
            Ok(())
        } else {
            log::error!("Invalid or expired token");
            remove_token_file()?;
            Err(anyhow::anyhow!("Invalid or expired token"))
        }
    } else {
        
        log::error!("Token check failed");
        remove_token_file()?;
        Err(anyhow::anyhow!("Token check failed"))
    }
}

// Retrieves the token from the file
pub fn get_token() -> String {
    let data_dir = dirs::data_local_dir().unwrap().join("yato");
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).unwrap();
    }
    let auth_file = data_dir.join("anilist_token");
    if auth_file.exists() {
        // remove the token file if it is empty
        if auth_file.metadata().unwrap().len() == 0 {
            fs::remove_file(&auth_file).unwrap();
            println!("Token file is empty. Please enter it again.");
            log::warn!("Token file is empty, deleting file for new token.");
            process::exit(1);
        }
        // Read the auth code from the file
        let token = std::fs::read_to_string(auth_file).unwrap();
        return token;
    } else {
        // Create the auth code file
        std::fs::File::create(&auth_file).unwrap();
        let prompt = format!(
            "Please enter your Anilist access token generated here:\nhttps://anilist.co/api/v2/oauth/authorize?client_id={}&response_type=token",
            ANILIST_CLIENT_ID
        );
        let token: String = Input::new().with_prompt(prompt).interact().unwrap();
        fs::write(auth_file, &token).unwrap();
        return token;
    }
}

// Removes the token file (used in checking credentials)
pub fn remove_token_file() -> Result<()> {
    let auth_file = dirs::config_dir().unwrap().join("yato/anilist_token");
    if auth_file.exists() {
        fs::remove_file(&auth_file)?;
    }
    Ok(())
}

// Retrieves the user ID from the file
pub fn get_id() -> Result<i32> {
    let client_file = dirs::data_local_dir().unwrap().join("yato/anilist_user_id");
    let id: i32 = fs::read_to_string(client_file)?.parse()?;
    return Ok(id);
}

// Lists all anime from users list (every single one) and returns one chosen one
// Val is for different print outs
pub async fn list_all(client: &Client, val: u8) -> Result<i32> {
    let user_id: i32 = get_id()?;
    let query_string = r#"
        query ($userId: Int) {
            MediaListCollection(userId: $userId, type: ANIME) {
                lists {
                    name
                    entries {
                        score
                        status
                        media {
                            id
                            title {
                                romaji
                                english
                            }
                        }
                    }
                }
            }
        }
    "#;

    let variables = serde_json::json!({"userId": user_id as i32});
    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", get_token()))
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let data: Value = res.json().await.expect("Failed to parse response");

                let lists = data["data"]["MediaListCollection"]["lists"]
                    .as_array()
                    .expect("Expected 'lists' to be an array");

                let mut anime_list = Vec::new();
                for list in lists {
                    if let Some(entries) = list["entries"].as_array() {
                        for entry in entries {
                            anime_list.push(entry);
                        }
                    }
                }

                if anime_list.is_empty() {
                    println!("Start watching something new");
                    process::exit(0);
                }

                let options: Vec<String> = anime_list
                    .iter()
                    .map(|anime| {
                        let media = &anime["media"];
                        let title = media["title"]["english"]
                            .as_str()
                            .or_else(|| media["title"]["romaji"].as_str())
                            .unwrap_or("Unknown Title")
                            .to_string();

                        let status = &anime["status"]
                            .as_str()
                            .unwrap_or("Unknown Status")
                            .to_string();

                        let mut score = anime["score"]
                            .as_u64()
                            .map(|s| s.to_string())
                            .unwrap_or("?".to_string());

                        if score == "0" {
                            score = "Not yet scored".to_string();
                        }

                        let id = anime["media"]["id"].as_i64().unwrap_or(0) as i32;
                        let override_setting = skip_override::search(id);
                        let intro = override_setting.intro;
                        let outro = override_setting.outro;
                        let recap = override_setting.recap;

                        match val {
                            0 => format!("{} - Current status: {}", title, status), // For status updating
                            1 => format!("{} - Score: {}", title, score), // For score updating
                            2 => format!("{} - intro: {} | outro: {} | recap: {}", title, intro, outro, recap), // For override updating
                            _ => format!("{}", title), // For everything else
                        }
                    })
                    .collect();

                let theme = theme::CustomTheme {};
                let selected_index = FuzzySelect::with_theme(&theme)
                    .with_prompt("Choose an anime:")
                    .items(&options)
                    .default(0)
                    .clear(true)
                    .interact_opt()
                    .unwrap();
                utils::clear();
                if let Some(index) = selected_index {
                    Ok(
                        anime_list[index]["media"]["id"]
                            .as_i64()
                            .expect("No ID found") as i32,
                    )
                } else {
                    println!("See you later!");
                    process::exit(0);
                }
            } else {
                log::error!("Failed to fetch data: {}", res.status());
                Err(anyhow::anyhow!("Failed to fetch data: {}", res.status()))
            }
        }
        Err(e) => {
            if e.is_timeout() {
                log::warn!("Internet connection error");
                Err(anyhow::anyhow!("Internet connection error"))
            } else {
                eprintln!("Error retrieving data: {}", e);
                log::error!("Error retrieving data: {}", e);
                Err(e.into())
            }
        }
    }
}

// Gets list of both CURRENT and REPEATING
// Returns id, progress, episode count, anime name of selected anime
pub struct AnimeData {
    pub id: i32,
    pub progress: u32,
    pub episodes: u32,
    pub title: String,
}
impl AnimeData {
    fn new(id: i32, progress: u32, episodes: u32, title: String) -> Self {
        Self {
            id,
            progress,
            episodes,
            title,
        }
    }
}
pub async fn current(client: &Client) -> Result<AnimeData> {
    let user_id: i32 = get_id()?;
    let query_string = r#"
        query ($userId: Int) {
            MediaListCollection(userId: $userId, type: ANIME, status_in: [CURRENT, REPEATING]) {
                lists {
                    name
                    entries {
                        media {
                            id
                            title {
                                romaji
                                english
                            }
                            status
                            episodes
                        }
                        progress
                    }
                }
            }
        }
    "#;

    let variables = serde_json::json!({"userId": user_id });
    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", get_token()))
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let data: Value = res.json().await.expect("Failed to parse response");

                let lists = data["data"]["MediaListCollection"]["lists"]
                    .as_array()
                    .expect("Expected 'lists' to be an array");

                let mut anime_list = Vec::new();
                for list in lists {
                    if let Some(entries) = list["entries"].as_array() {
                        for entry in entries {
                            anime_list.push(entry);
                        }
                    }
                }

                if anime_list.is_empty() {
                    println!("Start watching something new");
                    process::exit(0);
                }

                let options: Vec<String> = anime_list
                    .iter()
                    .map(|anime| {
                        let media = &anime["media"];
                        let title = media["title"]["english"]
                            .as_str()
                            .or_else(|| media["title"]["romaji"].as_str())
                            .unwrap_or("Unknown Title")
                            .to_string();

                        let progress = anime["progress"]
                            .as_u64()
                            .map(|p| p.to_string())
                            .unwrap_or("0".to_string());

                        let episodes = media["episodes"]
                            .as_u64()
                            .map(|e| e.to_string())
                            .unwrap_or("?".to_string());

                        format!("{} - {}|{}", title, progress, episodes)
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
                    let id = anime_list[index]["media"]["id"]
                    .as_i64()
                    .expect("No ID found") as i32;
                    let progress = anime_list[index]["progress"]
                        .as_u64()
                        .expect("No progress found") as u32;
                    let episodes = anime_list[index]["media"]["episodes"]
                        .as_u64()
                        .expect("No episodes found") as u32;
                    let name = anime_list[index]["media"]["title"]["english"]
                        .as_str()
                        .or_else(|| anime_list[index]["media"]["title"]["romaji"].as_str())
                        .unwrap_or("Unknown Title")
                        .to_string();
                    Ok(
                        AnimeData::new(id, progress, episodes, name)
                    )
                } else {
                    println!("See you later!");
                    process::exit(0);
                }
            } else {
                log::error!("Failed to fetch data: {}", res.status());
                Err(anyhow::anyhow!("Failed to fetch data: {}", res.status()))
            }
        }
        Err(e) => {
            if e.is_timeout() {
                eprintln!(
                    "Check internet connection, there might be a problem. The request to the anilist API took too long."
                );
                log::warn!("Internet connection error");
            } else {
                eprintln!("Error retrieving data: {}", e);
                log::error!("Error retrieving data: {}", e);
            }
            Err(e.into())
        }
    }
}