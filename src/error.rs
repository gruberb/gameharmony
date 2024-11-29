use thiserror::Error;

#[derive(Error, Debug)]
pub enum GameError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Selector error: {0}")]
    Selector(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, GameError>;
