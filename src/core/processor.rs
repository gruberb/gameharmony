use super::GameEntry;
use crate::clients::rawg::RawgClient;
use crate::clients::steam::SteamClient;
use crate::config::{Config, Website};
use crate::error::{GameError, Result};
use crate::matcher::{Game, MergedGame, WebsiteGames, normalize_title};
use crate::scrapers::{
    eurogamer::EurogamerScraper, ign::IGNScraper, pcgamer::PCGamerScraper,
    rockpapershotgun::RPSScraper, Selectors, WebsiteScraper,
};
use reqwest::Client;
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use regex::Regex;
use tokio::time::sleep;
use tracing::{info, trace};
use crate::matcher::normalize::normalize_source;


#[derive(Debug, Serialize, Deserialize)]
struct GameWithSteamId {
    name: String,
    rankings: HashMap<String, i32>,
    steam_id: Option<String>,
}

pub struct GameProcessor {
    config: Config,
    client: Client,
    steam_client: SteamClient,
    rawg_client: RawgClient,
    cache_dir: PathBuf,
}

struct GameData {
    original_name: String,
    normalized_title: String,
    numeric_tokens: Vec<String>,
    non_numeric_title: String,
    rank: usize,
    source: String,
}

impl GameProcessor {
    pub fn new(
        config: Config,
        client: Client,
        steam_client: SteamClient,
        rawg_client: RawgClient,
    ) -> Self {
        Self {
            config,
            client,
            steam_client,
            rawg_client,
            cache_dir: PathBuf::from("cache"),
        }
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    pub async fn process(&self) -> Result<Vec<GameEntry>> {
        // Step 1: Scrape websites
        info!("Step 1: Getting website data...");
        let website_games = self.scrape_websites().await?;

        trace!("Found websites: {:#?}", website_games);

        // Step 2: Merge games
        info!("Step 2: Getting merged games...");
        let merged_games = self.merge_games(website_games)?;

        // Step 3: Add Steam IDs
        info!("Step 3: Getting games with Steam IDs...");
        let games_with_ids = self.add_steam_ids(merged_games).await?;

        // Step 4: Enrich with additional data
        info!("Step 4: Adding additional data...");
        self.enrich_games(games_with_ids).await
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
            Box::new(RPSScraper)
        }
    }

    async fn scrape_website(&self, website: &Website) -> Result<Vec<(String, i32)>> {
        let response = self.client.get(&website.url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let selectors = Selectors::new(&website.name_selector, &website.rank_selector)?;
        let scraper = self.get_scraper(&website.url);
        scraper.extract_games(&document, &selectors)
    }

    async fn scrape_websites(&self) -> Result<Vec<WebsiteGames>> {
        let mut all_website_games = Vec::new();

        for website in &self.config.websites {
            let filename = format!("{}.json", website.url.replace("/", "_"));
            let cache_path = self.cache_dir.join(&filename);

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
                        .map(|(name, rank)| Game {
                            name,
                            rank: rank as usize,
                        })
                        .collect(),
                };

                std::fs::write(&cache_path, serde_json::to_string_pretty(&website_games)?)?;
                website_games
            };

            all_website_games.push(website_games);
            sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(all_website_games)
    }

    fn merge_games(&self, website_games: Vec<WebsiteGames>) -> Result<Vec<MergedGame>> {
        let filename = "merged_games.json";
        let cache_path = self.cache_dir.join(filename);

        if cache_path.exists() {
            println!("Loading cached data for {}", filename);
            let cached_data = std::fs::read_to_string(cache_path.clone())?;
            let merged_games: Vec<MergedGame> = serde_json::from_str(&cached_data)?;
            return Ok(merged_games);
        }

        let mut games_data: Vec<GameData> = Vec::new();

        // Precompute normalized titles and tokens
        for website in website_games {
            let source = normalize_source(&website.source);
            println!("Processing games from {}", source);

            for game in website.games {
                let normalized_title = normalize_title(&game.name);

                // Extract numeric tokens
                let numbers_re = Regex::new(r"\b\d+\b").unwrap();
                let numeric_tokens: Vec<String> = numbers_re.find_iter(&normalized_title)
                    .map(|m| m.as_str().to_string())
                    .collect();

                // Remove numeric tokens to get non-numeric title
                let non_numeric_title = numbers_re.replace_all(&normalized_title, "").to_string();
                let non_numeric_title = non_numeric_title.split_whitespace().collect::<Vec<&str>>().join(" ");

                games_data.push(GameData {
                    original_name: game.name.clone(),
                    normalized_title: normalized_title.clone(),
                    numeric_tokens,
                    non_numeric_title,
                    rank: game.rank,
                    source: source.clone(),
                });
            }
        }

        // Group by non-numeric title
        let mut title_groups: HashMap<String, Vec<GameData>> = HashMap::new();

        for game in games_data {
            title_groups.entry(game.non_numeric_title.clone())
                .or_default()
                .push(game);
        }

        // Merge games within each group
        let mut merged_games: Vec<MergedGame> = Vec::new();

        for group in title_groups.values() {
            let mut merged_group: HashMap<String, MergedGame> = HashMap::new();

            for game in group {
                let key = game.numeric_tokens.join("_"); // Use numeric tokens as key

                if let Some(existing_game) = merged_group.get_mut(&key) {
                    if !existing_game.original_names.contains(&game.original_name) {
                        existing_game.original_names.push(game.original_name.clone());
                    }
                    existing_game.rankings.insert(game.source.clone(), game.rank);
                } else {
                    let mut rankings = HashMap::new();
                    rankings.insert(game.source.clone(), game.rank);

                    merged_group.insert(key.clone(), MergedGame {
                        normalized_name: game.normalized_title.clone(),
                        original_names: vec![game.original_name.clone()],
                        rankings,
                    });
                }
            }

            merged_games.extend(merged_group.into_values());
        }

        std::fs::write(&cache_path, serde_json::to_string_pretty(&merged_games)?)?;

        Ok(merged_games)
    }

    async fn add_steam_ids(&self, merged_games: Vec<MergedGame>) -> Result<Vec<GameWithSteamId>> {
        let cache_path = self.cache_dir.join("merged_with_steam_id.json");

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
                rankings: game
                    .rankings
                    .into_iter()
                    .map(|(k, v)| (k, v as i32))
                    .collect(),
                steam_id,
            };

            games_with_steam_ids.push(game_with_id);
        }

        std::fs::write(
            &cache_path,
            serde_json::to_string_pretty(&games_with_steam_ids)?,
        )?;

        Ok(games_with_steam_ids)
    }

    async fn enrich_games(&self, games_with_ids: Vec<GameWithSteamId>) -> Result<Vec<GameEntry>> {
        let mut enriched_games = Vec::new();

        for game in games_with_ids {
            let mut entry = GameEntry::new(game.name, game.rankings);

            if let Some(steam_id) = game.steam_id {
                let steam_id_num = steam_id
                    .parse()
                    .map_err(|_| GameError::Parse("Cannot parse SteamId".to_string()))?;
                entry.steam_id = Some(steam_id_num);

                if let Ok(deck_status) = self.steam_client.get_deck_verified(steam_id.clone()).await
                {
                    if let Some(results) = deck_status.results {
                        if results.resolved_category > 0 {
                            entry.platforms.steamdeck = "verified".to_string();
                            entry.protondb_url =
                                Some(format!("https://www.protondb.com/app/{}", steam_id));
                        }
                    }
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

            enriched_games.push(entry);
            sleep(std::time::Duration::from_millis(650)).await;
        }

        Ok(enriched_games)
    }
}
