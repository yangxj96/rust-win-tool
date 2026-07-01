use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Custom error: {0}")]
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub version: String,
    pub debug: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "Rust Win Tool".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            debug: cfg!(debug_assertions),
        }
    }
}

pub fn get_app_info() -> AppConfig {
    AppConfig::default()
}
