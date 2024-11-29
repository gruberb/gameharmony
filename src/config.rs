use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub websites: Vec<Website>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Website {
    pub url: String,
    pub name_selector: String,
    pub rank_selector: String,
    #[serde(default)]
    pub has_ranks: bool,
}
