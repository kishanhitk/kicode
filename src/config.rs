use crate::error::{KicodeError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_MODEL: &str = "x-ai/grok-code-fast-1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SafetyConfig {
    #[serde(default)]
    pub additional_patterns: Vec<String>,
    #[serde(default)]
    pub skip_confirmation: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub safety: SafetyConfig,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub safety: SafetyConfig,
}

impl Config {
    pub fn load(cli_model: Option<String>) -> Result<Self> {
        let file_config = Self::load_file_config()?;

        let api_key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .or(file_config.api_key)
            .ok_or_else(|| {
                KicodeError::Config(
                    "API key not found. Set OPENROUTER_API_KEY env var or add api_key to config file".to_string()
                )
            })?;

        let model = cli_model
            .or_else(|| std::env::var("KICODE_MODEL").ok())
            .or(file_config.model)
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        Ok(Config {
            api_key,
            model,
            safety: file_config.safety,
        })
    }

    /// Load config from file only (ignores environment variables).
    /// Useful for setup wizards that need to show/modify file-based config.
    pub fn load_file_config() -> Result<FileConfig> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: FileConfig = toml::from_str(&content)
                .map_err(|e| KicodeError::Config(format!("Invalid config file: {}", e)))?;
            Ok(config)
        } else {
            Ok(FileConfig::default())
        }
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kicode")
            .join("config.toml")
    }

    pub fn default_model() -> &'static str {
        DEFAULT_MODEL
    }
}

impl FileConfig {
    pub fn save(&self) -> Result<()> {
        let path = Config::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| KicodeError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
