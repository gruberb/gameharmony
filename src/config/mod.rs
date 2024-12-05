use crate::config::cli::Args;
use crate::error::Result;
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;

pub(crate) mod cli;

#[derive(Debug, Clone, Deserialize)]
pub struct Website {
    pub url: String,
    pub name_selector: String,
    pub rank_selector: String,
    pub scraper_type: String,
    pub display_name: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScraperConfig {
    pub websites: Vec<Website>,
}

pub struct Config {
    pub args: Args,
    pub scraper_config: ScraperConfig,
    pub http_client: Client,
}

impl Config {
    pub fn new() -> Result<Self> {
        let args = Args::parse();

        // Only load scraper config if we're doing the main scraping
        let scraper_config = if args.command.is_none() {
            serde_json::from_str(&std::fs::read_to_string(&args.config_file)?)?
        } else {
            ScraperConfig { websites: vec![] }
        };

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        Ok(Self {
            args,
            scraper_config,
            http_client,
        })
    }

    pub fn ensure_directories(&self) -> Result<()> {
        if !self.args.data_dir.exists() {
            std::fs::create_dir_all(&self.args.data_dir)?;
        }
        if !self.args.cache_dir.exists() {
            std::fs::create_dir_all(&self.args.cache_dir)?;
        }

        info!("Data and cache dirs exist");
        Ok(())
    }
}
