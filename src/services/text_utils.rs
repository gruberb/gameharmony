use regex::Regex;
use unicode_normalization::UnicodeNormalization;
use crate::config::ScraperConfig;

/// Utilities for processing and normalizing game titles
pub struct TitleNormalizer;

impl TitleNormalizer {
    /// Formats a title for display by properly capitalizing words
    pub fn format_for_display(title: &str) -> String {
        // List of words that should not be capitalized
        let lowercase_words = ["the", "of", "and", "in", "on", "at", "to", "for", "with"];

        title
            .split_whitespace()
            .enumerate()
            .map(|(i, word)| {
                if i == 0 || !lowercase_words.contains(&word.to_lowercase().as_str()) {
                    // Capitalize first letter, keep rest of the case
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(chars).collect(),
                    }
                } else {
                    word.to_lowercase()
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub fn normalize_source(source: &str, scraper_config: &ScraperConfig) -> String {
        scraper_config.websites.iter()
            .find(|w| source.contains(&w.pattern))
            .map(|w| w.display_name.clone())
            .unwrap_or_else(|| source.to_string())
    }

    /// Normalizes a game title by converting it to lowercase, removing apostrophes,
    /// replacing hyphens with spaces, removing punctuation, and collapsing multiple spaces.
    pub fn normalize(title: &str) -> String {
        // Remove year suffixes in parentheses
        let year_suffix_re = Regex::new(r"\s*(?:\(?\d{4}\)?)\b").unwrap();
        let title = year_suffix_re
            .replace_all(&title.to_lowercase(), "")
            .to_string();

        // Convert to lowercase
        let mut title = title.to_lowercase();

        let specific_game_replacements = [
            // Use word boundaries to ensure we match complete words
            (r"\bhalf[\s-]life\b", "halflife"),
            (r"\bcounter[\s-]strike\b", "counterstrike"),
            // Add other specific cases as needed
        ];

        // Remove game status suffixes
        let status_patterns = [
            (r"\s*\(early access\)", ""),
            (r"\s*early access", ""),
            (r"\s*\(full release\)", ""),
            // Add other status patterns as needed
        ];

        for (pattern, replacement) in status_patterns {
            let re = Regex::new(pattern).unwrap();
            title = re.replace_all(&title, replacement).to_string();
        }

        // Remove apostrophes and normalize possessives first
        let possessive_re = Regex::new(r"(?:'\s*s|\s+s)\b").unwrap();
        title = possessive_re.replace_all(&title, "s").to_string();

        // Remove all punctuation except hyphens initially
        let punctuation_re = Regex::new(r"[^\w\s-]").unwrap();
        title = punctuation_re.replace_all(&title, "").to_string();

        // Insert spaces around numbers
        let numbers_re = Regex::new(r"(\d+)").unwrap();
        title = numbers_re.replace_all(&title, " $1 ").to_string();

        for (pattern, replacement) in specific_game_replacements {
            let re = Regex::new(pattern).unwrap();
            title = re.replace_all(&title, replacement).to_string();
        }

        // Convert Roman numerals to Arabic numbers
        let roman_numerals = vec![
            ("(?i)\\bXXV\\b", "25"),
            ("(?i)\\bXXIV\\b", "24"),
            ("(?i)\\bXXIII\\b", "23"),
            ("(?i)\\bXXII\\b", "22"),
            ("(?i)\\bXXI\\b", "21"),
            ("(?i)\\bXX\\b", "20"),
            ("(?i)\\bXIX\\b", "19"),
            ("(?i)\\bXVIII\\b", "18"),
            ("(?i)\\bXVII\\b", "17"),
            ("(?i)\\bXVI\\b", "16"),
            ("(?i)\\bXV\\b", "15"),
            ("(?i)\\bXIV\\b", "14"),
            ("(?i)\\bXIII\\b", "13"),
            ("(?i)\\bXII\\b", "12"),
            ("(?i)\\bXI\\b", "11"),
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

        // Apply Roman numeral replacements
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

        // Remove all remaining punctuation
        let punctuation_re = Regex::new(r"[^\w\s]").unwrap();
        title = punctuation_re.replace_all(&title, "").to_string();

        // Remove stop words
        let stop_words = vec![
            "the",
            "and",
            "of",
            "edition",
            "remastered",
            "definitive",
            "part",
            "collection",
            "remake",
            "reincarnation",
            "rebirth",
            "ultra",
            "deluxe",
            "ultimate",
            "complete",
            "enhanced",
            "goty",
            "expanded",
            "final",
            "cut",
            "directors",
        ];

        let words: Vec<String> = title
            .split_whitespace()
            .filter(|word| !stop_words.contains(&word.to_lowercase().as_str()))
            .map(|s| s.to_string())
            .collect();

        let title = words.join(" ");

        // Unicode normalization for consistency
        title.nfkd().collect::<String>().trim().to_string()
    }
}
