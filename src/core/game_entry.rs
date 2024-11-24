use crate::clients::steam::StorePlatforms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameEntry {
    pub title: String,
    pub rankings: HashMap<String, i32>,
    pub platforms: StorePlatforms,
    pub stores: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_score: Option<f64>,
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
}

impl GameEntry {
    pub fn new(title: String, rankings: HashMap<String, i32>) -> Self {
        Self {
            title,
            rankings,
            platforms: StorePlatforms::default(),
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
        }
    }
}
