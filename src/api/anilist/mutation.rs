// Mutations

// =============== Imports ================
use crate::api::anilist::user_fetch;

use log;
use reqwest::Client;
use serde_json::json;
use std::process;
use anyhow::Result;

// Constant variables
const ANILIST_API_URL: &str = "https://graphql.anilist.co";
// const ANILIST_CLIENT_ID: &str = "25501";

// Works for both adding and modifying anime statuses
// Takes in an id and a status index
pub async fn update_status(client: &Client, id: i32, status_index: usize) -> Result<()> {
    let option = vec![
        "CURRENT",
        "COMPLETED",
        "PAUSED",
        "DROPPED",
        "PLANNING",
        "REPEATING",
    ];

    let anilist_status = option[status_index];

    let query_string = r#"
        mutation ($mediaId: Int, $status: MediaListStatus) { 
            SaveMediaListEntry(mediaId: $mediaId, status: $status) {
                id
                status
            }
        }
    "#;

    let variables = json!({
        "mediaId": id,
        "status": anilist_status
    });

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", user_fetch::get_token()))
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(response) => {
            let options = vec![
                "Watching",
                "Completed",
                "Paused",
                "Dropped",
                "Planning",
                "Rewatching",
            ];
            if response.status().is_success() {
                println!(
                    "Successfully updated anime status to {}!",
                    options[status_index]
                );
                Ok(())
            } else {
                Err(anyhow::anyhow!("Failed to update status. Status: {}", response.status()))
            }
        }
        Err(e) => {
            Err(anyhow::anyhow!("Error updating anime status: {}", e))
        }
    }
}


// Updates progress of selected anime to selected episode
pub async fn update_progress(client: &Client, anime_id: i32, episode: u32) -> Result<()> {
    let query_string = r#"
        mutation ($mediaId: Int, $progress: Int) { 
            SaveMediaListEntry(mediaId: $mediaId, progress: $progress) {
                id
                progress
            }
        }
    "#;

    let variables = json!({
        "mediaId": anime_id,
        "progress": episode as i32
    });

    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", user_fetch::get_token()))
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;
    match response {
        Ok(res) => {
            if !res.status().is_success() {
                return Err(anyhow::anyhow!("Failed to update progress. Status: {}", res.status()))
            }
        }
        Err(e) => {
            log::error!("Error updating anime progress: {}", e);
            return Err(e.into())
        }
    }
    Ok(())
}

// Updates score to given score of given anime
pub async fn update_score(client: &Client, anime_id: i32, score: f64) -> Result<(), anyhow::Error> {
    let query_string = r#"
        mutation ($mediaId: Int, $score: Float) { 
            SaveMediaListEntry(mediaId: $mediaId, score: $score) {
                id
                score
            }
        }
    "#;

    let variables = serde_json::json!({"score": score, "mediaId": anime_id });
    let response = client
        .post(ANILIST_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", user_fetch::get_token()))
        .json(&json!({
            "query": query_string,
            "variables": variables
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                println!("Successfully updated anime score to {}!", score);
            } else {
                eprintln!("Failed to update score. Status: {}", res.status());
                log::error!("Failed to update score. Status: {}", res.status());
                process::exit(1);
            }
            Ok(())
        }
        Err(e) => {
            if e.is_timeout() {
                log::warn!("Internet connection error");
                Err(anyhow::anyhow!("Internet connection error"))
            } else {
                log::error!("Error updating score: {}", e);
                Err(e.into())
            }
        }
    }
}

