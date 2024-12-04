use super::{Selectors, WebsiteScraper};
use crate::error::Result;
use scraper::Html;

pub struct EurogamerScraper;

impl WebsiteScraper for EurogamerScraper {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, u64)>> {
        let mut games = Vec::new();

        // Uses same structure as RockPaperShotgun
        let names: Vec<String> = document
            .select(&selectors.name)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .collect();

        let ranks: Vec<u64> = document
            .select(selectors.rank.as_ref().unwrap())
            .filter_map(|el| {
                let rank_str = el.text().collect::<String>();
                rank_str.trim().parse::<u64>().ok()
            })
            .collect();

        for (i, name) in names.into_iter().enumerate() {
            if let Some(&rank) = ranks.get(i) {
                if (1..=100).contains(&rank) {
                    games.push((name, rank));
                }
            }
        }

        Ok(games)
    }
}
