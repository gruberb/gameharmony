pub mod normalize;

use crate::clients::steam::SteamApp;
use ahash::AHashMap;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use rayon::prelude::*;
use strsim::normalized_levenshtein;
use tracing::info;
use once_cell::sync::Lazy;
use tokio::time::Instant;

pub use normalize::normalize_title;

pub struct GameMatcher {
    name_index: FxHashMap<String, Arc<SteamApp>>,
    letter_index: AHashMap<char, Vec<(Arc<SteamApp>, String)>>,
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
        static DLC_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"(?i)(DLC|Soundtrack|OST|Bonus|Season Pass|Content Pack|\bVR\b|\bBeta\b|\bDemo\b|\bArt\sof\b|\bUpgrade\b|\bEdition\b|\bPack\b|\bBundle\b)"
            ).unwrap()
        });

        let total_start = Instant::now();
        let mut last_checkpoint = total_start;

        // Logging helper
        let checkpoint = |name: &str, last: &mut Instant| {
            let elapsed = last.elapsed();
            let total = total_start.elapsed();
            info!("{}: {:?} (total: {:?})", name, elapsed, total);
            *last = Instant::now();
        };

        info!("Starting index build for {} apps", steam_apps.len());

        // Step 1: Parallel filtering and normalization
        let processed_apps: Vec<_> = steam_apps
            .into_par_iter()
            .filter(|app| !DLC_PATTERN.is_match(&app.name))
            .map(|app| {
                let app = Arc::new(app);
                let normalized = normalize_title(&app.name);
                (app, normalized)
            })
            .collect();

        checkpoint("Filtering and normalization", &mut last_checkpoint);

        // Step 2: Create indices with pre-allocated capacity
        let capacity = processed_apps.len();
        let mut name_index = FxHashMap::with_capacity_and_hasher(capacity, Default::default());
        let mut letter_index: AHashMap<char, Vec<(Arc<SteamApp>, String)>> = AHashMap::with_capacity(27); // 26 letters + numbers

        // Pre-initialize letter_index buckets
        for c in 'a'..='z' {
            letter_index.insert(c, Vec::with_capacity(capacity / 26));
        }
        letter_index.insert('0', Vec::with_capacity(capacity / 26)); // For numbers

        // Build both indices in a single pass
        for (app, normalized) in processed_apps {
            // Insert into name_index
            name_index.insert(normalized.clone(), Arc::clone(&app));

            // Insert into letter_index with normalized name
            if let Some(first_char) = normalized.chars().next() {
                if let Some(vec) = letter_index.get_mut(&first_char) {
                    vec.push((Arc::clone(&app), normalized.clone()));
                }
            }
        }

        checkpoint("Index building", &mut last_checkpoint);

        // Sort letter_index vectors by normalized name for potential binary search later
        letter_index.par_iter_mut().for_each(|(_, apps)| {
            apps.sort_by(|(_, a), (_, b)| a.cmp(b));
        });

        checkpoint("Sorting letter indices", &mut last_checkpoint);

        info!(
            "Built indices in {:?}. Stats: {} filtered apps, {} unique names, {} first letters",
            total_start.elapsed(),
            name_index.len(),
            name_index.len(),
            letter_index.len()
        );

        Self {
            name_index,
            letter_index,
        }
    }

    pub fn find_steam_id(&self, game_name: &str) -> Option<String> {
        info!("Find SteamID for: {game_name}");

        let normalized_search = normalize_title(game_name);

        // Exact match
        if let Some(app) = self.name_index.get(&normalized_search) {
            return Some(app.appid.to_string());
        }

        let first_char = normalized_search.chars().next()?;
        let candidates = self.letter_index.get(&first_char)?;

        const SIMILARITY_THRESHOLD: f64 = 0.9;

        // Now we use the cached normalized names
        candidates
            .par_iter()
            .map(|(app, normalized_name)| {
                // Use cached normalized name instead of computing it again
                let similarity = normalized_levenshtein(&normalized_search, normalized_name);
                (app, similarity)
            })
            .filter(|(_, similarity)| *similarity > SIMILARITY_THRESHOLD)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(app, _)| app.appid.to_string())
    }
}
