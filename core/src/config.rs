use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use config::{Config, ConfigError, File};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub model_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { model_path: None }
    }
}

pub fn get_config_dir() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("whisperkey")
}

pub fn get_config_file_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

pub fn load_config() -> Result<Arc<Settings>, ConfigError> {
    let config_dir = get_config_dir();
    let config_file = get_config_file_path();

    // Create the config directory if it doesn't exist
    if !config_dir.exists() {
        if let Err(e) = fs::create_dir_all(&config_dir) {
            error!("Failed to create config directory: {}", e);
            // Continue anyway, we'll use defaults
        } else {
            info!("Created config directory: {:?}", config_dir);
        }
    }

    // Start with default settings
    let default_settings = Settings::default();

    // If the config file doesn't exist, create it with default settings
    if !config_file.exists() {
        save_config(&default_settings).unwrap_or_else(|e| {
            error!("Failed to save default config: {}", e);
        });
    }

    // Load and parse the config file
    let s = Config::builder()
        .add_source(File::from(config_file))
        .build()?;

    // Deserialize the config
    let settings: Settings = s.try_deserialize()?;
    Ok(Arc::new(settings))
}

pub fn save_config(settings: &Settings) -> Result<(), ConfigError> {
    let config_file = get_config_file_path();

    // Serialize the settings to TOML
    let toml = toml::to_string_pretty(settings)
        .map_err(|e| ConfigError::Message(format!("Failed to serialize config: {}", e)))?;

    // Write the config file
    fs::write(&config_file, toml)
        .map_err(|e| ConfigError::Message(format!("Failed to write config file: {}", e)))?;

    info!("Config saved to {:?}", config_file);
    Ok(())
}
