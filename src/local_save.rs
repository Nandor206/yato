// =============== Imports ================
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use log;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchProgress {
    pub anilist_id: i32,
    pub episode: u32,
    pub position: f64,
    pub scraper_ids: HashMap<String, String>, // language -> scraper_id
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProgressDatabase {
    pub entries: Vec<WatchProgress>,
}

impl ProgressDatabase {
    pub fn config_path() -> Result<PathBuf> {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("yato");
        create_dir_all(&path)?;
        path.push("progress.json");
        Ok(path)
    }

    pub fn load() -> Result<Self> {
        log::info!("Loading progress database");
        let path = Self::config_path()?;
        if !path.exists() {
            log::info!("Progress database file does not exist, returning default");
            return Ok(Self::default());
        }

        let mut file = File::open(path.clone())
            .with_context(|| format!("Failed to open progress database file: {:?}", path))?;
        let mut json = String::new();
        file.read_to_string(&mut json)
            .with_context(|| "Failed to read progress database file")?;
        let db: ProgressDatabase = serde_json::from_str(&json)
            .with_context(|| "Failed to parse progress database JSON")?;
        log::info!("Progress database loaded successfully");
        Ok(db)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path.clone())
            .with_context(|| format!("Failed to create progress database file: {:?}", path))?;
        file.write_all(json.as_bytes())
            .with_context(|| "Failed to write progress database to file")?;
        Ok(())
    }

    pub fn update_or_add(&mut self, anilist_id: i32, episode: u32, position: f64, language: &str, scraper_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.anilist_id == anilist_id) {
            entry.position = position;
            entry.scraper_ids.insert(language.to_string(), scraper_id.to_string());
        } else {
            let mut scraper_ids = HashMap::new();
            scraper_ids.insert(language.to_string(), scraper_id.to_string());

            self.entries.push(WatchProgress {
                anilist_id,
                episode,
                position,
                scraper_ids,
            });
        }
    }

    pub fn get_entry(&self, anilist_id: i32) -> Option<&WatchProgress> {
        self.entries.iter().find(|entry| entry.anilist_id == anilist_id)
    }

    pub fn get_scraper_id(&self, anilist_id: i32, language: &str) -> Option<&str> {
        self.get_entry(anilist_id)
            .and_then(|entry| entry.scraper_ids.get(language).map(String::as_str))
    }
}
