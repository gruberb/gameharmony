use crate::error::Result;
use crate::utils::{get_steam_id_map, normalize_title};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use once_cell::sync::OnceCell;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SteamClient {
    client: Client,
    steam_apps: Vec<SteamApp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamResponse {
    pub applist: SteamAppList,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamAppList {
    pub apps: Vec<SteamApp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamApp {
    pub appid: u64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamStoreData {
    pub success: bool,
    pub data: SteamStoreDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamStoreDetails {
    pub price_overview: Option<PriceOverview>,
    pub platforms: StorePlatforms,
    pub header_image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceOverview {
    pub final_formatted: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorePlatforms {
    pub windows: bool,
    pub macos: bool,
    pub linux: bool,
    pub steamdeck: String,
    pub switch: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamReviewsResponse {
    pub query_summary: ReviewsSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewsSummary {
    pub total_positive: i64,
    pub total_reviews: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamStoreInfo {
    pub price: Option<String>,
    pub platforms: StorePlatforms,
    pub header_image: Option<String>,
    pub user_score: Option<f64>,
    pub total_reviews: i32,
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

    pub fn find_steam_id(&self, game_name: &str) -> Option<String> {
        static MATCHER: OnceCell<SkimMatcherV2> = OnceCell::new();
        let matcher = MATCHER.get_or_init(SkimMatcherV2::default);

        let normalized_title = normalize_title(game_name);
        let variations = [
            normalized_title.clone(),
            normalized_title.replace('2', "II"),
            normalized_title.replace("II", "2"),
            normalized_title.replace("III", "3"),
            normalized_title.replace('3', "III"),
            normalized_title.replace('4', "IV"),
            normalized_title.replace("IV", "4"),
        ];

        let steam_id_map = get_steam_id_map();

        // Check direct mapping first
        if let Some(steam_id) = steam_id_map.get(game_name) {
            return Some(steam_id.to_string());
        }

        use rayon::prelude::*;

        self.steam_apps
            .par_iter()
            .map(|app| {
                let max_score = variations
                    .iter()
                    .map(|variation| {
                        matcher
                            .fuzzy_match(&app.name.to_lowercase(), &variation.to_lowercase())
                            .unwrap_or(0)
                    })
                    .max()
                    .unwrap_or(0);
                (app, max_score)
            })
            .filter(|(_, score)| *score > 90)
            .max_by_key(|(_, score)| *score)
            .map(|(app, _)| {
                tracing::info!(
                    "Steam match found for '{}' with appid {}",
                    game_name,
                    app.appid
                );
                app.appid.to_string()
            })
    }

    pub async fn get_store_info(&self, app_id: u64) -> Result<Option<SteamStoreInfo>> {
        // Cache handling
        let cache_path = std::path::Path::new("cache").join(format!("steam_store_{}.json", app_id));
        if cache_path.exists() {
            if let Ok(cached) = std::fs::read_to_string(&cache_path) {
                if let Ok(info) = serde_json::from_str(&cached) {
                    return Ok(Some(info));
                }
            }
        }

        // Fetch store data
        let store_url = format!(
            "https://store.steampowered.com/api/appdetails?appids={}",
            app_id
        );
        let store_response = self.client.get(&store_url).send().await?;

        if !store_response.status().is_success() {
            return Ok(None);
        }

        let store_data: HashMap<String, SteamStoreData> = store_response.json().await?;
        let store_details = match store_data.get(&app_id.to_string()) {
            Some(data) if data.success => &data.data,
            _ => return Ok(None),
        };

        // Fetch reviews data
        let reviews_url = format!(
            "https://store.steampowered.com/appreviews/{}?json=1",
            app_id
        );
        let reviews_response = self.client.get(&reviews_url).send().await?;

        let reviews_data: SteamReviewsResponse = if reviews_response.status().is_success() {
            reviews_response.json().await?
        } else {
            return Ok(None);
        };

        let info = SteamStoreInfo {
            price: store_details
                .price_overview
                .as_ref()
                .map(|p| p.final_formatted.clone()),
            platforms: StorePlatforms {
                windows: store_details.platforms.windows,
                macos: store_details.platforms.macos,
                linux: store_details.platforms.linux,
                steamdeck: String::new(), // This will be filled by ProtonDB
                switch: false,            // This will be filled by RAWG
            },
            header_image: store_details.header_image.clone(),
            user_score: if reviews_data.query_summary.total_reviews > 0 {
                Some(
                    reviews_data.query_summary.total_positive as f64
                        / reviews_data.query_summary.total_reviews as f64,
                )
            } else {
                None
            },
            total_reviews: reviews_data.query_summary.total_reviews as i32,
        };

        // Cache the results
        if let Ok(cache_data) = serde_json::to_string_pretty(&info) {
            let _ = std::fs::write(&cache_path, cache_data);
        }

        Ok(Some(info))
    }
}
