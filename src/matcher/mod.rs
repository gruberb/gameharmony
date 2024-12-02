pub mod normalize;

use crate::clients::steam::SteamApp;
use crate::error::{GameError, Result};
use ahash::AHashMap;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use strsim::normalized_levenshtein;
use tokio::time::Instant;
use tracing::info;

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
    pub harmony_score: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CacheableSteamApp {
    appid: u64,
    name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedIndex {
    created_at: u64,
    // Using standard HashMap for serialization
    name_index: HashMap<String, CacheableSteamApp>,
    // Using standard HashMap for serialization
    letter_index: HashMap<char, Vec<(CacheableSteamApp, String)>>,
}

impl GameMatcher {
    pub fn new(steam_apps: Vec<SteamApp>) -> Result<Self> {
        let cache_path = PathBuf::from("cache/index_apps.json");

        // Try to load from cache first
        if let Ok(cached_data) = Self::load_cache(&cache_path) {
            info!("Using cached index from {:?}", cache_path);
            return Ok(Self {
                name_index: cached_data.name_index,
                letter_index: cached_data.letter_index,
            });
        }

        // If cache isn't available or valid, build new index
        info!("Building new index for {} apps", steam_apps.len());
        let matcher = Self::build_index(steam_apps)?;

        // Save the new index to cache
        matcher.save_cache(&cache_path)?;

        Ok(matcher)
    }

    // Main index building function
    fn build_index(steam_apps: Vec<SteamApp>) -> Result<Self> {
        static DLC_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"(?i)(DLC|Soundtrack|OST|Bonus|Season Pass|Content Pack|\bVR\b|\bBeta\b|\bDemo\b|\bArt\sof\b|\bUpgrade\b|\bEdition\b|\bPack\b|\bBundle\b)"
            ).unwrap()
        });

        let total_start = Instant::now();
        let mut last_checkpoint = total_start;

        // Helper function for logging checkpoints
        let checkpoint = |name: &str, last: &mut Instant| {
            let elapsed = last.elapsed();
            let total = total_start.elapsed();
            info!("{}: {:?} (total: {:?})", name, elapsed, total);
            *last = Instant::now();
        };

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
        let mut letter_index: AHashMap<char, Vec<(Arc<SteamApp>, String)>> =
            AHashMap::with_capacity(27);

        // Pre-initialize letter buckets
        for c in 'a'..='z' {
            letter_index.insert(c, Vec::with_capacity(capacity / 26));
        }
        letter_index.insert('0', Vec::with_capacity(capacity / 26));

        // Build both indices in a single pass
        for (app, normalized) in processed_apps {
            name_index.insert(normalized.clone(), Arc::clone(&app));

            if let Some(first_char) = normalized.chars().next() {
                if let Some(vec) = letter_index.get_mut(&first_char) {
                    vec.push((Arc::clone(&app), normalized.clone()));
                }
            }
        }

        checkpoint("Index building", &mut last_checkpoint);

        // Sort letter indices for potential binary search
        letter_index.par_iter_mut().for_each(|(_, apps)| {
            apps.sort_by(|(_, a), (_, b)| a.cmp(b));
        });

        checkpoint("Sorting letter indices", &mut last_checkpoint);

        Ok(Self {
            name_index,
            letter_index,
        })
    }

    // Cache loading function
    fn load_cache(cache_path: &PathBuf) -> Result<Self> {
        // First check if cache file exists and read it
        let cache_data = std::fs::read_to_string(cache_path)?;
        let cached: CachedIndex = serde_json::from_str(&cache_data)?;

        // Convert back to runtime format
        let name_index = cached
            .name_index
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    Arc::new(SteamApp {
                        appid: v.appid,
                        name: v.name,
                    }),
                )
            })
            .collect();

        let letter_index = cached
            .letter_index
            .into_iter()
            .map(|(k, v)| {
                let converted = v
                    .into_iter()
                    .map(|(app, s)| {
                        (
                            Arc::new(SteamApp {
                                appid: app.appid,
                                name: app.name,
                            }),
                            s,
                        )
                    })
                    .collect();
                (k, converted)
            })
            .collect();

        Ok(Self {
            name_index,
            letter_index,
        })
    }

    fn save_cache(&self, cache_path: &PathBuf) -> Result<()> {
        // Convert to cacheable format
        let cacheable = CachedIndex {
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| GameError::Other(e.to_string()))?
                .as_secs(),
            name_index: self
                .name_index
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        CacheableSteamApp {
                            appid: v.appid,
                            name: v.name.clone(),
                        },
                    )
                })
                .collect(),
            letter_index: self
                .letter_index
                .iter()
                .map(|(k, v)| {
                    let converted = v
                        .iter()
                        .map(|(app, s)| {
                            (
                                CacheableSteamApp {
                                    appid: app.appid,
                                    name: app.name.clone(),
                                },
                                s.clone(),
                            )
                        })
                        .collect();
                    (*k, converted)
                })
                .collect(),
        };

        std::fs::write(cache_path, serde_json::to_string_pretty(&cacheable)?)?;

        Ok(())
    }

    // Game lookup function with fuzzy matching
    pub fn find_steam_id(&self, game_name: &str) -> Option<String> {
        info!("Find SteamID for: {game_name}");
        let normalized_search = normalize_title(game_name);

        // Try exact match first
        if let Some(app) = self.name_index.get(&normalized_search) {
            return Some(app.appid.to_string());
        }

        // Fuzzy matching if exact match fails
        let first_char = normalized_search.chars().next()?;
        let candidates = self.letter_index.get(&first_char)?;

        const SIMILARITY_THRESHOLD: f64 = 0.87;

        candidates
            .par_iter()
            .map(|(app, normalized_name)| {
                let similarity = normalized_levenshtein(&normalized_search, normalized_name);
                (app, similarity)
            })
            .filter(|(_, similarity)| *similarity > SIMILARITY_THRESHOLD)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(app, _)| app.appid.to_string())
    }
}
