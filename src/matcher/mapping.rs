use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameMapping {
    pub original_name: String,
    pub normalized_name: String,
    pub steam_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct MappingConfig {
    pub mappings: HashMap<String, GameMapping>,
}

impl MappingConfig {
    pub fn load() -> Self {
        let config_path = Path::new("game_mappings.json");
        if config_path.exists() {
            let config_str =
                std::fs::read_to_string(config_path).expect("Failed to read game_mappings.json");
            serde_json::from_str(&config_str).expect("Failed to parse game_mappings.json")
        } else {
            Self {
                mappings: HashMap::new(),
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let config_path = Path::new("game_mappings.json");
        let config_str =
            serde_json::to_string_pretty(self).expect("Failed to serialize game mappings");
        std::fs::write(config_path, config_str)
    }

    pub fn add_mapping(&mut self, original: String, normalized: String, steam_id: Option<String>) {
        self.mappings.insert(
            original.clone(),
            GameMapping {
                original_name: original,
                normalized_name: normalized,
                steam_id,
            },
        );
    }

    pub fn get_steam_id(&self, game_name: &str) -> Option<String> {
        self.mappings
            .get(game_name)
            .and_then(|mapping| mapping.steam_id.clone())
    }
}
