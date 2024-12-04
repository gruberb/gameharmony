use crate::domain::storage::Storage;
use crate::domain::Game;
use crate::error::Result;
use crate::infrastructure::{RawgClient, SteamClient};
use crate::services::matching::GameWithSteamId;
use crate::services::scoring::calculate_harmony_score;
use crate::services::text_utils::TitleNormalizer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct Enrichment {
    steam_client: SteamClient,
    rawg_client: RawgClient,
    store: Arc<dyn Storage>,
}

impl Enrichment {
    pub fn new(
        steam_client: SteamClient,
        rawg_client: RawgClient,
        store: Arc<dyn Storage + 'static>,
    ) -> Self {
        Self {
            steam_client,
            rawg_client,
            store,
        }
    }

    pub(crate) async fn enrich_games(
        &self,
        games_with_ids: Vec<GameWithSteamId>,
    ) -> Result<Vec<Game>> {
        if let Some(cached) = self.store.load_enriched_games()? {
            return Ok(cached);
        }

        let mut enriched_games = Vec::new();
        for game in games_with_ids {
            let harmony_score = calculate_harmony_score(&game.rankings);
            let mut entry = Game::new(game.name, game.rankings, harmony_score);
            entry.steam_id = game.steam_id.as_ref().map(|id| id.parse().unwrap());

            if let Some(steam_id) = &game.steam_id {
                if let Ok(Some(store_info)) = self
                    .steam_client
                    .get_store_info(steam_id.parse().unwrap())
                    .await
                {
                    entry = entry.with_steam_info(store_info);
                }

                if let Ok(deck_status) = self.steam_client.get_deck_verified(steam_id.clone()).await
                {
                    entry = entry.with_steam_deck_info(deck_status, steam_id.clone());
                }
            }

            if let Ok(Some(detailed)) = self.rawg_client.get_game_info(&entry.title).await {
                entry = entry.with_rawg_info(&detailed);
            }

            entry.title = TitleNormalizer::format_for_display(&entry.title);
            enriched_games.push(entry);
            sleep(Duration::from_millis(650)).await;
        }

        enriched_games.sort_by(|a, b| b.harmony_score.cmp(&a.harmony_score));
        self.store.save_enriched_games(&enriched_games)?;
        Ok(enriched_games)
    }
}
