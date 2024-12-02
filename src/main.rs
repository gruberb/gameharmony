use crate::error::Result;
use crate::processor::Processor;
use tracing::info;

mod clients;
mod error;
mod matcher;
mod processor;
mod scrapers;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let harmony = Processor::new("scraper_config.json").await?;
    harmony.run().await?;

    info!("Scraping completed successfully!");
    Ok(())
}
