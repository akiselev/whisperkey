use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use config::{Config, ConfigError, File};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub model_path: Option<String>,
    pub enable_denoise: bool,
    pub enable_vad: bool,
    pub vad_mode: VadMode,
    pub vad_energy_threshold: f32, // Energy threshold for VAD (0.0 to 1.0)
    pub silence_threshold_ms: u32, // Time in ms to consider silence
    pub enable_keyboard_output: bool, // Enable keyboard output typing
    pub keyboard_output_delay_ms: u32, // Delay before typing begins
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum VadMode {
    Quality,
    LowBitrate,
    Aggressive,
    VeryAggressive,
}

impl From<VadMode> for webrtc_vad::VadMode {
    fn from(mode: VadMode) -> Self {
        match mode {
            VadMode::Quality => webrtc_vad::VadMode::Quality,
            VadMode::LowBitrate => webrtc_vad::VadMode::LowBitrate,
            VadMode::Aggressive => webrtc_vad::VadMode::Aggressive,
            VadMode::VeryAggressive => webrtc_vad::VadMode::VeryAggressive,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model_path: None,
            enable_denoise: true,
            enable_vad: true,
            vad_mode: VadMode::Quality,
            vad_energy_threshold: 0.01, // Default threshold (lower values are more sensitive)
            silence_threshold_ms: 1000, // 1 second of silence
            enable_keyboard_output: false, // Disabled by default for safety
            keyboard_output_delay_ms: 500, // 500ms delay by default
        }
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
