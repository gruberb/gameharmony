use crate::error::Result;
use scraper::{Html, Selector};

pub(crate) mod eurogamer;
pub mod ign;
pub(crate) mod pcgamer;
pub mod rockpapershotgun;

pub trait WebsiteScraper {
    fn extract_games(&self, document: &Html, selectors: &Selectors) -> Result<Vec<(String, i32)>>;
}

pub struct Selectors {
    pub name: Selector,
    pub rank: Selector,
}

impl Selectors {
    pub fn new(name_selector: &str, rank_selector: &str) -> Result<Self> {
        Ok(Self {
            name: Selector::parse(name_selector)
                .map_err(|e| crate::error::GameError::Selector(e.to_string()))?,
            rank: Selector::parse(rank_selector)
                .map_err(|e| crate::error::GameError::Selector(e.to_string()))?,
        })
    }
}
