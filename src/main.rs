// ! For better documentation readability, please use the following VScode extension:
// https://marketplace.visualstudio.com/items/?itemName=aaron-bond.better-comments

// * Meaning of the signs:
// ! - Important
// ? - No use yet
// * - Important, but not that important (usually just plain documentation, like what the function does)
// Without the signs, it's just a plain comment

// Achievements:
// Project started on 2025.03.27. by Nandor206
// First working version: 2025.04.10. (Hungarian only)
// Second working version: 2025.04.22. (Huge updates, English finally added, though the links from it don't work yet)
// First testings: 2025.04.24. (The code is so fast, that it quits before the video starts) - Fixed right away
// Discord rpc added! - The program is done, real men test in production
// Final version: 2025.04.27.

// =============== Imports ================
mod api;
mod config;
mod discord_rpc;
mod local_save;
mod mpvipc;
mod player;
mod scraping;
mod skip_override;
mod theme;
mod utils;
mod args;

use anyhow::{Context, Result};
use dialoguer::{Input, MultiSelect, Select};
use discord_rpc_client;
use reqwest::{Client, ClientBuilder};
use std::{collections::HashMap, io, process};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    utils::init_log()?; // Initialize logging
    log::info!("Application started");

    // ! Creating client with a 120 second timeout (needed for hun_scraping, if they finally fix their site I will remove it)
    let client = ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .with_context(|| "Failed to create HTTP client")?;
    let mut config = config::load_config();

    config::test(&config)?; // Testing if the config file is valid

    let (matches, rpc_client) = args::handle_args(&mut config, &client).await?;
    {

        utils::check_network(&client).await?; // Checking if the network is available

        log::debug!("Updated configuration: {:#?}", config);

        if matches.contains_id("anime") || matches.contains_id("number") {
            let anime_name = matches
                .get_one::<String>("anime")
                .map(String::as_str)
                .unwrap_or_else(|| "");
            let mut episode_number = matches
                .get_one::<u32>("number")
                .unwrap_or_else(|| &0)
                .to_owned();
            if episode_number > 0 {
                episode_number = episode_number - 1;
            }

            match anime_name.parse::<i32>() {
                Ok(anime_id) => {
                    let data = api::anilist::fetch::data_by_id(&client, anime_id).await?;
                    let max_ep = data.episodes;
                    let name = data.title;
                    let info = api::anilist::user_fetch::AnimeData {
                        id: anime_id,
                        title: name.clone(),
                        progress: episode_number,
                        episodes: max_ep,
                    };
                    watch(&client, config, rpc_client, info, false).await?;
                }
                Err(_) => {
                    let anime_id =
                        api::anilist::fetch::search(&client, anime_name.to_string()).await?;
                    let data = api::anilist::fetch::data_by_id(&client, anime_id).await?;
                    let max_ep = data.episodes;
                    let name = data.title;
                    let info = api::anilist::user_fetch::AnimeData {
                        id: anime_id,
                        title: name.clone(),
                        progress: episode_number,
                        episodes: max_ep,
                    };
                    watch(&client, config, rpc_client, info, false).await?;
                }
            }
            return Ok(());
        }

        api::anilist::user_fetch::check_credentials(&client).await?; // * Credentials are only needed after this part
        if matches.get_flag("new") {
            add_new_anime(&client).await?;
        }
    }

    // Clearing the screen for better looks
    utils::clear();

    let options = vec![
        "Continue Watching",
        "Edit (Episodes, Status, Score, Skipping)",
        "Info",
        "Add anime to list",
        "Exit",
    ];
    let theme = theme::CustomTheme {};

    let select_options = Select::with_theme(&theme)
        .with_prompt("Select an option:")
        .default(0)
        .items(&options)
        .interact_opt()?;

    // * Deciding what to do on each scenario
    if select_options.is_none() {
        // User pressed ESC or Q
        println!("See you later!");
        return Ok(());
    } else if select_options == Some(0) {
        if config.discord_presence {
            discord_rpc::selecting(&rpc_client,"Debating what to watch", "");
        }
        continue_watching(&client, config, rpc_client).await?;
    } else if select_options == Some(1) {
        // Edit (Episodes, Status, Score, Skipping)
        if config.discord_presence {
            discord_rpc::selecting(&rpc_client,"Updating their List", "");
        }
        update(&client).await?;
    } else if select_options == Some(2) {
        // Information about an anime
        info(&client).await?;
    } else if select_options == Some(3) {
        // Add new anime
        if config.discord_presence {
            discord_rpc::selecting(&rpc_client,"Thinking what to watch next", "");
        }
        add_new_anime(&client).await?;
    } else {
        // Exit
        utils::clear();
        println!("See you later!");
        return Ok(());
    }

    log::info!("Application exiting");
    Ok(())
}

async fn info(client: &Client) -> Result<()> {
    utils::clear();
    // * Input -> search for anime -> shows information of the selected one
    let theme = theme::CustomTheme {};
    let anime_name: String = Input::with_theme(&theme)
        .with_prompt("Enter the name of the anime")
        .interact_text()?;
    let search = api::anilist::fetch::search(&client, anime_name).await;
    utils::clear();
    if search.is_err() {
        eprintln!("Error: {}", search.unwrap_err());
        process::exit(1);
    } else {
        api::anilist::fetch::information(&client, search.unwrap()).await?;
    }

    Ok(())
}

async fn add_new_anime(client: &Client) -> Result<()> {
    utils::clear();
    // * Input -> search for anime -> add anime to the list
    let theme = theme::CustomTheme {};
    let anime_name: String = Input::with_theme(&theme)
        .with_prompt("Enter the name of the anime")
        .interact_text()?;

    utils::clear();

    let anime_id = api::anilist::fetch::search(&client, anime_name).await;
    match anime_id {
        Ok(anime_id) => {
            utils::clear();
            let options = vec![
                "Watching",
                "Completed",
                "Paused",
                "Dropped",
                "Planning",
                "Rewatching",
            ];
            let selection = Select::with_theme(&theme)
                .with_prompt("Select the status for the anime")
                .items(&options)
                .default(0)
                .interact_opt()?;

            if selection.is_none() {
                return Err(anyhow::anyhow!("No selection made"));
            }
            api::anilist::mutation::update_status(&client, anime_id, selection.unwrap()).await?;
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}

async fn continue_watching(
    client: &Client,
    config: config::Config,
    rpc_client: discord_rpc_client::Client,
) -> Result<()> {
    utils::clear();
    let info = api::anilist::user_fetch::current(&client).await?;
    utils::clear();

    watch(&client, config, rpc_client, info, true).await?;

    Ok(())
}

async fn update(client: &Client) -> Result<()> {
    utils::clear();
    let options = vec![
        "Change Progress",
        "Change Status",
        "Change Score",
        "Override skipping settings",
    ];
    let theme = theme::CustomTheme {};
    let select_options = Select::with_theme(&theme)
        .with_prompt("Choose an option:")
        .default(0)
        .items(&options)
        .interact_opt()?;
    utils::clear();
    if select_options.is_none() {
        // User pressed ESC or Q
        println!("See you later!");
        process::exit(0);
    } else if select_options == Some(0) {
        let anime_id = api::anilist::user_fetch::current(&client).await?.id;
        utils::clear();
        let new_episode: u32 = Input::new()
            .with_prompt("Enter a new episode number")
            .interact_text()?;
        utils::clear();
        api::anilist::mutation::update_progress(&client, anime_id, new_episode)
            .await
            .with_context(|| format!("Failed to update progress to episode {}", new_episode))?;
        println!("Progress updated!");
    } else if select_options == Some(1) {
        let anime_id = api::anilist::user_fetch::list_all(&client, 0).await?;
        utils::clear();
        let options = vec![
            "Watching",
            "Completed",
            "Paused",
            "Dropped",
            "Planning",
            "Rewatching",
        ];
        let selection = Select::with_theme(&theme)
            .with_prompt("Select the status for the anime")
            .items(&options)
            .default(0)
            .interact_opt()?;
        api::anilist::mutation::update_status(&client, anime_id, selection.unwrap()).await?;
        // For rewatching and current the progress will be reset.
        if selection.unwrap() == 5 || selection.unwrap() == 0 {
            api::anilist::mutation::update_progress(&client, anime_id, 0).await?;
        }
    } else if select_options == Some(2) {
        let anime_id = api::anilist::user_fetch::list_all(&client, 1).await?;
        utils::clear();
        let theme = theme::CustomTheme {};
        let new_score: f64 = Input::with_theme(&theme)
            .with_prompt("Enter a new score on a scale of 1 to 10")
            .interact_text()
            .expect("Failed to read input, or input was invalid");
        utils::clear();
        if new_score < 1.0 || new_score > 10.0 {
            eprintln!("Score must be between 1 and 10");
            process::exit(1);
        } else {
            api::anilist::mutation::update_score(&client, anime_id, new_score).await?;
        }
    } else if select_options == Some(3) {
        if !skip_override::check_override() {
            println!(
                "Note: Overriding a config setting means doing the *opposite* of what's defined in your config file."
            );
            println!(
                "For example, if \"skip_intro\" is set to true in the config, overriding will make it *not* skip the intro."
            );
            println!("\nUsage: Select with space, and press enter to confirm your selection.");
            println!("Press Enter to continue...");
            let _ = io::stdin().read_line(&mut String::new());
        }
        let options = vec!["Add/update override", "Delete an existing override"];
        let theme = theme::CustomTheme {};
        let over_ride = Select::with_theme(&theme)
            .with_prompt("Choose an option")
            .items(&options)
            .default(0)
            .interact_opt()?;
        utils::clear();

        if over_ride.is_none() {
            println!("See you later!");
            process::exit(0);
        } else if over_ride == Some(0) {
            let options = vec!["Update existing override", "Add new override"];
            let theme = theme::CustomTheme {};
            let select = Select::with_theme(&theme)
                .with_prompt("Would you like to update an existing override or add a new one?")
                .items(&options)
                .default(0)
                .interact_opt()?;
            utils::clear();
            if select.is_none() {
                println!("See you later!");
                process::exit(0);
            } else if select == Some(0) {
                // Updating existing one
                skip_override::interactive_update_override(&client).await;
            } else if select == Some(1) {
                // Adding new one
                let theme = theme::CustomTheme {};
                let input = Input::with_theme(&theme)
                    .with_prompt("Enter the name of the anime")
                    .interact_text()?;
                let anime_id = api::anilist::fetch::search(&client, input).await?;
                utils::clear();

                let options = vec!["Intro", "Outro", "Recap"];
                let selection = MultiSelect::with_theme(&theme)
                    .with_prompt("What overrides do you want to enable?")
                    .items(&options)
                    .interact_opt()?;
                utils::clear();
                if selection.is_none() {
                    println!("See you later!");
                    process::exit(0);
                } else if selection.is_some() {
                    let selected = selection.unwrap();
                    let mut intro = false;
                    let mut outro = false;
                    let mut recap = false;
                    let mut filler = false;
                    for i in selected {
                        if i == 0 {
                            intro = true;
                        } else if i == 1 {
                            outro = true;
                        } else if i == 2 {
                            recap = true;
                        } else if i == 3 {
                            filler = true;
                        }
                    }
                    skip_override::add_override(anime_id, intro, outro, recap, filler);
                }
                println!("Override has been successfully saved!");
            }
        } else if over_ride == Some(1) {
            // Deleting existing one
            skip_override::interactive_delete_override(&client).await;
        }
    }

    Ok(())
}

async fn sequel(client: &Client, anime_id: i32) -> Result<i32> {
    let sequel = api::anilist::fetch::sequel_data(&client, anime_id).await;
    match sequel {
        Ok(s) => {
            println!("Sequel: {}\n", s.title);
            let theme = theme::CustomTheme {};
            let options = vec![
                "Add sequel to \"Currently watching\"",
                "Don't add sequel to my watchlist",
            ];
            let select = Select::with_theme(&theme)
                .with_prompt("Would you like to add the sequel to watchlist?")
                .items(&options)
                .default(0)
                .interact_opt()
                .unwrap();

            utils::clear();
            match select {
                Some(id) => {
                    if id == 0 {
                        api::anilist::mutation::update_status(&client, s.id, 0).await?;

                        let options = vec!["Yes", "No"];
                        let select = Select::with_theme(&theme)
                            .with_prompt("Would you like to continue watching with the sequel?")
                            .items(&options)
                            .default(0)
                            .clear(true)
                            .interact_opt()
                            .unwrap();
                        match select {
                            Some(id) => {
                                if id == 1 {
                                    Err(anyhow::anyhow!(
                                        "User chose not to continue watching with the sequel."
                                    ))
                                } else {
                                    Ok(s.id)
                                }
                            }
                            None => Err(anyhow::anyhow!("No selection made")),
                        }
                    } else {
                        println!("See you later!");
                        process::exit(0);
                    }
                }
                None => Err(anyhow::anyhow!("No selection made")),
            }
        }
        Err(e) => Err(anyhow::anyhow!("Failed to get sequel data: {}", e)),
    }
}

async fn watch(
    client: &Client,
    config: config::Config,
    mut rpc_client: discord_rpc_client::Client,
    anime_data: api::anilist::user_fetch::AnimeData,
    syncing: bool,
) -> Result<()> {
    let mut cur_ep = anime_data.progress;
    let mut link_cache: HashMap<u32, String> = HashMap::new();

    let mut anime_id = anime_data.id;
    let mut max_ep = anime_data.episodes;
    let mut anime_name = anime_data.title;
    let mut mal_id = api::anilist::fetch::id_converter(&client, anime_id).await?;

    loop {
        // * binge is set true when the user gets after the credits scene, before that it's false
        let binge = player::play(
            &client,
            anime_id,
            mal_id,
            cur_ep,
            max_ep,
            &config,
            &anime_name,
            &mut link_cache,
            syncing,
            &mut rpc_client,
        )
        .await?;
        log::info!("Binge watching: {}\n", binge);

        // * Scoring anime, then checking if the user wants to continue watching with the sequel
        if cur_ep == max_ep && config.score_on_completion && binge {
            utils::clear();
            let theme = theme::CustomTheme {};
            let new_score: f64 = loop {
                let input: String = Input::with_theme(&theme)
                    .with_prompt("Enter a score on a scale of 1 to 10")
                    .interact_text()
                    .expect("Failed to read input, or input was invalid");

                match input.parse::<f64>() {
                    Ok(score) if score >= 1.0 && score <= 10.0 => break score,
                    _ => {
                        eprintln!("Score must be between 1 and 10.");
                        continue;
                    }
                }
            };
            utils::clear();
            api::anilist::mutation::update_score(&client, anime_id, new_score).await?;
            println!("This was the last episode of the season.");
            let sequel = sequel(&client, anime_id).await;
            if let Ok(sequel_id) = sequel {
                api::anilist::mutation::update_status(&client, sequel_id, 0).await?;
            } else {
                break;
            }
        } else if cur_ep == max_ep && binge {
            // * Same as above, but without scoring
            println!("This was the last episode of the season.");
            let sequel = sequel(&client, anime_id).await;
            if let Ok(sequel_id) = sequel {
                api::anilist::mutation::update_progress(&client, sequel_id, 0).await?; // Setting the progress to 0 just incase
                anime_id = sequel_id;
                cur_ep = 0;
                let data = api::anilist::fetch::data_by_id(&client, anime_id).await;
                match data {
                    Ok(d) => {
                        max_ep = d.episodes;
                        anime_name = d.title;
                    }
                    Err(e) => return Err(e),
                }
                println!("Starting the sequel...");
                link_cache.clear();
                mal_id = api::anilist::fetch::id_converter(&client, sequel_id)
                    .await
                    .unwrap();
                link_cache.insert(
                    cur_ep + 1,
                    player::get_url(
                        &client,
                        &config.language,
                        mal_id,
                        anime_id,
                        cur_ep + 1,
                        &config.quality,
                        &config.sub_or_dub,
                        &anime_name,
                    )
                    .await?,
                );
                continue;
            } else {
                break;
            }
        }

        if !binge {
            if config.discord_presence {
                rpc_client.clear_activity().expect("Failed to clear activity");
            }
            break;
        } else {
            cur_ep += 1;
        }
    }
    println!("See you later!");

    Ok(())
}
