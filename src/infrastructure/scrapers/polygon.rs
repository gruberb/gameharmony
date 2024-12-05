use super::{Selectors, WebsiteScraper};
use crate::error::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::Html;

pub struct PolygonScraper;

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)\.\s*(.+)").unwrap());

impl WebsiteScraper for PolygonScraper {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, u64)>> {
        let mut games = Vec::new();

        for element in document.select(&selectors.name) {
            let text = element.text().collect::<String>();
            if let Some(caps) = REGEX.captures(&text) {
                if let (Some(rank_str), Some(name)) = (caps.get(1), caps.get(2)) {
                    if let Ok(rank) = rank_str.as_str().parse::<u64>() {
                        if (1..=50).contains(&rank) {
                            games.push((name.as_str().trim().to_string(), rank));
                        }
                    }
                }
            }
        }

        Ok(games)
    }
}