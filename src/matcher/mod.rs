mod mapping;
pub mod normalize;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strsim::normalized_levenshtein;
use crate::clients::steam::SteamApp;

pub use normalize::normalize_title;

pub struct GameMatcher {
    mapping_config: MappingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteGames {
    pub source: String,
    pub games: Vec<Game>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergedGame {
    pub normalized_name: String,
    pub original_names: Vec<String>,
    pub rankings: HashMap<String, usize>,
}

pub use self::mapping::MappingConfig;

impl GameMatcher {
    pub fn new() -> Self {
        Self {
            mapping_config: MappingConfig::load(),
        }
    }

    pub fn remember_match(
        &mut self,
        original: String,
        normalized: String,
        steam_id: Option<String>,
    ) {
        self.mapping_config.add_mapping(original, normalized, steam_id);
        if let Err(e) = self.mapping_config.save() {
            tracing::warn!("Failed to save game mappings: {}", e);
        }
    }

    fn is_likely_dlc_or_extra(&self, app_name: &str) -> bool {
        let dlc_indicators = [
            "dlc",
            "soundtrack",
            "ost",
            "playtest",
            "demo",
            "art of",
            "dimension",
            "trailer",
            "upgrade",
            "edition",
            "content",
            "pack",
            "bundle",
            "season pass",
            "skin",
            "cosmetic",
            "vr",
            "beta",
            "intro",
            "video",
            "documentary",
            "making of",
            "digital deluxe",
            "bonus",
            "collection",
            "complete",
            "definitive",
            "guide",              // Added for Prima Guide cases
            "manual",            // Added for game manuals
            "handbook",          // Added for handbooks
            "strategy",          // Added for strategy guides
            "companion",         // Added for companion apps/books
            "artbook",           // Added for art books
            "Prima",             // Specific to Prima guides
            "official guide",    // Generic guide indicator
        ];

        let clean_name = self.clean_name_for_comparison(app_name);

        // Check for common DLC patterns
        if dlc_indicators.iter().any(|&indicator| clean_name.contains(indicator)) {
            return true;
        }

        // Additional pattern checks
        let patterns = [
            // Books/Guides pattern
            r"(?i)(guide|book|manual|handbook)s?$",
            // Additional content pattern
            r"(?i)(add-?on|expansion|dlc|pack)s?$",
            // Soundtrack/OST pattern
            r"(?i)(soundtrack|ost|music|theme)s?$",
            // Bonus content pattern
            r"(?i)(bonus|extra|additional)s?\s+content",
        ];

        for pattern in patterns {
            if regex::Regex::new(pattern).unwrap().is_match(&clean_name) {
                return true;
            }
        }

        false
    }

    fn clean_name_for_comparison(&self, name: &str) -> String {
        // Keep "The" at the start of the name for better matching
        let name = if name.to_lowercase().starts_with("the ") {
            name.to_string()
        } else {
            name.replace("The ", "")
        };

        name.to_lowercase()
            .replace(['™', ':', '®', '(', ')', '-', '.', '\'', '"'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn find_steam_id(
        &mut self,
        game_name: &str,
        steam_apps: &[SteamApp],
    ) -> Option<String> {
        // Check if we already have a mapping
        if let Some(steam_id) = self.mapping_config.get_steam_id(game_name) {
            return Some(steam_id);
        }

        // Normalize the game name
        let normalized_game_name = normalize_title(game_name);

        // Build a HashMap of normalized app names to SteamApp
        // For performance, we can cache this map if it's used multiple times
        let app_map: HashMap<String, &SteamApp> = steam_apps
            .iter()
            .map(|app| {
                let normalized_app_name = normalize_title(&app.name);
                (normalized_app_name, app)
            })
            .collect();

        // First, attempt an exact match
        if let Some(app) = app_map.get(&normalized_game_name) {
            tracing::info!(
                "Exact match found for '{}' with appid {}",
                game_name,
                app.appid
            );

            self.remember_match(
                game_name.to_string(),
                app.name.clone(),
                Some(app.appid.to_string()),
            );

            return Some(app.appid.to_string());
        }

        // If no exact match, proceed to fuzzy matching
        // Calculate similarity with all app names
        let mut best_match: Option<(&SteamApp, f64)> = None;

        for (normalized_app_name, app) in &app_map {
            // Skip DLCs or extras if needed
            if self.is_likely_dlc_or_extra(&app.name) {
                continue;
            }

            let similarity = normalized_levenshtein(&normalized_game_name, normalized_app_name);

            // Adjust the threshold as needed (e.g., 0.8)
            if similarity > 0.8 {
                if let Some((_, best_similarity)) = best_match {
                    if similarity > best_similarity {
                        best_match = Some((app, similarity));
                    }
                } else {
                    best_match = Some((app, similarity));
                }
            }
        }

        if let Some((app, similarity)) = best_match {
            tracing::info!(
                "Fuzzy match found for '{}' with appid {} (similarity: {:.2})",
                game_name,
                app.appid,
                similarity
            );

            self.remember_match(
                game_name.to_string(),
                app.name.clone(),
                Some(app.appid.to_string()),
            );

            return Some(app.appid.to_string());
        }

        // If still no match, return None
        tracing::warn!("No match found for '{}'", game_name);
        None
    }
}