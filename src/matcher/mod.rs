pub mod normalize;

use crate::clients::steam::SteamApp;
use ahash::AHashMap;
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use strsim::normalized_levenshtein;
use tracing::info;

pub use normalize::normalize_title;

pub struct GameMatcher {
    name_index: FxHashMap<String, Arc<SteamApp>>,
    letter_index: AHashMap<char, Vec<Arc<SteamApp>>>,
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

impl GameMatcher {
    pub fn new(steam_apps: Vec<SteamApp>) -> Self {
        static DLC_PATTERN: OnceCell<Regex> = OnceCell::new();
        let dlc_regex = DLC_PATTERN.get_or_init(|| {
            Regex::new(r"(?i)(DLC|Soundtrack|OST|Bonus|Season Pass|Content Pack|\bVR\b|\bBeta\b|\bDemo\b|\bArt\sof\b|\bUpgrade\b|\bEdition\b|\bPack\b|\bBundle\b)").unwrap()
        });

        info!("Build filtered apps");
        let filtered_apps: Vec<_> = steam_apps
            .into_iter()
            .filter(|app| !dlc_regex.is_match(&app.name))
            .collect();

        let apps = Arc::new(filtered_apps);
        let mut name_index = FxHashMap::default();
        let mut letter_index = AHashMap::new();

        info!("Build letter index");
        for app in apps.iter() {
            let app = Arc::new(app.clone());
            let normalized = normalize_title(&app.name);

            name_index.insert(normalized.clone(), Arc::clone(&app));

            if let Some(first_char) = normalized.chars().next() {
                letter_index
                    .entry(first_char)
                    .or_insert_with(Vec::new)
                    .push(Arc::clone(&app));
            }
        }

        Self {
            name_index,
            letter_index,
        }
    }

    pub fn find_steam_id(&self, game_name: &str) -> Option<String> {
        info!("Find SteamID for: {game_name}");
        let normalized_search = normalize_title(game_name);

        // 1. Try exact match first
        if let Some(app) = self.name_index.get(&normalized_search) {
            return Some(app.appid.to_string());
        }

        // 2. Fuzzy match only on relevant subset
        const SIMILARITY_THRESHOLD: f64 = 0.9;
        let first_char = normalized_search.chars().next()?;

        // Get candidates sharing the same first letter
        let candidates = self.letter_index.get(&first_char)?;

        // Parallel fuzzy matching on the subset
        candidates
            .par_iter()
            .map(|app| {
                let similarity =
                    normalized_levenshtein(&normalized_search, &normalize_title(&app.name));
                (app, similarity)
            })
            .filter(|(_, similarity)| *similarity > SIMILARITY_THRESHOLD)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(app, _)| app.appid.to_string())
    }
}
