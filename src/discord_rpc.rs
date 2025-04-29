// =============== Imports ================
use crate::api::anilist::fetch::AnimeData;

use anyhow::Result;
use discord_rpc_client::{self, models::Activity, Client};

const APP_ID: u64 = 1359438420304334929;

pub fn init() -> Result<discord_rpc_client::Client> {
    let client = discord_rpc_client::Client::new(APP_ID);

    Ok(client)
}

pub fn selecting(client: &Client, state: &str, detail: &str) -> () {
    let mut client = client.clone();
    let state = state.to_string();
    let detail = detail.to_string();
    tokio::task::spawn(async move {
        client.set_activity(|_| Activity::new().state(state.to_string()).details(detail.to_string())).unwrap();
    });
}

pub fn payload(
    data: &AnimeData,
    progress: u32,
    max_ep: u32,
    time_pos: u64
) -> Activity {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()-time_pos;
    
    let activity = Activity::new()
        .state(format!("{} - Episode ", data.title))
        .assets(|assets| assets
            .large_image(data.large_pic.as_ref().unwrap())
        )
        .party(|a| a.size((progress, max_ep)))
        .timestamps(|t| t
            .start(now));

    activity
}

pub fn paused_payload(data: &AnimeData, progress: u32, max_ep: u32) -> Activity {
    let activity = Activity::new()
        .state(format!("{} - Episode ", data.title))
        .details("Paused")
        .assets(|assets| assets
            .large_image(data.large_pic.as_ref().unwrap())
        )
        .party(|a| a.size((progress, max_ep)));
    
    activity
}