mod config;
mod domain;
mod error;
mod infrastructure;
mod services;

use crate::config::Config;
use crate::domain::storage::Storage;
use crate::error::Result;
use crate::infrastructure::FileSystemStore;
use crate::infrastructure::RawgClient;
use crate::infrastructure::SteamClient;
use crate::services::enrichment::Enrichment;
use crate::services::game_service::GameService;
use crate::services::matching::MatchingService;
use crate::services::merging::MergingService;
use crate::services::scraping::ScrapingService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::new()?;

    tracing_subscriber::fmt::init();

    config
        .ensure_directories()
        .expect("Cache and Data dirs could not be created");

    let store: Arc<dyn Storage> = Arc::new(FileSystemStore::new(
        config.args.data_dir.clone(),
        config.args.cache_dir.clone(),
    ));

    let steam_client = SteamClient::new(config.http_client.clone(), Arc::clone(&store)).await?;

    let scraping = ScrapingService::new(config.http_client.clone());
    let merging = MergingService::new(Arc::clone(&store));
    let matching = MatchingService::new(steam_client.steam_apps.clone(), Arc::clone(&store))?;
    let enrichment = Enrichment::new(
        steam_client,
        RawgClient::new(
            config.http_client.clone(),
            config.args.rawg_api_key.clone(),
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

    Ok(())
}
