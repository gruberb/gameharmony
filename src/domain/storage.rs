use super::{Game, GameWithSteamId, IndexedGames, Manifest, MergedGame, WebsiteGames};
use crate::error::Result;
use crate::infrastructure::StoreInfo;

pub trait Storage: Send + Sync {
    fn load_indexed_games(&self) -> Result<Option<IndexedGames>>;
    fn save_indexed_games(&self, index: &IndexedGames) -> Result<()>;
    fn load_website_games(&self, url: String) -> Result<Option<WebsiteGames>>;
    fn save_website_games(&self, games: &[WebsiteGames]) -> Result<()>;
    fn load_merged_games(&self) -> Result<Option<Vec<MergedGame>>>;
    fn save_merged_games(&self, games: &[MergedGame]) -> Result<()>;
    fn load_matched_games(&self) -> Result<Option<Vec<GameWithSteamId>>>;
    fn save_matched_games(&self, games: &[GameWithSteamId]) -> Result<()>;
    fn load_app_info(&self, app_id: u64) -> Result<Option<StoreInfo>>;
    fn save_app_info(&self, app_id: u64, store_info: StoreInfo) -> Result<()>;
    fn load_enriched_games(&self) -> Result<Option<Vec<Game>>>;
    fn save_enriched_games(&self, games: &[Game]) -> Result<()>;
    fn save_manifest(&self, manifest: &Manifest) -> Result<()>;
}

pub struct StorageKeys;

impl StorageKeys {
    // Base directories
    pub const SOURCES_DIR: &'static str = "sources";
    pub const STEAM_APPS_DIR: &'static str = "steam_apps";
    pub const ENHANCEMENTS_DIR: &'static str = "enhancements";

    pub const STEAM_APPS_INDEX: &'static str = "index_apps";
    pub const MERGED_GAMES: &'static str = "merged_games";
    pub const MERGED_GAMES_WITH_STEAM_ID: &'static str = "merged_with_steam_id";

    pub const ENRICHED_GAMES: &'static str = "enriched_games";
    pub const MANIFEST: &'static str = "manifest";
}
