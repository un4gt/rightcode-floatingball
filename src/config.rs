use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_api_base")]
    pub api_base: String,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default)]
    pub bearer_token: String,
    #[serde(default)]
    pub cookie: String,
    #[serde(default = "default_refresh_seconds")]
    pub refresh_seconds: u64,
    #[serde(default = "default_preferred_subscription_name")]
    pub preferred_subscription_name: String,
}

fn default_api_base() -> String {
    "https://right.codes".to_string()
}

fn default_refresh_seconds() -> u64 {
    60
}

fn default_user_agent() -> String {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:146.0) Gecko/20100101 Firefox/146.0"
        .to_string()
}

fn default_preferred_subscription_name() -> String {
    "小股东套餐".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_base: default_api_base(),
            user_agent: default_user_agent(),
            bearer_token: String::new(),
            cookie: String::new(),
            refresh_seconds: default_refresh_seconds(),
            preferred_subscription_name: default_preferred_subscription_name(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unable to resolve a config directory")]
    MissingConfigDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    pub path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self, ConfigError> {
        let project_dirs =
            ProjectDirs::from("codes", "rightcode", "rightcode-floatingball")
                .ok_or(ConfigError::MissingConfigDir)?;
        let path = project_dirs.config_dir().join("config.toml");
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<AppConfig, ConfigError> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }
        let raw = std::fs::read_to_string(&self.path)?;
        Ok(toml::from_str::<AppConfig>(&raw)?)
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(config)?;
        std::fs::write(&self.path, raw)?;
        Ok(())
    }

    pub fn display_path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

pub fn normalize_bearer_token(input: &str) -> String {
    let token = input.trim();
    if token.is_empty() {
        return String::new();
    }

    if token.to_ascii_lowercase().starts_with("bearer ") {
        token.to_string()
    } else {
        format!("Bearer {token}")
    }
}

pub fn normalize_cookie_header_value(input: &str) -> String {
    let cookie = input.trim();
    if cookie.is_empty() {
        return String::new();
    }

    if cookie.contains("cf_clearance=") {
        cookie.to_string()
    } else {
        format!("cf_clearance={cookie}")
    }
}

pub fn is_configured(config: &AppConfig) -> bool {
    !config.bearer_token.trim().is_empty() && !config.cookie.trim().is_empty()
}

pub fn try_parse_refresh_seconds(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}
