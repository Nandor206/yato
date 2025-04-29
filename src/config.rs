// =============== Imports ================
use std::path::PathBuf;
use std::fs;
use serde_yaml;
use serde::Deserialize;
use log;
use anyhow::{Context, Result};


#[derive(Deserialize, Debug)]
pub struct Config {
    pub player: String,
    pub player_args: String,
    pub show_adult_content: bool,
    pub sub_or_dub: String,
    pub score_on_completion: bool,
    pub discord_presence: bool,
    pub completion_time: u8,
    pub skip_opening: bool,
    pub skip_credits: bool,
    pub skip_recap: bool,
    pub skip_filler: bool,
    pub quality: String,
    pub language: String,
}
// Default implementation for Config
// This will be used to create a default config file if it doesn't exist
impl Default for Config {
    fn default() -> Self {
        Config {
            player: "mpv".to_string(),
            player_args: "".to_string(),
            show_adult_content: false,
            sub_or_dub: "sub".to_string(),
            score_on_completion: false,
            discord_presence: false,
            completion_time: 85,
            skip_opening: true,
            skip_credits: true,
            skip_recap: true,
            skip_filler: true,
            quality: "best".to_string(),
            language: "english".to_string(),
        }
    }
}

// Create the config file if it doesn't exist
pub fn create(config_file: &PathBuf) -> () {
    let default_config: &str =
r#"#Please do not remove any setting, because it will break the app, just leave it as is.

player: "mpv"
player_args: ""
# Player arguments, you can add any argument here. For example: "--no-cache --fullscreen=yes"
show_adult_content: false

score_on_completion: false
completion_time: 85
# You can change this to any number between 0 and 100.

skip_opening: true
skip_credits: true
skip_recap: true
skip_filler: true

quality: "best"
# You can change this to any other quality. If desired quality is not available, the app will choose the best available quality.

language: "english"
# Supported languages rn: hungarian, english. Hungarian uses a custom scraper for links (made by me)
sub_or_dub: "sub"
# This setting is currently only available for english. Needs to be "sub" or "dub"

discord_presence: false "#;

    fs::create_dir_all(config_file.parent().unwrap()).expect("Failed to create config directory");
    fs::write(&config_file, default_config).expect("Failed to create config file");
    println!("Config file created at: {}", config_file.display());
    log::trace!("Config file has been created with default values");
}
// This function loads the config file from the config directory
// If the config file doesn't exist, returns the default config
pub fn load_config() -> Config {
    log::info!("Loading configuration");
    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        log::error!("Couldn't find config directory!");
        panic!("Could not find config directory. Please set the XDG_CONFIG_HOME environment variable.");
    });
    let config_path = config_dir.join("yato");
    if !config_path.exists() {
        std::fs::create_dir_all(&config_path).expect("Failed to create config directory");
    }
    let config_file = config_path.join("yato.conf");
    if let Ok(contents) = fs::read_to_string(&config_file) {
        let config: Config = serde_yaml::from_str(&contents)
            .with_context(|| "Failed to parse config file")
            .expect("Failed to parse config file");
        log::info!("Configuration loaded successfully");
        return config;
    } else {
        let default_config = Config::default();
        log::info!("Configuration loaded successfully");
        return default_config;
    }
}

// This function checks if the config file is valid
// If the config file is not valid, it will print an error message and exit the program
pub fn test(config: &Config) -> Result<()> {
    log::info!("Testing configuration");
    if config.completion_time > 100 {
        return Err(anyhow::anyhow!("The completion time must be between 0 and 100. Please change the completion time in the config file."))
    }
    if config.sub_or_dub != "sub" && config.sub_or_dub != "dub" {
        return Err(anyhow::anyhow!("The sub_or_dub value must be either 'sub' or 'dub'. Please change the sub_or_dub value in the config file."))
    }
    log::info!("Configuration test passed");
    Ok(())
}
