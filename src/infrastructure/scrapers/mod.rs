use crate::error::Result;
use scraper::{Html, Selector};

pub(crate) mod eurogamer;
pub(crate) mod ign;
pub(crate) mod pcgamer;
pub(crate) mod polygon_ps5_top25;
pub(crate) mod rockpapershotgun;

pub trait WebsiteScraper {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, i32)>>;
}

pub struct Selectors {
    pub name: Selector,
    pub rank: Option<Selector>,
}

impl Selectors {
    pub fn new(name_selector: &str, rank_selector: &str) -> Result<Self> {
        // The name selector is always required
        let name = Selector::parse(name_selector)
            .map_err(|e| crate::error::GameError::Selector(e.to_string()))?;

        // The rank selector is optional - only parse it if it's not empty
        let rank = if !rank_selector.is_empty() {
            Some(
                Selector::parse(rank_selector)
                    .map_err(|e| crate::error::GameError::Selector(e.to_string()))?,
            )
        } else {
            None
        };

        Ok(Self { name, rank })
    }
}
