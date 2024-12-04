use crate::domain::storage::{Storage, StorageKeys};
use crate::domain::{Game, Manifest};
use crate::error::Result;
use crate::infrastructure::{RawgGameDetailed, StoreInfo};
use crate::services::matching::{GameWithSteamId, IndexedGames};
use crate::services::merging::MergedGame;
use crate::services::scraping::WebsiteGames;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct FileSystemStore {
    data_dir: PathBuf,
    cache_dir: PathBuf,
}

impl FileSystemStore {
    pub fn new(data_dir: impl Into<PathBuf>, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            cache_dir: cache_dir.into(),
        }
    }

    fn get_path_for_key(&self, key: &str, subdir: Option<&str>, use_data_dir: bool) -> PathBuf {
        let base_dir = if use_data_dir {
            &self.data_dir
        } else {
            &self.cache_dir
        };

        if let Some(dir) = subdir {
            base_dir.join(dir).join(format!("{}.json", key))
        } else {
            base_dir.join(format!("{}.json", key))
        }
    }

    fn ensure_dir(&self, dir: &PathBuf) -> Result<()> {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    fn write_json_file<T: serde::Serialize + ?Sized>(
        &self,
        key: &str,
        subdir: Option<&str>,
        data: &T,
        use_data_dir: bool,
    ) -> Result<()> {
        let base_dir = if use_data_dir {
            &self.data_dir
        } else {
            &self.cache_dir
        };

        if let Some(dir) = subdir {
            self.ensure_dir(&base_dir.join(dir))?;
        }

        let path = self.get_path_for_key(key, subdir, use_data_dir);
        let content = serde_json::to_string_pretty(data)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn read_json_file<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        subdir: Option<&str>,
        use_data_dir: bool,
    ) -> Result<Option<T>> {
        let path = self.get_path_for_key(key, subdir, use_data_dir);
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(Some(serde_json::from_str(&content)?))
        } else {
            Ok(None)
        }
    }
}

impl Storage for FileSystemStore {
    fn save_indexed_games(&self, indexed_games: &IndexedGames) -> Result<()> {
        self.write_json_file(
            StorageKeys::STEAM_APPS_INDEX,
            Some(StorageKeys::STEAM_APPS_DIR),
            indexed_games,
            false,
        )
    }

    fn load_indexed_games(&self) -> Result<Option<IndexedGames>> {
        self.read_json_file(
            StorageKeys::STEAM_APPS_INDEX,
            Some(StorageKeys::STEAM_APPS_DIR),
            false,
        )
    }

    fn save_website_games(&self, website_games: &[WebsiteGames]) -> Result<()> {
        for game in website_games {
            let filename = game.source.replace('/', "_");
            self.write_json_file(&filename, Some(StorageKeys::SOURCES_DIR), game, false)?;
        }
        Ok(())
    }

    fn load_website_games(&self, url: String) -> Result<Option<WebsiteGames>> {
        let filename = url.replace('/', "_");
        self.read_json_file(&filename, Some(StorageKeys::SOURCES_DIR), false)
    }

    fn load_merged_games(&self) -> Result<Option<Vec<MergedGame>>> {
        self.read_json_file(
            StorageKeys::MERGED_GAMES,
            Some(StorageKeys::ENHANCEMENTS_DIR),
            false,
        )
    }

    fn save_merged_games(&self, games: &[MergedGame]) -> Result<()> {
        self.write_json_file(
            StorageKeys::MERGED_GAMES,
            Some(StorageKeys::ENHANCEMENTS_DIR),
            games,
            false,
        )
    }

    fn load_matched_games(&self) -> Result<Option<Vec<GameWithSteamId>>> {
        self.read_json_file(
            StorageKeys::MERGED_GAMES_WITH_STEAM_ID,
            Some(StorageKeys::ENHANCEMENTS_DIR),
            false,
        )
    }

    fn save_matched_games(&self, games: &[GameWithSteamId]) -> Result<()> {
        self.write_json_file(
            StorageKeys::MERGED_GAMES_WITH_STEAM_ID,
            Some(StorageKeys::ENHANCEMENTS_DIR),
            games,
            false,
        )
    }

    fn load_app_info(&self, app_id: u64) -> Result<Option<StoreInfo>> {
        self.read_json_file(
            &app_id.to_string(),
            Some(StorageKeys::STEAM_APPS_DIR),
            false,
        )
    }

    fn save_app_info(&self, app_id: u64, store_info: StoreInfo) -> Result<()> {
        self.write_json_file(
            &app_id.to_string(),
            Some(StorageKeys::STEAM_APPS_DIR),
            &store_info,
            false,
        )
    }

    fn load_rawg_info(&self, name: &str) -> Result<Option<RawgGameDetailed>> {
        self.read_json_file(name, Some(StorageKeys::RAWG_APPS_DIR), false)
    }

    fn save_rawg_info(&self, name: &str, rawg_info: RawgGameDetailed) -> Result<()> {
        self.write_json_file(name, Some(StorageKeys::RAWG_APPS_DIR), &rawg_info, false)
    }

    fn load_enriched_games(&self) -> Result<Option<Vec<Game>>> {
        self.read_json_file(
            StorageKeys::ENRICHED_GAMES,
            Some(StorageKeys::ENHANCEMENTS_DIR),
            false,
        )
    }

    fn save_enriched_games(&self, games: &[Game]) -> Result<()> {
        self.write_json_file(
            StorageKeys::ENRICHED_GAMES,
            Some(StorageKeys::ENHANCEMENTS_DIR),
            games,
            false,
        )
    }

    fn save_manifest(&self, manifest: &Manifest) -> Result<()> {
        self.write_json_file(
            StorageKeys::MANIFEST,
            None,
            manifest,
            true, // Use data_dir
        )
    }
}
