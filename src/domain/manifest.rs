use crate::domain::game::Game;
use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub total_games: usize,
    pub last_updated: String,
    pub games: Vec<Game>,
    pub metadata: ManifestMetadata,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    pub sources: Vec<String>,
    pub enrichment_used: EnrichmentInfo,
    pub version: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentInfo {
    pub steam: bool,
    pub rawg: bool,
}

impl Manifest {
    pub fn new(games: Vec<Game>) -> Self {
        let sources: Vec<String> = games
            .iter()
            .flat_map(|game| game.rankings.keys().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let enrichment_used = EnrichmentInfo {
            steam: games.iter().any(|g| g.steam_id.is_some()),
            rawg: games.iter().any(|g| g.reddit_url.is_some()),
        };

        Self {
            total_games: games.len(),
            last_updated: Local::now().to_rfc3339(),
            games,
            metadata: ManifestMetadata {
                sources,
                enrichment_used,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}
