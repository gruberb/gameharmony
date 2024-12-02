mod game_entry;
mod manifest;
mod processor;

pub use self::game_entry::GameEntry;
use self::manifest::Manifest;
use processor::GameProcessor;

use crate::clients::rawg::RawgClient;
use crate::clients::steam::SteamClient;
use crate::error::{GameError, Result};
use crate::scrapers::config::Config;
use chrono::Utc;
use reqwest::Client;
use std::path::PathBuf;

pub struct Processor {
    processor: GameProcessor,
    data_dir: PathBuf,
}

impl Processor {
    pub async fn new(config_file: &str) -> Result<Self> {
        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_file)?)?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(GameError::Network)?;

        let rawg_api_key =
            std::env::var("RAWG_API_KEY").expect("RAWG_API_KEY environment variable not set");

        let processor = GameProcessor::new(
            config,
            client.clone(),
            SteamClient::new(client.clone()).await?,
            RawgClient::new(client, rawg_api_key),
        );

        Ok(Self {
            processor,
            data_dir: PathBuf::from("data"),
        })
    }

    pub async fn run(&self) -> Result<()> {
        self.ensure_dirs()?;

        let games = self.processor.process().await?;
        self.save_manifest(games)?;

        Ok(())
    }

    fn ensure_dirs(&self) -> Result<()> {
        let cache_dir = self.processor.cache_dir();
        if !cache_dir.exists() {
            std::fs::create_dir_all(cache_dir)?;
        }
        if !self.data_dir.exists() {
            std::fs::create_dir_all(&self.data_dir)?;
        }
        Ok(())
    }

    fn save_manifest(&self, games: Vec<GameEntry>) -> Result<()> {
        let manifest = Manifest::new(games);

        // Get current timestamp
        let timestamp = Utc::now().timestamp();
        let filename = format!("manifest_{}.json", timestamp);
        let manifest_path = self.data_dir.join(&filename);

        // Write the manifest with timestamp in filename
        std::fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        Ok(())
    }
}
