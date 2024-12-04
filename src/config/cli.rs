use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to scraper configuration file
    #[arg(long, default_value = "scraper_config.json")]
    pub config_file: PathBuf,

    /// Directory to store output data
    #[arg(long, default_value = "data")]
    pub data_dir: PathBuf,

    /// Directory for caching intermediate results
    #[arg(long, default_value = "cache")]
    pub cache_dir: PathBuf,

    /// RAWG API key for game data enrichment
    #[clap(long, env = "RAWG_API_KEY")]
    pub rawg_api_key: Option<String>,

    /// Skip using cached data
    #[arg(long)]
    pub skip_cache: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Download game header images from manifest
    Publish {
        /// Source manifest file
        #[arg(long, default_value = "data/manifest.json")]
        manifest: PathBuf,

        /// GitHub username
        #[arg(long)]
        username: String,

        /// Repository name
        #[arg(long)]
        repo: String,
    },
}
