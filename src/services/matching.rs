use crate::domain::storage::Storage;
use crate::error::{GameError, Result};
use crate::infrastructure::SteamApp;
use crate::services::merging::MergedGame;
use crate::services::text_utils::TitleNormalizer;
use ahash::AHashMap;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use strsim::normalized_levenshtein;
use tokio::time::Instant;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWithSteamId {
    pub name: String,
    pub rankings: HashMap<String, u64>,
    pub steam_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedGames {
    pub created_at: u64,
    pub name_index: HashMap<String, IndexedGame>,
    pub letter_index: HashMap<char, Vec<(IndexedGame, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedGame {
    pub appid: u64,
    pub name: String,
}

pub struct MatchingConfig {
    pub similarity_threshold: f64,
    pub dlc_pattern: String,
    pub filter_dlc: bool,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.9,
            dlc_pattern: String::from(
                r"(?i)(DLC|Soundtrack|OST|Bonus|Season Pass|Content Pack|\bVR\b|\bBeta\b|\bDemo\b|\bArt\sof\b|\bUpgrade\b|\bPack\b|\bBundle\b)",
            ),
            filter_dlc: true,
        }
    }
}

// Internal structure used during index building
struct AppIndex {
    name_index: FxHashMap<String, Arc<SteamApp>>,
    letter_index: AHashMap<char, Vec<(Arc<SteamApp>, String)>>,
}

impl AppIndex {
    fn build_index(
        steam_apps: Vec<SteamApp>,
        dlc_pattern: &str,
        should_filter: bool,
    ) -> Result<Self> {
        let dlc_pattern = Regex::new(dlc_pattern)
            .map_err(|e| GameError::Other(format!("Invalid regex pattern: {}", e)))?;

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
            .filter(|app| {
                if should_filter {
                    let filtered = dlc_pattern.is_match(&app.name);
                    !filtered
                } else {
                    true
                }
            })
            .map(|app| {
                let app = Arc::new(app);
                let normalized = TitleNormalizer::normalize(&app.name);
                (app, normalized)
            })
            .collect();

        info!("After filtering: {} apps", processed_apps.len());

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

    fn create_indexed_games(&self) -> IndexedGames {
        IndexedGames {
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            name_index: self
                .name_index
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        IndexedGame {
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
                    let entries = v
                        .iter()
                        .map(|(app, s)| {
                            (
                                IndexedGame {
                                    appid: app.appid,
                                    name: app.name.clone(),
                                },
                                s.clone(),
                            )
                        })
                        .collect();
                    (*k, entries)
                })
                .collect(),
        }
    }
}

pub struct MatchingService {
    pub name_index: FxHashMap<String, Arc<SteamApp>>,
    pub letter_index: AHashMap<char, Vec<(Arc<SteamApp>, String)>>,
    store: Arc<dyn Storage>,
    config: MatchingConfig,
}

impl MatchingService {
    pub fn new(
        steam_apps: Vec<SteamApp>,
        store: Arc<dyn Storage>,
        config: MatchingConfig,
    ) -> Result<Self> {
        let index_data = match store.load_indexed_games()? {
            Some(cached) => {
                info!("Found cached indexed games");
                cached
            }
            None => {
                info!("Building new index");
                let app_index =
                    AppIndex::build_index(steam_apps, &config.dlc_pattern, config.filter_dlc)?;
                let index_data = app_index.create_indexed_games();
                store.save_indexed_games(&index_data)?;
                index_data
            }
        };

        Ok(Self::from_indexed_games(index_data, store, config))
    }

    fn from_indexed_games(
        indexed: IndexedGames,
        store: Arc<dyn Storage>,
        config: MatchingConfig,
    ) -> Self {
        let name_index = indexed
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

        let letter_index = indexed
            .letter_index
            .into_iter()
            .map(|(k, v)| {
                let entries = v
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
                (k, entries)
            })
            .collect();

        Self {
            name_index,
            letter_index,
            store,
            config,
        }
    }

    pub async fn match_games(&self, merged_games: Vec<MergedGame>) -> Result<Vec<GameWithSteamId>> {
        if let Some(cached) = self.store.load_matched_games()? {
            info!("Using cached matched games");
            return Ok(cached);
        }

        info!("Matching games with Steam IDs in parallel");
        let matched_games: Vec<GameWithSteamId> = merged_games
            .into_par_iter()
            .map(|game| {
                let steam_id = self.find_steam_id(&game.original_names[0]);

                if steam_id.is_none() {
                    info!("No Steam ID found for: {}", game.original_names[0]);
                }
                GameWithSteamId {
                    name: game.original_names[0].clone(),
                    rankings: game.rankings,
                    steam_id,
                }
            })
            .collect();

        Ok(matched_games)
    }

    pub fn find_steam_id(&self, game_name: &str) -> Option<String> {
        info!("Finding Steam ID for: {}", game_name);
        let normalized_search = TitleNormalizer::normalize(game_name);

        // Try exact match first
        if let Some(app) = self.name_index.get(&normalized_search) {
            return Some(app.appid.to_string());
        }

        // Fuzzy matching if exact match fails
        let first_char = normalized_search.chars().next()?;
        let candidates = self.letter_index.get(&first_char)?;

        candidates
            .par_iter()
            .map(|(app, normalized_name)| {
                let similarity = normalized_levenshtein(&normalized_search, normalized_name);
                (app, similarity)
            })
            .filter(|(_, similarity)| *similarity > self.config.similarity_threshold)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(app, _)| app.appid.to_string())
    }
}
