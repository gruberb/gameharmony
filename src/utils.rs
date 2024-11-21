use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use strsim::normalized_levenshtein;

static NORMALIZATION_MAP: OnceCell<HashMap<&'static str, &'static str>> = OnceCell::new();

pub fn get_normalization_map() -> &'static HashMap<&'static str, &'static str> {
    NORMALIZATION_MAP.get_or_init(|| {
        HashMap::from([
            ("grand theft auto 5", "Grand Theft Auto V"),
            ("crusader kings 3", "Crusader Kings III"),
            ("crusader king 3", "Crusader Kings III"),
            ("titanfall 2", "Titanfall 2"),
            ("titanfall® 2", "Titanfall 2"),
            (
                "mass effect: legendary edition",
                "Mass Effect Legendary Edition",
            ),
            (
                "mass effect legendary edition",
                "Mass Effect Legendary Edition",
            ),
            ("resident evil 4 remake", "Resident Evil 4 (Remake)"),
            ("resident evil iv (remake)", "Resident Evil 4 (Remake)"),
            ("resident evil iv", "Resident Evil 4 (Remake)"),
            ("total war: warhammer 3", "Total War: Warhammer III"),
            ("god of war 2018", "God of War"),
            ("god of war (2018)", "God of War"),
            ("hunt: showdown", "Hunt: Showdown 1896"),
            ("doom (1993)", "Doom"),
            ("Dark Souls", "DARK SOULS: REMASTERED"),
            ("final fantasy 14", "FINAL FANTASY XIV"),
        ])
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamIdMapping {
    pub name: String,
    pub steam_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamIdConfig {
    pub mappings: Vec<SteamIdMapping>,
}

impl SteamIdConfig {
    pub fn load() -> Self {
        let config_path = Path::new("steam_ids.json");

        // Load existing config
        let config_str =
            std::fs::read_to_string(config_path).expect("Failed to read steam_ids.json");
        serde_json::from_str(&config_str).expect("Failed to parse steam_ids.json")
    }

    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.mappings
            .iter()
            .map(|mapping| (mapping.name.clone(), mapping.steam_id.clone()))
            .collect()
    }
}

pub fn get_steam_id_map() -> HashMap<String, String> {
    SteamIdConfig::load().to_hashmap()
}

pub fn normalize_title(title: &str) -> String {
    let normalization_map = get_normalization_map();

    let normalized = title
        .to_lowercase()
        .replace(['®', '™'], "")
        .trim()
        .to_string();

    let normalized = normalization_map
        .get(normalized.as_str())
        .map(|&mapped| mapped.to_string())
        .unwrap_or_else(|| normalized);

    static YEAR_PATTERN: OnceCell<regex::Regex> = OnceCell::new();
    let year_re = YEAR_PATTERN.get_or_init(|| regex::Regex::new(r"\s*\((\d{4})\)").unwrap());

    let final_normalized = year_re.replace_all(&normalized, "").to_string();
    final_normalized.trim_end_matches([':', '-']).to_string()
}

pub fn are_titles_same_game(title1: &str, title2: &str) -> bool {
    let normalized1 = normalize_title(title1);
    let normalized2 = normalize_title(title2);

    // First check the normalization map for exact matches
    let normalization_map = get_normalization_map();
    let mapped1 = normalization_map.get(normalized1.as_str());
    let mapped2 = normalization_map.get(normalized2.as_str());

    // If either title has a mapping, use those for comparison
    match (mapped1, mapped2) {
        (Some(m1), Some(m2)) => return m1 == m2,
        (Some(m1), None) => return *m1 == normalized2,
        (None, Some(m2)) => return normalized1 == *m2,
        _ => {}
    }

    // Perfect match after normalization
    if normalized1 == normalized2 {
        return true;
    }

    // Check if either game is in the steam ID map
    let steam_id_map = get_steam_id_map();
    let id1 = steam_id_map.get(normalized1.as_str());
    let id2 = steam_id_map.get(normalized2.as_str());

    // If both games map to the same Steam ID, they're the same game
    if let (Some(id1), Some(id2)) = (id1, id2) {
        return id1 == id2;
    }

    // Check for numbered sequels
    let get_number = |s: &str| -> Option<(String, String)> {
        let mut base = String::new();
        let mut number = String::new();
        let mut found_number = false;

        for c in s.chars() {
            if c.is_numeric() {
                found_number = true;
                number.push(c);
            } else if found_number {
                break;
            } else {
                base.push(c);
            }
        }

        if found_number {
            Some((base.trim().to_string(), number))
        } else {
            None
        }
    };

    if let (Some((base1, num1)), Some((base2, num2))) =
        (get_number(&normalized1), get_number(&normalized2))
    {
        if normalized_levenshtein(&base1, &base2) > 0.9 && num1 != num2 {
            return false;
        }
    }

    normalized_levenshtein(&normalized1, &normalized2) > 0.95
}
