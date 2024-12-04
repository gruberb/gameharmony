use crate::config::Website;
use crate::domain::{ScrapedGame, WebsiteGames};
use crate::error::Result;
use crate::infrastructure::EurogamerScraper;
use crate::infrastructure::IGNScraper;
use crate::infrastructure::PCGamerScraper;
use crate::infrastructure::PolygonPS5Top25;
use crate::infrastructure::RPSScraper;
use crate::infrastructure::{Selectors, WebsiteScraper};
use reqwest::Client;
use scraper::Html;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

pub struct ScrapingService {
    client: Client,
}

impl ScrapingService {
    pub fn new(client: Client) -> Self {
        info!("Created new Scraping service");
        Self { client }
    }

    fn get_scraper(&self, url: &str) -> Box<dyn WebsiteScraper> {
        match url {
            url if url.contains("ign.com") => Box::new(IGNScraper),
            url if url.contains("rockpapershotgun.com") => Box::new(RPSScraper),
            url if url.contains("eurogamer.net") => Box::new(EurogamerScraper),
            url if url.contains("pcgamer.com") => Box::new(PCGamerScraper),
            url if url.contains("best-ps5-games-playstation-5") => Box::new(PolygonPS5Top25),
            _ => Box::new(RPSScraper), // Default scraper
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

        let scraper = self.get_scraper(&website.url);
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
