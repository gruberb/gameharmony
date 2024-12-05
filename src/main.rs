mod config;
mod domain;
mod error;
mod infrastructure;
mod services;

use crate::config::cli::{Args, Commands};
use crate::config::Config;
use crate::domain::storage::Storage;
use crate::error::Result;
use crate::infrastructure::FileSystemStore;
use crate::infrastructure::RawgClient;
use crate::infrastructure::SteamClient;
use crate::services::enrichment::Enrichment;
use crate::services::game_service::GameService;
use crate::services::matching::{MatchingConfig, MatchingService};
use crate::services::merging::MergingService;
use crate::services::publish::PublishService;
use crate::services::scraping::ScrapingService;
use clap::Parser;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt::init();

    match &args.command {
        Some(Commands::Publish {
            manifest,
            username,
            repo,
        }) => {
            let prepare_service = PublishService::new(username.clone(), repo.clone());
            prepare_service.prepare(manifest).await?;
        }
        None => {
            let config = Config::new()?;
            config.ensure_directories()?;

            let store: Arc<dyn Storage> = Arc::new(FileSystemStore::new(
                config.args.data_dir.clone(),
                config.args.cache_dir.clone(),
            ));

            let steam_client =
                SteamClient::new(config.http_client.clone(), Arc::clone(&store)).await?;
            let scraping = ScrapingService::new(config.http_client.clone());
            let merging = MergingService::new(Arc::clone(&store));
            let matching = MatchingService::new(
                steam_client.steam_apps.clone(),
                Arc::clone(&store),
                MatchingConfig::default(),
            )?;
            let enrichment = Enrichment::new(
                steam_client,
                RawgClient::new(
                    config.http_client.clone(),
                    config
                        .args
                        .rawg_api_key
                        .clone()
                        .expect("No RAWG API key given"),
                    Arc::clone(&store),
                ),
                Arc::clone(&store),
            );
            let service = GameService::new(
                config,
                Arc::clone(&store),
                scraping,
                merging,
                matching,
                enrichment,
            );
            service.process().await?;
        }
    }

    Ok(())
}
