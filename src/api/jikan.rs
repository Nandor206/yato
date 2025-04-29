// =============== Imports ================
use reqwest::{self, Client};
use serde_json;
use serde::Deserialize;
use anyhow::{Context, Result};

#[derive(Deserialize)]
struct EpisodeWrapper {
    data: JikanData,
}
#[derive(Deserialize)]
struct JikanData {
    filler: bool,
}

pub async fn filler(client: &Client, mal_id: i32, episode: u32) -> Result<bool> {
    let base_url = "https://api.jikan.moe/v4/anime";
    let url = format!("{}/{}/episodes/{}", base_url, mal_id, episode);

    let response = client.get(url.clone()).send().await
        .with_context(|| format!("Failed to send request to Jikan API: {}", url));

    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.text().await {
                    Ok(res) => {
                        let json = res;
                        let data: EpisodeWrapper = serde_json::from_str(&json)
                            .with_context(|| "Failed to parse JSON response from Jikan API")?;
                        Ok(data.data.filler)
                    }
                    Err(err) => {
                        eprintln!("Failed to read response body: {}", err);
                        Ok(false)
                    },
                }
            }
            else {
                log::error!("Error getting episode data from Jikan. Status: {}", res.status());
                eprintln!("Couldn't get filler data.");
                Ok(false)
            }
        }
        Err(e) => {
            log::error!("Error fetching data from Jikan: {}", e);
            eprintln!("Error fetching data from Jikan: {}", e);
            Ok(false)
        }
    }
}