mod clients;
mod scrapers;
mod storage;

pub use clients::{
    rawg::{RawgClient, RawgGameBasic, RawgGameDetailed},
    steam::{ExtendedPlatforms, SteamApp, SteamClient, SteamDeckVerifiedResponse, StoreInfo},
};
pub use scrapers::{
    eurogamer::EurogamerScraper, ign::IGNScraper, pcgamer::PCGamerScraper,
    polygon_ps5_top25::PolygonPS5Top25, rockpapershotgun::RPSScraper, Selectors, WebsiteScraper,
};
pub use storage::fs_store::FileSystemStore;
