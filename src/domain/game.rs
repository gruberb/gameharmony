use crate::infrastructure::{
    ExtendedPlatforms, RawgGameDetailed, SteamDeckVerifiedResponse, StoreInfo,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub title: String,
    pub rankings: HashMap<String, u64>,
    pub platforms: ExtendedPlatforms,
    pub stores: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_score: Option<u64>,
    #[serde(default)]
    pub total_reviews: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reddit_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protondb_url: Option<String>,
    pub harmony_score: u64,
}

impl Game {
    pub fn new(title: String, rankings: HashMap<String, u64>, harmony_score: u64) -> Self {
        Self {
            title,
            rankings,
            platforms: ExtendedPlatforms::default(),
            stores: Vec::new(),
            steam_id: None,
            user_score: None,
            total_reviews: 0,
            price: None,
            header_image: None,
            metacritic: None,
            release_date: None,
            reddit_url: None,
            metacritic_url: None,
            protondb_url: None,
            harmony_score,
        }
    }

    pub fn with_steam_info(mut self, store_info: StoreInfo) -> Self {
        self.price = store_info.price;
        self.platforms = store_info.platforms;
        self.user_score = Some(store_info.user_score);
        self.total_reviews = store_info.total_reviews;
        self.header_image = store_info.header_image;
        self.metacritic = store_info.metacritic_score;
        self.metacritic_url = store_info.metacritic_url;
        self.stores.push("Steam".to_string());
        self
    }

    pub fn with_steam_deck_info(
        mut self,
        deck_status: SteamDeckVerifiedResponse,
        steam_id: String,
    ) -> Self {
        if let Some(results) = deck_status.results {
            if results.resolved_category > 0 {
                self.platforms.steamdeck = "verified".to_string();
                self.protondb_url = Some(format!("https://www.protondb.com/app/{}", steam_id));
            }
        }
        self
    }

    pub fn with_rawg_info(mut self, detailed: &RawgGameDetailed) -> Self {
        if self.header_image.is_none() {
            self.header_image = detailed.background_image.clone();
        }

        if !self.platforms.switch {
            self.platforms.switch = detailed
                .platforms
                .iter()
                .any(|p| p.platform.name == "Nintendo Switch");
        }

        if let Some(stores) = &detailed.stores {
            let mut updated_stores = self.stores.clone();
            for store in stores {
                if !updated_stores.contains(&store.store.name) {
                    updated_stores.push(store.store.name.clone());
                }
            }
            updated_stores.sort();
            self.stores = updated_stores;
        }

        if self.metacritic.is_none() {
            self.metacritic = detailed.metacritic;
        }
        if self.release_date.is_none() {
            self.release_date = detailed.released.clone();
        }
        if self.reddit_url.is_none() {
            self.reddit_url = detailed.reddit_url.clone();
        }
        if self.metacritic_url.is_none() {
            self.metacritic_url = detailed.metacritic_url.clone();
        }

        self
    }
}
