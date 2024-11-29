use super::{Selectors, WebsiteScraper};
use crate::error::Result;
use scraper::Html;

pub struct PolygonPS5Top25;

impl WebsiteScraper for PolygonPS5Top25 {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, i32)>> {
        let mut games = Vec::new();

        let names: Vec<String> = document
            .select(&selectors.name)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .collect();

        for (i, name) in names.into_iter().enumerate() {
            games.push((name, (i + 1) as i32));
        }

        Ok(games)
    }
}
