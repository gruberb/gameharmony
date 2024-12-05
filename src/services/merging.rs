use crate::domain::storage::Storage;
use crate::error::Result;
use crate::services::scraping::WebsiteGames;
use crate::services::text_utils::TitleNormalizer;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;
use crate::config::ScraperConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedGame {
    pub normalized_name: String,
    pub original_names: Vec<String>,
    pub rankings: HashMap<String, u64>,
}

struct GameData {
    original_name: String,
    normalized_title: String,
    numeric_tokens: Vec<String>,
    non_numeric_title: String,
    rank: u64,
    source: String,
}

pub struct MergingService {
    store: Arc<dyn Storage>,
    scraper_config: ScraperConfig,
}

impl MergingService {
    pub fn new(store: Arc<dyn Storage + 'static>, scraper_config: &ScraperConfig) -> Self {
        info!("Created new Merging Service");
        Self { store, scraper_config: scraper_config.clone() }
    }

    pub fn merge_games(&self, website_games: Vec<WebsiteGames>) -> Result<Vec<MergedGame>> {
        // Try to load from cache first
        if let Some(cached) = self.store.load_merged_games()? {
            return Ok(cached);
        }

        let games_data = self.prepare_game_data(&website_games);
        let merged_games = self.perform_merge(games_data);

        // Cache the results
        self.store.save_merged_games(&merged_games)?;

        Ok(merged_games)
    }

    fn prepare_game_data(&self, website_games: &[WebsiteGames]) -> Vec<GameData> {
        let mut games_data = Vec::new();
        let numbers_re = Regex::new(r"\b\d+\b").unwrap();

        for website in website_games {
            let source = TitleNormalizer::normalize_source(&website.source, &self.scraper_config);
            info!("Processing games from {}", source);

            for game in &website.games {
                let normalized_title = TitleNormalizer::normalize(&game.name);

                let numeric_tokens = numbers_re
                    .find_iter(&normalized_title)
                    .map(|m| m.as_str().to_string())
                    .collect();

                let non_numeric_title = numbers_re
                    .replace_all(&normalized_title, "")
                    .to_string()
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

        games_data
    }

    fn perform_merge(&self, games_data: Vec<GameData>) -> Vec<MergedGame> {
        // Group by non-numeric title
        let mut title_groups: HashMap<String, Vec<GameData>> = HashMap::new();
        for game in games_data {
            title_groups
                .entry(game.non_numeric_title.clone())
                .or_default()
                .push(game);
        }

        let mut merged_games = Vec::new();

        for group in title_groups.values() {
            let mut merged_group: HashMap<String, MergedGame> = HashMap::new();

            for game in group {
                let key = game.numeric_tokens.join("_");

                if let Some(existing_game) = merged_group.get_mut(&key) {
                    self.update_existing_game(existing_game, game);
                } else {
                    self.create_new_merged_game(&mut merged_group, game, &key);
                }
            }

            merged_games.extend(merged_group.into_values());
        }

        merged_games
    }

    fn update_existing_game(&self, existing_game: &mut MergedGame, game: &GameData) {
        if !existing_game.original_names.contains(&game.original_name) {
            existing_game
                .original_names
                .push(game.original_name.clone());
        }
        existing_game
            .rankings
            .insert(game.source.clone(), game.rank);
    }

    fn create_new_merged_game(
        &self,
        merged_group: &mut HashMap<String, MergedGame>,
        game: &GameData,
        key: &str,
    ) {
        let mut rankings = HashMap::new();
        rankings.insert(game.source.clone(), game.rank);

        merged_group.insert(
            key.to_string(),
            MergedGame {
                normalized_name: game.normalized_title.clone(),
                original_names: vec![game.original_name.clone()],
                rankings,
            },
        );
    }
}
