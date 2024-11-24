use regex::Regex;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

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

    // Insert spaces around numbers to ensure they're separate tokens
    let numbers_re = Regex::new(r"(\d+)").unwrap();
    title = numbers_re.replace_all(&title, " $1 ").to_string();

    // Map of Roman numerals to numbers (case-insensitive)
    let roman_numerals = vec![
        ("(?i)\\bX\\b", "10"),
        ("(?i)\\bIX\\b", "9"),
        ("(?i)\\bVIII\\b", "8"),
        ("(?i)\\bVII\\b", "7"),
        ("(?i)\\bVI\\b", "6"),
        ("(?i)\\bV\\b", "5"),
        ("(?i)\\bIV\\b", "4"),
        ("(?i)\\bIII\\b", "3"),
        ("(?i)\\bII\\b", "2"),
        ("(?i)\\bI\\b", "1"),
    ];

    // Apply Roman numeral replacements using regex
    for (roman_pattern, num) in roman_numerals {
        let re = Regex::new(roman_pattern).unwrap();
        title = re.replace_all(&title, num).to_string();
    }

    // Replace word numbers with digits
    let word_numbers = vec![
        (" zero ", " 0 "),
        (" one ", " 1 "),
        (" two ", " 2 "),
        (" three ", " 3 "),
        (" four ", " 4 "),
        (" five ", " 5 "),
        (" six ", " 6 "),
        (" seven ", " 7 "),
        (" eight ", " 8 "),
        (" nine ", " 9 "),
        (" ten ", " 10 "),
    ];
    for (word, num) in word_numbers {
        title = title.replace(word, num);
    }

    // Remove punctuation
    let re = Regex::new(r"[^\w\s]").unwrap();
    title = re.replace_all(&title, "").to_string();

    // Remove extra whitespace
    title = title.split_whitespace().collect::<Vec<&str>>().join(" ");

    // Remove stop words
    let stop_words = vec![
        "the", "and", "of", "edition", "remastered", "definitive", "part", "collection",
        "remake", "reincarnation", "rebirth", "ultra", "deluxe",
    ];
    let mut words: Vec<&str> = title.split_whitespace().collect();
    words.retain(|word| !stop_words.contains(word));
    title = words.join(" ");

    // Unicode normalization
    title = title.nfkd().collect::<String>();

    title.trim().to_string()
}
