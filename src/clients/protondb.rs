use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonDBReport {
    #[serde(rename = "bestReportedTier", default)]
    pub best_reported_tier: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub total: i32,
    #[serde(rename = "trendingTier", default)]
    pub trending_tier: String,
}

pub struct ProtonDBClient {
    client: Client,
}

impl ProtonDBClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn get_protondb_data(
        &self,
        steam_id: &str,
    ) -> Result<ProtonDBReport, Box<dyn Error>> {
        let url = format!(
            "https://www.protondb.com/api/v1/reports/summaries/{}.json",
            steam_id
        );

        match self.client.get(&url).send().await?.json().await {
            Ok(report) => {
                info!("Found ProtonDB info for {steam_id}");
                Ok(report)
            }
            Err(_) => Ok(ProtonDBReport {
                best_reported_tier: String::new(),
                confidence: String::new(),
                score: 0.0,
                tier: String::new(),
                total: 0,
                trending_tier: String::new(),
            }),
        }
    }
}
