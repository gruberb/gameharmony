use super::{Selectors, WebsiteScraper};
use crate::error::Result;
use scraper::Html;

pub struct RPSScraper;

impl WebsiteScraper for RPSScraper {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, i32)>> {
        let mut games = Vec::new();

        let names: Vec<String> = document
            .select(&selectors.name)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .collect();

        let ranks: Vec<i32> = document
            .select(&selectors.rank)
            .filter_map(|el| {
                let rank_str = el.text().collect::<String>();
                rank_str.trim().parse::<i32>().ok()
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
