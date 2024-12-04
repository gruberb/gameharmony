use super::{Selectors, WebsiteScraper};
use crate::error::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::Html;

pub struct PCGamerScraper;

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d+)-([a-zA-Z0-9-]+?)(-\d+)?$").unwrap());

fn clean_name(raw_name: &str) -> String {
    raw_name
        .replace("-", " ") // Replace dashes with spaces
        .trim()
        .to_string()
}

impl WebsiteScraper for PCGamerScraper {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, i32)>> {
        let mut games = Vec::new();

        for element in document.select(&selectors.name) {
            if let Some(id) = element.value().attr("id") {
                // Match the ID using regex
                if let Some(caps) = REGEX.captures(id) {
                    let rank_str = caps.get(1).map(|m| m.as_str());
                    let name_str = caps.get(2).map(|m| m.as_str());

                    // Parse rank and clean name
                    if let (Some(rank_str), Some(name_str)) = (rank_str, name_str) {
                        if let Ok(rank) = rank_str.parse::<i32>() {
                            if (1..=100).contains(&rank) {
                                let clean_name = clean_name(name_str);
                                games.push((clean_name, rank));
                            }
                        }
                    }
                }
            }
        }

        Ok(games)
    }
}
