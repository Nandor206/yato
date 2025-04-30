// =============== Imports ================
use crate::api;
use crate::config;
use crate::discord_rpc::paused_payload;
use crate::discord_rpc::payload;
use crate::local_save;
use crate::mpvipc;
use crate::scraping;
use crate::skip_override;
use crate::utils;

use anyhow::{Context, Result};
use console::style;
use discord_rpc_client;
use reqwest::Client;
use std::{collections::HashMap, path::Path, time::Duration};
use tokio::{self, sync::mpsc};

pub async fn play(
    client: &Client,
    id: i32,
    mal_id: i32,
    progress: u32,
    max_ep: u32,
    config: &config::Config,
    name: &String,
    cache: &mut HashMap<u32, String>,
    syncing: bool,
    rpc_client: &mut discord_rpc_client::Client,
) -> Result<bool> {
    let mut cur_ep = progress + 1;

    log::info!("Starting playback for anime: {}, Episode: {}", name, cur_ep,);

    if config.skip_filler {
        loop {
            let response = api::jikan::filler(&client, mal_id, cur_ep).await;
            if response.is_err() {
                break;
            } else if response.is_ok() && response? {
                log::info!("Skipping episode {} because it's a filler", cur_ep);
                cur_ep = cur_ep + 1;
            } else {
                break;
            }
        }
    }

    println!("Loading - {}, episode: {}", name, cur_ep);

    let (tx, mut rx) = mpsc::channel(1);
    let url = cache.get(&cur_ep);

    if url.is_none() {
        let client_clone = client.clone();
        let config_lang = config.language.clone();
        let config_quality = config.quality.clone();
        let config_sub_or_dub = config.sub_or_dub.clone();
        let name_clone = name.clone();
        let tx_clone = tx.clone();
        tokio::task::spawn(async move {
            let mut next_url: Result<String> = Err(anyhow::anyhow!(""));
            while next_url.is_err() {
                tokio::time::sleep(Duration::from_secs(3)).await;
                next_url = get_url(
                    &client_clone,
                    &config_lang,
                    mal_id,
                    id,
                    cur_ep,
                    &config_quality,
                    &config_sub_or_dub,
                    &name_clone,
                )
                .await
                .with_context(|| format!("Failed to fetch URL for episode {}", cur_ep + 1));
                if next_url.is_err() {
                    eprintln!("Failed to get episode link, retrying...");
                    log::warn!("Failed to get episode link for id: {}", id);
                }
            }
            tx_clone.send(next_url).await.unwrap();
        });
    }

    let mut anime = api::aniskip::Anime {
        episode: cur_ep,
        mal_id: mal_id,
        skip_times: api::aniskip::SkipData::default(),
    };
    let skip_times =
        api::aniskip::get_and_parse_ani_skip_data(&client, mal_id, cur_ep, 2, &mut anime);

    let mut player_args = config.player_args.split(' ').collect::<Vec<&str>>();
    player_args.retain(|arg| !arg.is_empty());

    let socket_path = format!("/tmp/yato-mpvsocket");
    if Path::new(&socket_path).exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let program = &config.player;
    let title = format!("--title={} - Episode {}", name, cur_ep);
    let media_title = format!(
        "--force-media-title={}",
        format!("{} - Episode {}", name, cur_ep)
    );
    let ipc_socket = format!("--input-ipc-server={}", socket_path);

    if url.is_none() {
        let url = rx
            .recv()
            .await
            .unwrap()
            .context("Failed to receive next episode URL from channel")?;
        cache.insert(cur_ep, url);
    }
    let url = cache.get(&cur_ep).unwrap();

    let mut db =
        local_save::ProgressDatabase::load().with_context(|| "Failed to load progress database")?;

    let entry = match db.get_entry(id) {
        Some(e) => e.to_owned(),
        None => local_save::WatchProgress {
            anilist_id: id,
            episode: cur_ep,
            position: 0.0,
            scraper_ids: {
                let mut map = HashMap::new();
                map.insert(config.language.to_string(), String::new());
                map
            },
        },
    };

    let start_time: String;
    if entry.episode == cur_ep && syncing {
        start_time = format!("--start={}", entry.position);
    } else {
        start_time = "--start=0".to_string();
    }

    let _ = std::process::Command::new(program)
        .arg("--hwdec=auto")
        .arg("--quiet")
        .arg(ipc_socket)
        .arg(title)
        .arg(media_title)
        .arg(start_time)
        .args(player_args)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to start player with program: {}", program))?;

    if cur_ep == entry.episode {
        let position = entry.position.round() as u64;
        let resuming_text = format!(
            "{:02}:{:02}:{:02}",
            position / 3600,
            (position % 3600) / 60,
            position % 60
        );
        println!("Resuming from - {}", style(resuming_text).bold());
    }
    else {
        println!("Starting from the begining");
    }

    loop {
        // * The code is so fast it quits before the player is ready, we have to wait for it
        if mpvipc::get_property("time-pos").is_ok() {
            log::info!("Player started successfully");
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let override_setting = skip_override::search(id);
    let scraper_id = entry
        .scraper_ids
        .get(&config.language)
        .cloned()
        .unwrap_or_default();

    match skip_times.await {
        Ok(_) => {
            api::aniskip::send_skip_times_to_mpv(&anime)
                .with_context(|| "Failed to send skip times to MPV")?;
        }
        Err(e) => {
            eprintln!("Failed to fetch AniSkip data: {}", e);
        }
    }

    let mut update_progress = false;
    let mut binge_watching = false;
    // Needed for link caching
    let mut started_caching = false;
    let mut last_time_pos: f64 = 0.0;

    if config.discord_presence {
        let client_clone = client.clone();
        let id_clone = id.clone();
        let mut rpc_clone = rpc_client.clone();

        tokio::task::spawn(async move {
            let data = api::anilist::fetch::data_by_id(&client_clone, id_clone)
                .await
                .unwrap();
            let mut paused;
            let mut last_time_pos = 0.0;
            let mut set_paused = false;
            let mut set = false;

            loop {
                let time_pos = mpvipc::get_property("time-pos");
                match time_pos {
                    Ok(_) => (),
                    Err(_) => {
                        break;
                    }
                }

                let cur_pos = time_pos.unwrap();
                let rounded = cur_pos.round() as u64;

                if cur_pos == last_time_pos {
                    paused = true;
                } else {
                    paused = false;
                }
                // * If the user jumps, we reset the time (to be accurate in discord)
                if cur_pos > last_time_pos + 10.0 {
                    let payload = payload(&data, progress + 1, max_ep, rounded);
                    rpc_clone
                        .set_activity(|_| payload)
                        .expect("Failed to update activity");
                }

                last_time_pos = cur_pos;

                if !paused && !set {
                    let payload = payload(&data, progress + 1, max_ep, rounded);
                    rpc_clone
                        .set_activity(|_| payload)
                        .expect("Failed to update activity");
                    set = true;
                    set_paused = false;
                } else if !set_paused && paused {
                    let payload = paused_payload(&data, progress + 1, max_ep);
                    rpc_clone
                        .set_activity(|_| payload)
                        .expect("Failed to update activity");
                    set_paused = true;
                    set = false;
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }

    loop {
        let time_pos = mpvipc::get_property("time-pos");
        let duration = mpvipc::get_property("duration");
        if time_pos.is_err() || duration.is_err() {
            break;
            // If app is closed breaks the loop
        }

        let time_pos = time_pos?;
        let duration = duration?;

        let percent = (time_pos / duration) * 100.0;

        if percent > config.completion_time as f64 && !update_progress {
            update_progress = true;
        }
        if percent < config.completion_time as f64 && update_progress {
            update_progress = false;
        }

        last_time_pos = time_pos;

        // Skipping intro and outro
        // Override basically does the opposite of the setting in the config file
        if override_setting.intro {
            if !config.skip_opening {
                if time_pos >= anime.skip_times.op.start && time_pos <= anime.skip_times.op.end {
                    mpvipc::seek_to(anime.skip_times.op.end)
                        .with_context(|| "Failed to seek past opening")?;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        } else {
            if time_pos >= anime.skip_times.op.start
                && time_pos <= anime.skip_times.op.end
                && config.skip_opening
            {
                mpvipc::seek_to(anime.skip_times.op.end)
                    .with_context(|| "Failed to seek past opening")?;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        if override_setting.outro {
            if !config.skip_credits {
                if time_pos >= anime.skip_times.ed.start && time_pos <= anime.skip_times.ed.end {
                    mpvipc::seek_to(anime.skip_times.ed.end)
                        .with_context(|| "Failed to seek past credits")?;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        } else {
            if time_pos >= anime.skip_times.ed.start
                && time_pos <= anime.skip_times.ed.end
                && config.skip_credits
            {
                mpvipc::seek_to(anime.skip_times.ed.end)
                    .with_context(|| "Failed to seek past credits")?;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        // Skipping recap
        if config.skip_recap {
            if time_pos >= anime.skip_times.recap.start && time_pos <= anime.skip_times.recap.end {
                mpvipc::seek_to(anime.skip_times.recap.end)
                    .with_context(|| "Failed to seek past recap")?;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        // Adding next link to cache
        if percent >= 70.0 && !started_caching {
            started_caching = true;
            let client = client.clone();
            let config_lang = config.language.clone();
            let config_quality = config.quality.clone();
            let config_sub_or_dub = config.sub_or_dub.clone();
            let name_clone = name.clone();
            let tx_clone = tx.clone();
            tokio::task::spawn(async move {
                let mut next_url: Result<String> = Err(anyhow::anyhow!(""));
                while next_url.is_err() {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    next_url = get_url(
                        &client,
                        &config_lang,
                        mal_id,
                        id,
                        cur_ep + 1,
                        &config_quality,
                        &config_sub_or_dub,
                        &name_clone,
                    )
                    .await
                    .with_context(|| format!("Failed to fetch URL for episode {}", cur_ep + 1));
                    if next_url.is_err() {
                        eprintln!("Failed to get next episode link, retrying...");
                        log::warn!("Failed to get next episode link for id: {}", id);
                    } else {
                        println!("Next episode link cached.");
                    }
                }
                tx_clone.send(next_url).await.unwrap();
            });
        }

        // Binge watching starts after credits scene or completion time
        if time_pos >= anime.skip_times.ed.end || percent >= config.completion_time as f64 {
            binge_watching = true;
        } else if time_pos < anime.skip_times.ed.end || percent < config.completion_time as f64 {
            binge_watching = false;
        }

        // * Sleep for a bit to avoid high CPU usage
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    utils::clear();

    if cur_ep != max_ep && update_progress && syncing {
        api::anilist::mutation::update_progress(&client, id, cur_ep + 1)
            .await
            .with_context(|| format!("Failed to update progress to episode {}", cur_ep + 1))?;
        println!("Progress updated to {}", cur_ep + 1);
    }

    if syncing {
        db.update_or_add(id, cur_ep + 1, last_time_pos, &config.language, &scraper_id);
        db.save()
            .with_context(|| "Failed to save progress database")?;
    }

    if started_caching {
        let next_url = rx
            .recv()
            .await
            .unwrap()
            .context("Failed to receive next episode URL from channel")?;
        cache.insert(cur_ep + 1, next_url);
    }

    log::info!(
        "Playback stopped for Episode: {} at {}",
        progress + 1,
        last_time_pos
    );
    log::info!("Saved progress for episode: {}", cur_ep);
    Ok(binge_watching)
}

pub async fn get_url(
    client: &Client,
    lang: &str,
    mal_id: i32,
    id: i32,
    episode: u32,
    quality: &str,
    sub_or_dub: &str,
    name: &String,
) -> Result<String> {
    let url: String = match lang {
        "hungarian" => {
            let url = scraping::hun_scraping::get_link(&client, mal_id, id, episode, quality).await;
            if url.is_err() {
                let err = format!("Anime not found on AnimeDrive, error: {}", url.unwrap_err());
                log::warn!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
            url?
        }
        "english" => {
            let url = scraping::eng_scraping::get_link(
                &client, lang, id, episode, quality, sub_or_dub, name,
            )
            .await;
            if url.is_err() {
                let err = format!("Anime not found on AllAnime, error: {}", url.unwrap_err());
                log::warn!("{}", err);
                return Err(anyhow::anyhow!(err));
            }
            url?
        }
        _ => {
            // todo: Add more languages
            return Err(anyhow::anyhow!("Language not supported"));
        }
    };
    Ok(url)
}
