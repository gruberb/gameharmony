mod clients;
mod config;
mod error;
mod scrapers;
mod utils;

use reqwest::Client;
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use crate::clients::protondb::ProtonDBClient;
use crate::clients::rawg::RawgClient;
use crate::clients::steam::{SteamClient, StorePlatforms};
use crate::config::{Config, Website};
use crate::error::{GameError, Result};
use crate::utils::{are_titles_same_game, get_normalization_map, normalize_title};

use crate::scrapers::eurogamer::EurogamerScraper;
use crate::scrapers::pcgamer::PCGamerScraper;
use crate::scrapers::{ign::IGNScraper, rockpapershotgun::RPSScraper, Selectors, WebsiteScraper};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameEntry {
    pub title: String,
    pub rankings: HashMap<String, i32>,
    pub platforms: StorePlatforms,
    pub stores: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_score: Option<f64>,
    #[serde(default)]
    pub total_reviews: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reddit_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct Manifest {
    total_games: usize,
    last_updated: String,
    games: Vec<GameEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameRanking {
    name: String,
    rank: i32,
}

// Step 1: Website Scraping Structures
#[derive(Debug, Serialize, Deserialize)]
struct WebsiteGames {
    source: String,
    games: Vec<GameRanking>,
}

// Step 2: Merged Names Structure
#[derive(Debug, Serialize, Deserialize)]
struct MergedGame {
    normalized_name: String,
    original_names: Vec<String>,
    rankings: HashMap<String, i32>,
}

// Step 3: Games with Steam IDs
#[derive(Debug, Serialize, Deserialize)]
struct GameWithSteamId {
    name: String,
    rankings: HashMap<String, i32>,
    steam_id: Option<String>,
}

struct GameHarmony {
    config: Config,
    client: Client,
    steam_client: SteamClient,
    rawg_client: RawgClient,
    protondb_client: ProtonDBClient,
    data_dir: PathBuf,
}

impl GameHarmony {
    async fn new(config_file: &str) -> Result<Self> {
        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_file)?)?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(GameError::Network)?;

        // Get RAWG API key from environment
        let rawg_api_key =
            env::var("RAWG_API_KEY").expect("RAWG_API_KEY environment variable not set");

        let steam_client = SteamClient::new(client.clone()).await?;
        let rawg_client = RawgClient::new(client.clone(), rawg_api_key);
        let protondb_client = ProtonDBClient::new(client.clone());

        Ok(Self {
            config,
            client,
            steam_client,
            rawg_client,
            protondb_client,
            data_dir: PathBuf::from("data"),
        })
    }

    fn get_cache_dir(&self) -> PathBuf {
        PathBuf::from("cache")
    }

    fn ensure_dirs(&self) -> Result<()> {
        let cache_dir = self.get_cache_dir();
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }
        if !self.data_dir.exists() {
            std::fs::create_dir_all(&self.data_dir)?;
        }
        Ok(())
    }

    fn get_scraper(&self, url: &str) -> Box<dyn WebsiteScraper> {
        if url.contains("ign.com") {
            Box::new(IGNScraper)
        } else if url.contains("rockpapershotgun.com") {
            Box::new(RPSScraper)
        } else if url.contains("eurogamer.net") {
            Box::new(EurogamerScraper)
        } else if url.contains("pcgamer.com") {
            Box::new(PCGamerScraper)
        } else {
            // Default scraper or handle error
            Box::new(RPSScraper) // temporary default
        }
    }

    async fn scrape_website(&self, website: &Website) -> Result<Vec<(String, i32)>> {
        let response = self.client.get(&website.url).send().await?.text().await?;

        let document = Html::parse_document(&response);
        let selectors = Selectors::new(&website.name_selector, &website.rank_selector)?;

        let scraper = self.get_scraper(&website.url);
        scraper.extract_games(&document, &selectors)
    }

    // Step 1: Scrape each website
    async fn scrape_individual_websites(&self) -> Result<Vec<WebsiteGames>> {
        self.ensure_dirs()?;
        let mut all_website_games = Vec::new();

        for website in &self.config.websites {
            let filename = format!("{}.json", website.url.replace("/", "_"));
            let cache_path = self.get_cache_dir().join(&filename);

            let website_games = if cache_path.exists() {
                info!("Loading cached data for {}", website.url);
                serde_json::from_str(&std::fs::read_to_string(cache_path)?)?
            } else {
                info!("Scraping {}", website.url);
                let games = self.scrape_website(website).await?;

                let website_games = WebsiteGames {
                    source: website.url.clone(),
                    games: games
                        .into_iter()
                        .map(|(name, rank)| GameRanking { name, rank })
                        .collect(),
                };

                // Save to cache
                std::fs::write(&cache_path, serde_json::to_string_pretty(&website_games)?)?;

                website_games
            };

            all_website_games.push(website_games);
            sleep(Duration::from_secs(1)).await;
        }

        Ok(all_website_games)
    }

    // Step 2: Merge games
    async fn merge_games(&self, website_games: Vec<WebsiteGames>) -> Result<Vec<MergedGame>> {
        let cache_path = self.get_cache_dir().join("merged_names.json");

        if cache_path.exists() {
            info!("Loading cached merged games");
            return Ok(serde_json::from_str(&std::fs::read_to_string(cache_path)?)?);
        }

        info!("Merging games from all sources");
        let mut merged_games: HashMap<String, MergedGame> = HashMap::new();

        for website in website_games {
            for game in website.games {
                let normalized_name = normalize_title(&game.name);

                let mut found_match = false;
                let existing_keys: Vec<String> = merged_games.keys().cloned().collect();

                for existing_key in existing_keys {
                    if are_titles_same_game(&existing_key, &normalized_name) {
                        if let Some(existing_game) = merged_games.get_mut(&existing_key) {
                            // If the new name is in the normalization map, prefer it
                            let norm_map = get_normalization_map();
                            if norm_map.contains_key(normalized_name.as_str()) {
                                existing_game.normalized_name = normalized_name.clone();
                            }

                            if !existing_game.original_names.contains(&game.name) {
                                existing_game.original_names.push(game.name.clone());
                            }
                            existing_game
                                .rankings
                                .insert(website.source.clone(), game.rank);
                            found_match = true;
                            break;
                        }
                    }
                }

                if !found_match {
                    let mut rankings = HashMap::new();
                    rankings.insert(website.source.clone(), game.rank);

                    merged_games.insert(
                        normalized_name.clone(),
                        MergedGame {
                            normalized_name,
                            original_names: vec![game.name],
                            rankings,
                        },
                    );
                }
            }
        }

        let merged_vec: Vec<MergedGame> = merged_games.into_values().collect();

        std::fs::write(&cache_path, serde_json::to_string_pretty(&merged_vec)?)?;

        Ok(merged_vec)
    }

    // Step 3: Add Steam IDs
    async fn add_steam_ids(&self, merged_games: Vec<MergedGame>) -> Result<Vec<GameWithSteamId>> {
        let cache_path = self.get_cache_dir().join("merged_with_steam_id.json");

        if cache_path.exists() {
            info!("Loading cached games with Steam IDs");
            return Ok(serde_json::from_str(&std::fs::read_to_string(cache_path)?)?);
        }

        info!("Adding Steam IDs to games");
        let mut games_with_steam_ids = Vec::new();

        for game in merged_games {
            let steam_id = self.steam_client.find_steam_id(&game.original_names[0]);

            let game_with_id = GameWithSteamId {
                name: game.original_names[0].clone(),
                rankings: game.rankings,
                steam_id,
            };

            games_with_steam_ids.push(game_with_id);
        }

        // Save to cache
        std::fs::write(
            &cache_path,
            serde_json::to_string_pretty(&games_with_steam_ids)?,
        )?;

        Ok(games_with_steam_ids)
    }

    // Step 4: Add ProtonDB and RAWG data (final manifest)
    async fn add_additional_data(&self, games_with_ids: Vec<GameWithSteamId>) -> Result<()> {
        info!("Adding ProtonDB and RAWG data");
        let mut final_games = Vec::new();

        for game in games_with_ids {
            let mut entry = GameEntry {
                title: game.name,
                rankings: game.rankings,
                platforms: StorePlatforms {
                    windows: true,
                    macos: false,
                    linux: false,
                    steamdeck: "unknown".to_string(),
                    switch: false,
                },
                stores: Vec::new(),
                steam_id: None,
                user_score: None,
                total_reviews: 0,
                price: None,
                header_image: None,
                metacritic: None,
                release_date: None,
                reddit_url: None,
                metacritic_url: None,
            };

            if let Some(steam_id) = game.steam_id {
                let steam_id_num = steam_id
                    .parse()
                    .map_err(|_| GameError::Parse("Cannot parse SteamId".to_string()))?;
                entry.steam_id = Some(steam_id_num);

                if let Ok(protondb_data) = self.protondb_client.get_protondb_data(&steam_id).await {
                    entry.platforms.steamdeck = protondb_data.tier;
                }

                if let Ok(Some(store_info)) = self.steam_client.get_store_info(steam_id_num).await {
                    entry.price = store_info.price;
                    entry.platforms = store_info.platforms;
                    entry.user_score = store_info.user_score;
                    entry.total_reviews = store_info.total_reviews;
                    entry.header_image = store_info.header_image;
                    entry.stores.push("Steam".to_string());
                }
            }

            if let Ok(Some((basic, detailed))) = self.rawg_client.get_game_info(&entry.title).await
            {
                if entry.header_image.is_none() {
                    entry.header_image = basic.background_image;
                }

                for platform in &basic.platforms {
                    match platform.platform.name.as_str() {
                        "PC" => entry.platforms.windows = true,
                        "macOS" => entry.platforms.macos = true,
                        "Linux" => entry.platforms.linux = true,
                        "Nintendo Switch" => entry.platforms.switch = true,
                        _ => {}
                    }
                }

                if let Some(stores) = basic.stores {
                    entry
                        .stores
                        .extend(stores.into_iter().map(|s| s.store.name));
                    entry.stores.sort();
                    entry.stores.dedup();
                }

                entry.metacritic = detailed.metacritic;
                entry.release_date = detailed.released;
                entry.reddit_url = detailed.reddit_url;
                entry.metacritic_url = detailed.metacritic_url;
            }

            final_games.push(entry);
            sleep(Duration::from_secs(1)).await;
        }

        // Create and save final manifest
        let manifest = Manifest {
            total_games: final_games.len(),
            last_updated: chrono::Local::now().to_rfc3339(),
            games: final_games,
        };

        std::fs::write(
            self.data_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;

        Ok(())
    }
    pub async fn run(&self) -> Result<()> {
        // Step 1: Get website data (from cache or scrape)
        info!("Step 1: Getting website data...");
        let website_games = self.scrape_individual_websites().await?;

        // Step 2: Get merged games (from cache or merge)
        info!("Step 2: Getting merged games...");
        let merged_games = self.merge_games(website_games).await?;

        // Step 3: Get games with Steam IDs (from cache or add)
        info!("Step 3: Getting games with Steam IDs...");
        let games_with_ids = self.add_steam_ids(merged_games).await?;

        // Step 4: Create final manifest
        info!("Step 4: Creating final manifest...");
        self.add_additional_data(games_with_ids).await?;

        info!("All steps completed successfully!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let harmony = GameHarmony::new("scraper_config.json").await?;
    harmony.run().await?;

    info!("Scraping completed successfully!");
    Ok(())
}
