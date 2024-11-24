use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Game {
    name: String,
    rank: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct Source {
    source: String,
    games: Vec<Game>,
}

pub fn normalize_source(source: &str) -> String {
    if source.contains("rockpapershotgun") {
        "RPS".to_string()
    } else if source.contains("pcgamer") {
        "PCGamer".to_string()
    } else if source.contains("eurogamer") {
        "Eurogamer".to_string()
    } else if source.contains("ign") {
        "IGN".to_string()
    } else {
        source.to_string()
    }
}

/// Normalizes a game title by converting it to lowercase, removing apostrophes,
/// replacing hyphens with spaces, removing punctuation, and collapsing multiple spaces.
pub fn normalize_title(title: &str) -> String {
    let mut title = title.to_lowercase();

    // Remove apostrophes
    title = title.replace("'", "");

    // Replace hyphens with spaces
    title = title.replace("-", " ");

    // Remove all non-alphanumeric characters except spaces
    let re = Regex::new(r"[^a-z0-9\s]").unwrap();
    title = re.replace_all(&title, "").to_string();

    // Collapse multiple spaces into a single space
    let re_spaces = Regex::new(r"\s+").unwrap();
    title = re_spaces.replace_all(&title, " ").trim().to_string();

    title
}