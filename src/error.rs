use thiserror::Error;

#[derive(Error, Debug)]
pub enum KicodeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Tool execution error: {0}")]
    Tool(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Ambiguous edit: found {0} matches for search text")]
    AmbiguousEdit(usize),

    #[error("Text not found in file")]
    TextNotFound,

    #[error("Command rejected by user")]
    CommandRejected,

    #[error("Setup cancelled by user")]
    SetupCancelled,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),
}

pub type Result<T> = std::result::Result<T, KicodeError>;
