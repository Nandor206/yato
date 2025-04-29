// =============== Imports ================
use anyhow::{Context, Result};
use log;
use reqwest::Client;
use simplelog;
use std::fs;
use std::fs::File;


pub fn init_log() -> Result<()> {
    let log_file = dirs::data_local_dir().unwrap().join("yato/debug.log");
    let log_config = simplelog::ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();

    // Creates file if doesn't exists
    if !log_file.exists() {
        fs::create_dir_all(&log_file.parent().unwrap())
            .context("Failed to create config directory")?;
        File::create_new(&log_file)?;
    }
    let log_file_handle = File::create(log_file).context("Failed to create log file")?;
    simplelog::WriteLogger::init(log::LevelFilter::Trace, log_config, log_file_handle)
        .context("Failed to initialize logger")?;

    Ok(())
}

// Clearing screen
pub fn clear() -> () {
    let _ = console::Term::stdout().clear_screen();
}

// Check if network is available
pub async fn check_network(client: &Client) -> Result<()> {
    let url = "https://www.google.com";
    let response = client.get(url).send().await;
    match response {
        Ok(_) => Ok(()), // Network is available
        Err(e) => {
            log::error!("Network is not available: {}", e);
            Err(anyhow::anyhow!("Network is not available: {}", e))
        }
    }
}
