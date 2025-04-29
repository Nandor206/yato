// =============== Imports ================
use crate::api::anilist::fetch;
use crate::theme;
use crate::utils;

use anyhow::{Context, Result};
use console::style;
use dialoguer::MultiSelect;
use dialoguer::Select;
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Override {
    id: i32,
    pub intro: bool,
    pub outro: bool,
    pub recap: bool,
    pub filler: bool,
}

pub fn read_settings_from_file(file_path: &str) -> Result<Vec<Override>> {
    if !Path::new(file_path).exists() {
        return Ok(Vec::new());
    }

    let mut file =
        File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read contents of file: {}", file_path))?;

    let settings: Vec<Override> = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse JSON from file: {}", file_path))?;
    Ok(settings)
}

fn save_settings_to_file(file_path: &str, settings: Vec<Override>) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)
        .with_context(|| format!("Failed to open file for writing: {}", file_path))?;

    let serialized =
        serde_json::to_string(&settings).with_context(|| "Failed to serialize settings to JSON")?;
    file.write_all(serialized.as_bytes())
        .with_context(|| format!("Failed to write to file: {}", file_path))?;
    Ok(())
}

fn update_or_add_setting(file_path: &str, new_setting: Override) -> Result<()> {
    let mut settings = read_settings_from_file(file_path)
        .with_context(|| format!("Failed to read settings from file: {}", file_path))?;

    if let Some(setting) = settings.iter_mut().find(|s| s.id == new_setting.id) {
        setting.intro = new_setting.intro;
        setting.outro = new_setting.outro;
        setting.recap = new_setting.recap;
        setting.filler = new_setting.filler;
    } else {
        settings.push(new_setting);
    }

    save_settings_to_file(file_path, settings)
        .with_context(|| format!("Failed to save updated settings to file: {}", file_path))?;
    Ok(())
}

fn find_setting_by_id(file_path: &str, id: i32) -> Option<Override> {
    read_settings_from_file(file_path)
        .with_context(|| format!("Failed to read settings from file: {}", file_path))
        .ok()
        .and_then(|settings| settings.into_iter().find(|setting| setting.id == id))
}

pub fn add_override(id: i32, intro: bool, outro: bool, recap: bool, filler: bool) {
    let data_path = dirs::data_local_dir().unwrap();
    let dir_path = data_path.join("yato");
    let file_path = dir_path.join("override.json");

    let new_setting = Override {
        id,
        intro,
        outro,
        recap,
        filler,
    };

    if let Err(e) = update_or_add_setting(file_path.to_str().unwrap(), new_setting) {
        eprintln!("Failed to update settings: {}", e);
    }
}

pub fn search(id: i32) -> Override {
    let data_path = dirs::data_local_dir().unwrap();
    let dir_path = data_path.join("yato");
    let file_path = dir_path.join("override.json");

    match find_setting_by_id(file_path.to_owned().to_str().unwrap(), id) {
        Some(setting) => setting,
        None => Override {
            id: id,
            intro: false,
            outro: false,
            recap: false,
            filler: false,
        },
    }
}

pub fn delete_override(id: i32) {
    let data_path = dirs::data_local_dir().unwrap();
    let dir_path = data_path.join("yato");
    let file_path = dir_path.join("override.json");

    match read_settings_from_file(file_path.to_str().unwrap()).with_context(|| {
        format!(
            "Failed to read settings from file: {}",
            file_path.to_str().unwrap()
        )
    }) {
        Ok(mut settings) => {
            let original_len = settings.len();
            settings.retain(|s| s.id != id);

            if settings.len() != original_len {
                if let Err(e) = save_settings_to_file(file_path.to_str().unwrap(), settings)
                    .with_context(|| {
                        format!(
                            "Failed to save updated settings to file: {}",
                            file_path.to_str().unwrap()
                        )
                    })
                {
                    eprintln!("Failed to save updated settings: {}", e);
                }
            } else {
                eprintln!("No override found with id: {}", id);
            }
        }
        Err(e) => {
            eprintln!("Failed to read settings: {}", e);
        }
    }
}

pub async fn interactive_delete_override(client: &Client) -> Result<()> {
    let data_path = dirs::data_local_dir().unwrap();
    let dir_path = data_path.join("yato");
    let file_path = dir_path.join("override.json");

    let settings = match read_settings_from_file(file_path.to_str().unwrap()) {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            println!("No overrides found.");
            return Ok(())
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to read settings: {}", e))
        }
    };

    // Fetch all names concurrently using join_all
    let name_futures = settings.iter().map(|s| fetch::data_by_id(client, s.id));
    let data_vec: Vec<Result<fetch::AnimeData>> = join_all(name_futures).await;

    let options: Vec<String> = settings
        .iter()
        .zip(data_vec.iter())
        .map(|(s, data)| {
            let title = match data {
                Ok(anime) => &anime.title,
                Err(_) => "<Unknown Title>",
            };
            format!(
                "{} | Current override settings: \topening: {} | credits: {} | recap: {} | filler: {}",
                title, s.intro, s.outro, s.recap, s.filler
            )
        })
        .collect();

    let theme = theme::CustomTheme {};
    let selection = Select::with_theme(&theme)
        .with_prompt("Select a setting to delete")
        .items(&options)
        .default(0)
        .interact_opt();
    utils::clear();

    match selection {
        Ok(index) => match index {
            Some(index) => {
                let id_to_delete = settings[index].id;
                delete_override(id_to_delete);
            }
            None => {
                return Err(anyhow::anyhow!("No selection made"));
            }
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Selection failed: {}", e));
        }
    }

    println!("Override deleted!");
    Ok(())
}

pub fn check_override() -> bool {
    let data_path = dirs::data_local_dir().unwrap();
    let dir_path = data_path.join("yato");
    let file_path = dir_path.join("override.json");

    if file_path.exists() { true } else { false }
}

pub async fn interactive_update_override(client: &Client) -> Result<()> {
    let data_path = dirs::data_local_dir().unwrap();
    let dir_path = data_path.join("yato");
    let file_path = dir_path.join("override.json");

    let settings = match read_settings_from_file(file_path.to_str().unwrap()) {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            println!("No overrides found.");
            return Ok(())
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to read settings: {}", e))
        }
    };

    // Fetch all names concurrently using join_all
    let name_futures = settings.iter().map(|s| fetch::data_by_id(client, s.id));
    let data_vec: Vec<Result<fetch::AnimeData>> = join_all(name_futures).await;

    let options: Vec<String> = settings
        .iter()
        .zip(data_vec.iter())
        .map(|(_, data)| {
            let title = match data {
                Ok(anime) => &anime.title,
                Err(_) => "<Unknown Title>",
            };
            format!("{}", title,)
        })
        .collect();

    let theme = theme::CustomTheme {};
    let selection = Select::with_theme(&theme)
        .with_prompt("Select an anime to update")
        .items(&options)
        .default(0)
        .clear(true)
        .interact_opt();
    utils::clear();

    match selection {
        Ok(index) => match index {
            Some(index) => {
                let anime = &options[index];
                println!("Selected anime: {}", style(anime).blue());
                let id_to_update = settings[index].id;
                let mut intro = settings[index].intro;
                let mut outro = settings[index].outro;
                let mut recap = settings[index].recap;
                let mut filler = settings[index].filler;

                let options = vec!["Intro", "Outro", "Recap", "Filler"];
                let selection = MultiSelect::with_theme(&theme)
                    .with_prompt("What overrides do you want to enable?")
                    .item_checked(&options[0], intro)
                    .item_checked(&options[1], outro)
                    .item_checked(&options[2], recap)
                    .item_checked(&options[3], filler)
                    .interact_opt()
                    .unwrap();
                utils::clear();
                if selection.is_none() {
                    println!("See you later!");
                    process::exit(0);
                } else {
                    let selected = selection.unwrap();
                    for i in selected {
                        if i == 0 {
                            intro = true;
                        } else if i == 1 {
                            outro = true;
                        } else if i == 2 {
                            recap = true;
                        }
                        if i == 3 {
                            filler = true;
                        }
                    }
                    add_override(id_to_update, intro, outro, recap, filler);
                    println!("Overrides updated!");
                    Ok(())
                }
            },
            None => Err(anyhow::anyhow!("No selection made"))
        },
        Err(e) => {
            Err(anyhow::anyhow!("Selection failed: {}", e))
        }
    }
}
