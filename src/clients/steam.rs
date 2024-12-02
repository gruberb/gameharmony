use crate::error::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug)]
pub struct SteamClient {
    client: Client,
    pub steam_apps: Vec<SteamApp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamApp {
    pub appid: u64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamResponse {
    pub applist: SteamAppList,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamAppList {
    pub apps: Vec<SteamApp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamStoreData {
    pub success: bool,
    pub data: SteamStoreDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamStoreDetails {
    pub price_overview: Option<PriceOverview>,
    pub platforms: Platforms,
    pub header_image: Option<String>,
    pub metacritic: Option<Metacritic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceOverview {
    pub final_formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metacritic {
    pub score: i32,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platforms {
    pub windows: bool,
    pub mac: bool,
    pub linux: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamReviewsResponse {
    pub query_summary: ReviewsSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewsSummary {
    pub total_positive: i64,
    pub total_reviews: i64,
    pub review_score: i32,
    pub review_score_desc: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamDeckVerifiedResponse {
    pub success: i32,
    pub results: Option<DeckResults>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeckResults {
    pub appid: u64,
    pub resolved_category: i32,
    pub resolved_items: Vec<DeckResultItem>,
    pub steam_deck_blog_url: String,
    pub search_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeckResultItem {
    pub display_type: i32,
    pub loc_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreInfo {
    pub price: Option<String>,
    pub platforms: ExtendedPlatforms,
    pub header_image: Option<String>,
    pub user_score: i32,
    pub total_reviews: i32,
    pub metacritic_score: Option<i32>,
    pub metacritic_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExtendedPlatforms {
    pub windows: bool,
    pub macos: bool,
    pub linux: bool,
    pub steamdeck: String,
    pub switch: bool,
}

impl From<Platforms> for ExtendedPlatforms {
    fn from(p: Platforms) -> Self {
        Self {
            windows: p.windows,
            macos: p.mac,
            linux: p.linux,
            steamdeck: String::new(),
            switch: false,
        }
    }
}

impl SteamClient {
    pub async fn new(client: Client) -> Result<Self> {
        let steam_apps = Self::fetch_steam_apps(&client).await?;
        Ok(Self { client, steam_apps })
    }

    async fn fetch_steam_apps(client: &Client) -> Result<Vec<SteamApp>> {
        let url = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";
        let response: SteamResponse = client.get(url).send().await?.json().await?;
        Ok(response.applist.apps)
    }

    pub async fn get_store_info(&self, app_id: u64) -> Result<Option<StoreInfo>> {
        info!("Get store info for: {app_id}");

        let cache_dir = std::path::Path::new("cache");
        if !cache_dir.exists() {
            std::fs::create_dir_all(cache_dir)?;
        }

        let cache_path = cache_dir.join(format!("steam_store_{}.json", app_id));
        if cache_path.exists() {
            if let Ok(cached) = std::fs::read_to_string(&cache_path) {
                if let Ok(info) = serde_json::from_str(&cached) {
                    info!("Using cached data");
                    return Ok(Some(info));
                }
            }
        }

        let store_data = self.fetch_store_data(app_id).await?;
        let reviews = self.fetch_reviews(app_id).await?;

        let info = match (store_data, reviews) {
            (Some(store), Some(reviews)) => Some(StoreInfo {
                price: store.price_overview.map(|p| p.final_formatted),
                platforms: store.platforms.into(),
                header_image: store.header_image,
                metacritic_score: store.metacritic.clone().map(|m| m.score),
                metacritic_url: store.metacritic.map(|m| m.url),
                user_score: reviews.query_summary.review_score,
                total_reviews: reviews.query_summary.total_reviews as i32,
            }),
            _ => None,
        };

        if let Some(info) = &info {
            if let Ok(cache_data) = serde_json::to_string_pretty(info) {
                if let Err(e) = std::fs::write(&cache_path, cache_data) {
                    info!("Failed to write cache: {}", e);
                }
            }
        }

        Ok(info)
    }

    async fn fetch_store_data(&self, app_id: u64) -> Result<Option<SteamStoreDetails>> {
        let url = format!(
            "https://store.steampowered.com/api/appdetails?appids={}",
            app_id
        );

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Ok(None);
        }

        let data: HashMap<String, SteamStoreData> = response.json().await?;
        Ok(data
            .get(&app_id.to_string())
            .filter(|d| d.success)
            .map(|d| d.data.clone()))
    }

    async fn fetch_reviews(&self, app_id: u64) -> Result<Option<SteamReviewsResponse>> {
        let url = format!(
            "https://store.steampowered.com/appreviews/{}?json=1",
            app_id
        );

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Ok(None);
        }

        Ok(Some(response.json().await?))
    }

    pub async fn get_deck_verified(&self, app_id: String) -> Result<SteamDeckVerifiedResponse> {
        let url = format!(
            "https://store.steampowered.com/saleaction/ajaxgetdeckappcompatibilityreport?nAppID={app_id}"
        );

        let response = self.client.get(&url).send().await?;
        let deck_status: SteamDeckVerifiedResponse = response.json().await?;

        Ok(deck_status)
    }
}
