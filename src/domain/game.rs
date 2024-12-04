use crate::infrastructure::{
    ExtendedPlatforms, RawgGameBasic, RawgGameDetailed, SteamDeckVerifiedResponse, StoreInfo,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub title: String,
    pub rankings: HashMap<String, i32>,
    pub platforms: ExtendedPlatforms,
    pub stores: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_score: Option<i32>,
    #[serde(default)]
    pub total_reviews: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reddit_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metacritic_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protondb_url: Option<String>,
    pub harmony_score: i32,
}

impl Game {
    pub fn new(title: String, rankings: HashMap<String, i32>, harmony_score: i32) -> Self {
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

    pub fn with_rawg_info(mut self, basic: &RawgGameBasic, detailed: &RawgGameDetailed) -> Self {
        if self.header_image.is_none() {
            self.header_image = basic.background_image.clone();
        }

        if !self.platforms.switch {
            self.platforms.switch = basic
                .platforms
                .iter()
                .any(|p| p.platform.name == "Nintendo Switch");
        }

        if let Some(stores) = &basic.stores {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteGames {
    pub source: String,
    pub games: Vec<ScrapedGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedGame {
    pub name: String,
    pub rank: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedGame {
    pub normalized_name: String,
    pub original_names: Vec<String>,
    pub rankings: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWithSteamId {
    pub name: String,
    pub rankings: HashMap<String, i32>,
    pub steam_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedGames {
    pub created_at: u64,
    pub name_index: HashMap<String, IndexedGame>,
    pub letter_index: HashMap<char, Vec<(IndexedGame, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedGame {
    pub appid: u64,
    pub name: String,
}
