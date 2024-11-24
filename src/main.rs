use crate::core::GameHarmony;
use crate::error::Result;
use tracing::info;

mod clients;
mod config;
mod core;
mod error;
mod matcher;
mod scrapers;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let harmony = GameHarmony::new("scraper_config.json").await?;
    harmony.run().await?;

    info!("Scraping completed successfully!");
    Ok(())
}
