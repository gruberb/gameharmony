use super::GameEntry;
use crate::clients::rawg::RawgClient;
use crate::clients::steam::SteamClient;
use crate::error::{GameError, Result};
use crate::matcher::normalize::{format_display_title, normalize_source};
use crate::matcher::{normalize_title, Game, GameMatcher, MergedGame, WebsiteGames};
use crate::scrapers::config::{Config, Website};
use crate::scrapers::{
    eurogamer::EurogamerScraper, ign::IGNScraper, pcgamer::PCGamerScraper,
    polygon_ps5_top25::PolygonPS5Top25, rockpapershotgun::RPSScraper, Selectors, WebsiteScraper,
};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use regex::Regex;
use reqwest::Client;
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct GameWithSteamId {
    name: String,
    rankings: HashMap<String, i32>,
    steam_id: Option<String>,
    harmony_score: i32,
}

#[derive(Debug)]
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

        // Step 2: Merge games
        info!("Step 2: Getting merged games...");
        let merged_games = self.merge_games(website_games)?;

        let game_matcher = GameMatcher::new(self.steam_client.steam_apps.clone())?;

        // Step 3: Add Steam IDs
        info!("Step 3: Getting games with Steam IDs...");
        let games_with_ids = self.add_steam_ids(game_matcher, merged_games).await?;

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
        } else if url.contains("best-ps5-games-playstation-5") {
            Box::new(PolygonPS5Top25)
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
            info!("Loading cached data for {}", filename);
            let cached_data = std::fs::read_to_string(cache_path.clone())?;
            let merged_games: Vec<MergedGame> = serde_json::from_str(&cached_data)?;
            return Ok(merged_games);
        }

        let mut games_data: Vec<GameData> = Vec::new();

        // Precompute normalized titles and tokens
        for website in website_games {
            let source = normalize_source(&website.source);
            info!("Processing games from {}", source);

            for game in website.games {
                let normalized_title = normalize_title(&game.name);

                // Extract numeric tokens
                let numbers_re = Regex::new(r"\b\d+\b").unwrap();
                let numeric_tokens: Vec<String> = numbers_re
                    .find_iter(&normalized_title)
                    .map(|m| m.as_str().to_string())
                    .collect();

                // Remove numeric tokens to get non-numeric title
                let non_numeric_title = numbers_re.replace_all(&normalized_title, "").to_string();
                let non_numeric_title = non_numeric_title
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join(" ");

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
            title_groups
                .entry(game.non_numeric_title.clone())
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
                        existing_game
                            .original_names
                            .push(game.original_name.clone());
                    }
                    existing_game
                        .rankings
                        .insert(game.source.clone(), game.rank);
                    existing_game.harmony_score = calculate_score(&existing_game.rankings);
                } else {
                    let mut rankings = HashMap::new();
                    rankings.insert(game.source.clone(), game.rank);
                    let harmony_score = calculate_score(&rankings);

                    merged_group.insert(
                        key.clone(),
                        MergedGame {
                            normalized_name: game.normalized_title.clone(),
                            original_names: vec![game.original_name.clone()],
                            rankings,
                            harmony_score,
                        },
                    );
                }
            }

            merged_games.extend(merged_group.into_values());
        }

        std::fs::write(&cache_path, serde_json::to_string_pretty(&merged_games)?)?;

        Ok(merged_games)
    }

    async fn add_steam_ids(
        &self,
        game_matcher: GameMatcher,
        merged_games: Vec<MergedGame>,
    ) -> Result<Vec<GameWithSteamId>> {
        let cache_path = self.cache_dir.join("merged_with_steam_id.json");

        if cache_path.exists() {
            info!("Loading cached games with Steam IDs");
            return Ok(serde_json::from_str(&std::fs::read_to_string(cache_path)?)?);
        }

        info!("Adding Steam IDs and calculating scores for games in parallel");

        let games_with_steam_ids: Vec<GameWithSteamId> = merged_games
            .into_par_iter()
            .map(|game| {
                let steam_id = game_matcher.find_steam_id(&game.original_names[0]);

                // Use the calculate_score function instead of inline calculation
                let harmony_score = calculate_score(&game.rankings);

                info!(
                    "Game: {} - SteamID: {:?} - Score: {:.1}",
                    game.original_names[0], steam_id, harmony_score
                );

                GameWithSteamId {
                    name: game.original_names[0].clone(),
                    rankings: game
                        .rankings
                        .into_iter()
                        .map(|(k, v)| (k, v as i32))
                        .collect(),
                    harmony_score,
                    steam_id,
                }
            })
            .collect();
        // Cache the results
        std::fs::write(
            &cache_path,
            serde_json::to_string_pretty(&games_with_steam_ids)?,
        )?;

        Ok(games_with_steam_ids)
    }

    async fn enrich_games(&self, games_with_ids: Vec<GameWithSteamId>) -> Result<Vec<GameEntry>> {
        let mut enriched_games = Vec::new();

        for game in games_with_ids {
            let mut entry = GameEntry::new(game.name, game.rankings, game.harmony_score);

            if let Some(steam_id) = game.steam_id {
                let steam_id_num = steam_id
                    .parse()
                    .map_err(|_| GameError::Parse("Cannot parse SteamId".to_string()))?;
                entry.steam_id = Some(steam_id_num);

                if let Ok(Some(store_info)) = self.steam_client.get_store_info(steam_id_num).await {
                    entry.update_steam_info(store_info);
                }

                if let Ok(deck_status) = self.steam_client.get_deck_verified(steam_id.clone()).await
                {
                    entry.update_steam_deck_info(deck_status, steam_id);
                }
            }

            if let Ok(Some((basic, detailed))) = self.rawg_client.get_game_info(&entry.title).await
            {
                entry.update_rawg_info(&basic, &detailed);
            }

            entry.title = format_display_title(&entry.title);
            enriched_games.push(entry);
            sleep(std::time::Duration::from_millis(200)).await;
        }

        enriched_games.sort_by(|a, b| {
            b.harmony_score
                .partial_cmp(&a.harmony_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(enriched_games)
    }
}

fn calculate_score(rankings: &HashMap<String, usize>) -> i32 {
    if rankings.is_empty() {
        return 0;
    }

    // Average position score (0-100)
    let position_score: i32 = rankings
        .values()
        .map(|&rank| {
            if rank <= 100 {
                (101 - rank) as i32 // Gives 100 points for #1, down to 1 point for #100
            } else {
                0
            }
        })
        .sum::<i32>()
        / rankings.len() as i32;

    // Appearance multiplier (1.0 to 2.0)
    // With 5 sites, this gives:
    // 1 site:   no bonus (multiplier 1.0)
    // 2 sites:  25% bonus (multiplier 1.25)
    // 3 sites:  50% bonus (multiplier 1.5)
    // 4 sites:  75% bonus (multiplier 1.75)
    // 5 sites:  100% bonus (multiplier 2.0)
    let appearance_multiplier = 100 + (25 * (rankings.len() - 1)) as i32;

    // Final score: position_score * appearance_multiplier / 100
    position_score * appearance_multiplier / 100
}
