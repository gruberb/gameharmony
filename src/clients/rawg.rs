use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
pub struct RawgSearchResponse {
    pub results: Vec<RawgGameBasic>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawgGameBasic {
    pub id: i32,
    pub name: String,
    pub background_image: Option<String>,
    pub platforms: Vec<RawgPlatform>,
    pub stores: Option<Vec<RawgStore>>,
    pub rating: Option<f64>,
    pub metacritic: Option<i32>,
    pub released: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawgGameDetailed {
    pub id: i32,
    pub name: String,
    pub metacritic: Option<i32>,
    pub released: Option<String>,
    pub background_image: Option<String>,
    pub reddit_url: Option<String>,
    pub metacritic_url: Option<String>,
    pub platforms: Vec<RawgPlatform>,
    pub stores: Option<Vec<RawgStore>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawgPlatform {
    pub platform: PlatformInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlatformInfo {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawgStore {
    pub store: StoreInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreInfo {
    pub name: String,
}

pub struct RawgClient {
    client: Client,
    api_key: String,
}

impl RawgClient {
    pub fn new(client: Client, api_key: String) -> Self {
        Self { client, api_key }
    }

    pub async fn get_game_info(
        &self,
        title: &str,
    ) -> Result<Option<(RawgGameBasic, RawgGameDetailed)>, Box<dyn Error>> {
        let search_url = "https://api.rawg.io/api/games";
        let response = self
            .client
            .get(search_url)
            .query(&[
                ("key", &self.api_key),
                ("search", &title.to_string()),
                ("page_size", &"1".to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            error!("RAWG API error: Status {}", response.status());
            return Ok(None);
        }

        let search_data: RawgSearchResponse = response.json().await?;
        if search_data.results.is_empty() {
            info!("No RAWG data found for: {}", title);
            return Ok(None);
        }

        let basic_info = search_data.results.into_iter().next().unwrap();
        info!("Basic RAWG data found for {title}: {}", basic_info.name);

        // Get detailed information
        let detail_url = format!("https://api.rawg.io/api/games/{}", basic_info.id);
        let detailed_response = self
            .client
            .get(&detail_url)
            .query(&[("key", &self.api_key)])
            .send()
            .await?;

        if !detailed_response.status().is_success() {
            error!(
                "RAWG API detail error: Status {}",
                detailed_response.status()
            );
            return Ok(None);
        }

        let detailed_info: RawgGameDetailed = detailed_response.json().await?;

        sleep(Duration::from_secs(1)).await; // Rate limiting

        Ok(Some((basic_info, detailed_info)))
    }
}