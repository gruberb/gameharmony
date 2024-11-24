use super::GameEntry;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Manifest {
    total_games: usize,
    last_updated: String,
    games: Vec<GameEntry>,
}

impl Manifest {
    pub fn new(games: Vec<GameEntry>) -> Self {
        Self {
            total_games: games.len(),
            last_updated: chrono::Local::now().to_rfc3339(),
            games,
        }
    }
}
