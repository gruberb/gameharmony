use crate::config::Website;
use crate::error::Result;
use crate::infrastructure::EurogamerScraper;
use crate::infrastructure::IGNScraper;
use crate::infrastructure::PCGamerScraper;
use crate::infrastructure::PolygonPS5Top25;
use crate::infrastructure::PolygonScraper;
use crate::infrastructure::RPSScraper;
use crate::infrastructure::{Selectors, WebsiteScraper};
use reqwest::Client;
use scraper::Html;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteGames {
    pub source: String,
    pub games: Vec<ScrapedGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedGame {
    pub name: String,
    pub rank: u64,
}

pub struct ScrapingService {
    client: Client,
}

impl ScrapingService {
    pub fn new(client: Client) -> Self {
        info!("Created new Scraping service");
        Self { client }
    }

    fn get_scraper(&self, website: &Website) -> Box<dyn WebsiteScraper> {
        match website.scraper_type.as_str() {
            "ign" => Box::new(IGNScraper),
            "polygon_top_ps5" => Box::new(PolygonPS5Top25),
            "polygon" => Box::new(PolygonScraper),
            "eurogamer" => Box::new(EurogamerScraper),
            "rps" => Box::new(RPSScraper),
            "pcgamer" => Box::new(PCGamerScraper),
            _ => panic!("Unknown scraper type")
        }
    }

    pub async fn scrape_all(&self, websites: &[Website]) -> Result<Vec<WebsiteGames>> {
        let mut games = Vec::new();

        for website in websites {
            let website_games = self.scrape_website(website).await?;
            games.push(website_games);
            sleep(Duration::from_secs(1)).await;
        }

        Ok(games)
    }

    async fn scrape_website(&self, website: &Website) -> Result<WebsiteGames> {
        let response = self.client.get(&website.url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let selectors = Selectors::new(&website.name_selector, &website.rank_selector)?;

        let scraper = self.get_scraper(website);
        let games = scraper.extract_games(&document, &selectors)?;

        Ok(WebsiteGames {
            source: website.url.clone(),
            games: games
                .into_iter()
                .map(|(name, rank)| ScrapedGame { name, rank })
                .collect(),
        })
    }
}
