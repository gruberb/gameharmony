use crate::config::Config;
use crate::domain::storage::Storage;
use crate::domain::{Game, GameWithSteamId, Manifest, MergedGame, WebsiteGames};
use crate::error::Result;
use crate::services::{
    enrichment::Enrichment, matching::MatchingService, merging::MergingService,
    scraping::ScrapingService,
};
use std::sync::Arc;
use tracing::info;

pub struct GameService {
    config: Config,
    store: Arc<dyn Storage>,
    scraping: ScrapingService,
    merging: MergingService,
    matching: MatchingService,
    enrichment: Enrichment,
}

impl GameService {
    pub fn new(
        config: Config,
        store: Arc<dyn Storage + 'static>,
        scraping: ScrapingService,
        merging: MergingService,
        matching: MatchingService,
        enrichment: Enrichment,
    ) -> Self {
        Self {
            config,
            store,
            scraping,
            merging,
            matching,
            enrichment,
        }
    }

    pub async fn process(&self) -> Result<()> {
        info!("Starting game data processing pipeline");

        let website_games = self.scrape_websites().await?;
        info!(
            "Website games processing completed: {} sources",
            website_games.len()
        );

        let merged_games = self.merge_games(website_games).await?;
        info!(
            "Game merging completed: {} unique games",
            merged_games.len()
        );

        let games_with_steam = self.add_steam_ids(merged_games).await?;
        info!("Steam matching completed");

        let enriched_games = self.enrich_games(games_with_steam).await?;
        info!("Game enrichment completed");

        self.save_final_manifest(enriched_games).await?;
        info!("Processing pipeline completed successfully");

        Ok(())
    }

    /// This method is going through all sources in the `scraper_config.json`,
    /// and fetches all sources. We first check if we already fetched the source
    /// previously, and if so, take it from the file in the cache folder.
    ///
    /// If `skip-cache` is set via the CLI, we always fetch from remote.
    async fn scrape_websites(&self) -> Result<Vec<WebsiteGames>> {
        let mut website_games: Vec<WebsiteGames> = Vec::new();
        let mut to_scrape = Vec::new();

        if !self.config.args.skip_cache {
            for website in self.config.scraper_config.websites.clone() {
                if let Some(website) = self.store.load_website_games(website.clone().url)? {
                    website_games.push(website);
                } else {
                    to_scrape.push(website);
                }
            }

            let mut games = self.scraping.scrape_all(&to_scrape).await?;

            games.extend(website_games);

            self.store.save_website_games(&games)?;
            return Ok(games);
        }

        let games = self
            .scraping
            .scrape_all(&self.config.scraper_config.websites)
            .await?;

        self.store.save_website_games(&games)?;

        Ok(games)
    }

    async fn merge_games(&self, website_games: Vec<WebsiteGames>) -> Result<Vec<MergedGame>> {
        if !self.config.args.skip_cache {
            if let Some(games) = self.store.load_merged_games()? {
                info!("Using cached merged games data");
                return Ok(games);
            }
        }

        let games = self.merging.merge_games(website_games)?;
        self.store.save_merged_games(&games)?;
        Ok(games)
    }

    async fn add_steam_ids(&self, merged_games: Vec<MergedGame>) -> Result<Vec<GameWithSteamId>> {
        if !self.config.args.skip_cache {
            if let Some(games) = self.store.load_matched_games()? {
                info!("Using cached Steam-matched games data");
                return Ok(games);
            }
        }

        let games = self.matching.match_games(merged_games).await?;
        self.store.save_matched_games(&games)?;
        Ok(games)
    }

    async fn enrich_games(&self, games_with_steam: Vec<GameWithSteamId>) -> Result<Vec<Game>> {
        if !self.config.args.skip_cache {
            if let Some(games) = self.store.load_enriched_games()? {
                info!("Using cached enriched games data");
                return Ok(games);
            }
        }

        self.enrichment.enrich_games(games_with_steam).await
    }

    async fn save_final_manifest(&self, games: Vec<Game>) -> Result<()> {
        let manifest = Manifest::new(games);
        self.store.save_manifest(&manifest)?;
        Ok(())
    }
}
