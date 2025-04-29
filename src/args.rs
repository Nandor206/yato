// =============== Imports ================
use crate::api::anilist;
use crate::config::{self, Config};
use crate::utils;
use crate::discord_rpc;

use anyhow::{Result, Context};
use clap::{Arg, ArgAction, ArgMatches, Command};
use reqwest::Client;
use std::process;

fn build() -> Command {
    Command::new("yato")
        .version("1.0.0")
        .author("Nandor206")
        .arg(
            Arg::new("edit")
                .short('e')
                .long("edit")
                .help("Edit your config file")
                .long_help("Edit your config file in nano")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(vec!["information", "anime", "number", "dub", "sub", "language", "quality", "rpc", "change-token", "new", "skip-op", "skip-ed", "skip-recap", "skip-filler"])
                .required(false),
        )
        .arg(
            Arg::new("continue")
                .short('c')
                .long("continue")
                .help("Continue watching from currently watching list")
                .long_help("Continue watching from currently watching list (using the user's anilist account)")
                .conflicts_with("sub")
                .action(ArgAction::SetTrue)
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("dub")
                .long("dub")
                .help("Allows user to watch anime in dub")
                .next_line_help(false)
                .conflicts_with("sub")
                .action(ArgAction::SetTrue)
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("sub")
                .long("sub")
                .help("Allows user to watch anime in sub")
                .next_line_help(false)
                .conflicts_with("dub")
                .action(ArgAction::SetTrue)
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("language")
                .short('l')
                .long("language")
                .visible_alias("lang")
                .value_name("LANGUAGE")
                .help("Set preferred language (e.g. english, japanese, hungarian, etc.)")
                .long_help("Set preferred language (e.g. english, japanese, hungarian, etc.) - default: english")
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("quality")
                .short('q')
                .long("quality")
                .value_name("QUALITY")
                .help("Specify the video quality (e.g. 1080p, etc.)")
                .long_help("Specify the video quality (e.g. 1080p, 720p, etc. — default: best available).")
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("information")
                .short('i')
                .long("information")
                .visible_alias("info")
                .value_name("ANILIST ID OR NAME")
                .help("Displays information of the anime")
                .conflicts_with_all(&["edit", "anime", "number"])
                .required(false),
        )
        .arg(
            Arg::new("number")
                .short('n')
                .long("number")
                .value_name("EPISODE NUMBER")
                .value_parser(clap::value_parser!(u32))
                .help("Specify the episode number to start watching from.\nMust be used with a [QUERY]")
                .requires("anime")
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("anime")
                .value_name("QUERY")
                .next_line_help(false)
                .help("Watch specific anime without syncing with Anilist.\nMust be used with --number.")
                .requires("number")
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("rpc")
                .short('d')
                .long("discord")
                .help("Toggles Discord Rich Presence")
                .action(ArgAction::SetTrue)
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("change-token")
                .long("change-token")
                .help("Deletes your auth token stored")
                .action(ArgAction::SetTrue)
                .conflicts_with("edit")
                .conflicts_with("information")
                .required(false),
        )
        .arg(
            Arg::new("new")
                .long("new")
                .help("Allows the user to add a new anime")
                .conflicts_with_all(vec!["edit", "information", "anime", "number"])
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("percentage")
                .long("completion-time")
                .help("Allows user to set a different completion time")
                .value_name("PERCENTAGE")
                .value_parser(clap::value_parser!(u8))
                .required(false),
        )
        .arg(
            Arg::new("score-completion")
                .long("score-on-completion")
                .help("Toggle whether to set a score when the anime is marked as completed")
                .conflicts_with_all(vec!["edit", "information"])
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("skip-op")
                .long("skip-op")
                .help("Toggles the setting set in the config")
                .conflicts_with_all(vec!["edit", "information"])
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("skip-ed")
                .long("skip-ed")
                .help("Toggles the setting set in the config")
                .conflicts_with_all(vec!["edit", "information"])
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("skip-filler")
                .long("skip-filler")
                .help("Toggles the setting set in the config")
                .conflicts_with_all(vec!["edit", "information"])
                .action(ArgAction::SetTrue)
                .required(false),
        )
        .arg(
            Arg::new("skip-recap")
                .long("skip-recap")
                .help("Toggles the setting set in the config")
                .conflicts_with_all(vec!["edit", "information"])
                .action(ArgAction::SetTrue)
                .required(false),
        )
}

pub async fn handle_args(config: &mut Config, client: &Client) -> Result<(ArgMatches, discord_rpc_client::Client)> {
    let matches = build().get_matches();

    if matches.get_flag("edit") {
        let config_file = dirs::config_dir().unwrap().join("yato/yato.conf");
        if !config_file.exists() {
            // Create the config file if it doesn't exist
            config::create(&config_file);
        }
        process::Command::new("nano")
            .arg(&config_file)
            .status()
            .expect("Failed to open config file in nano");
        println!(
            "If you want to edit the config file in another editor, please open it manually at: {}",
            config_file.display()
        );
        process::exit(0);
    }
    let default_language = config.language.clone();

    let lang = matches
        .get_one::<String>("language")
        .map(String::as_str)
        .unwrap_or(&default_language);

    config.language = match lang {
        "hun" => "hungarian".to_string(),
        "eng" => "english".to_string(),
        _ => lang.to_string(),
    };

    let default_quality = config.quality.clone();

    let quality = matches
        .get_one::<String>("quality")
        .map(String::as_str)
        .unwrap_or(&default_quality);

    config.quality = quality.replace("p", "").replace("P", "").to_string();

    if matches.get_flag("rpc") {
        let default_rpc = config.discord_presence;
        config.discord_presence = !default_rpc;
    }

    if matches.get_flag("change-token") {
        anilist::user_fetch::remove_token_file()?;
    }

    if matches.get_flag("dub") {
        config.sub_or_dub = "dub".to_string();
    }
    if matches.get_flag("sub") {
        config.sub_or_dub = "sub".to_string();
    }

    if matches.contains_id("percentage") {
        let percentage = matches
            .get_one::<u8>("percentage")
            .unwrap_or_else(|| &90)
            .to_owned();
        config.completion_time = percentage;
    }

    if matches.get_flag("score-completion") {
        let default_score_on = config.score_on_completion;
        config.score_on_completion = !default_score_on;
    }

    if matches.get_flag("skip-op") {
        let default_skip = config.skip_opening;
        config.score_on_completion = !default_skip;
    }
    if matches.get_flag("skip-ed") {
        let default_skip = config.skip_credits;
        config.score_on_completion = !default_skip;
    }
    if matches.get_flag("skip-recap") {
        let default_skip = config.skip_recap;
        config.score_on_completion = !default_skip;
    }
    if matches.get_flag("skip-filler") {
        let default_skip = config.skip_filler;
        config.score_on_completion = !default_skip;
    }

    let mut rpc_client;

    let drpc = discord_rpc::init();
    match drpc {
        Ok(client) => rpc_client = client,
        Err(e) => {
            log::error!("Failed to initialize Discord Rich Presence: {}", e);
            println!("Couldn't initialize Discord Rich Presence");
            config.discord_presence = false; // If the client failed to be initialized, don't bother with it
            rpc_client = discord_rpc_client::Client::new(123) // dummy client
        }
    }
    if config.discord_presence {
        rpc_client.start();
    }

    // * No new changes in config after this

    let original_input = matches
        .get_one::<String>("information")
        .map(String::as_str)
        .unwrap_or_else(|| "");

    utils::clear();
    if !original_input.is_empty() {
        let input = original_input.parse();
        match input {
            Ok(i) => {
                anilist::fetch::information(&client, i)
                    .await
                    .with_context(|| format!("Failed to fetch information for ID: {}", i))?;
            }
            Err(_) => {
                let i = anilist::fetch::search(&client, original_input.to_string()).await;
                match i {
                    Ok(i) => {
                        anilist::fetch::information(&client, i)
                            .await
                            .with_context(|| {
                                format!("Failed to fetch information for ID: {}", i)
                            })?;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        Err(e).context("Failed to get information")?;
                    }
                }
            }
        }
        process::exit(0);
    }

    Ok((matches,rpc_client))
}
