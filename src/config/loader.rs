use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{AppError, Result};

/// Default model directory name placed inside the user's home directory.
const DEFAULT_MODEL_DIR: &str = "models";
const CONFIG_DIR_NAME: &str = ".llamalauncher";
const CONFIG_FILE_NAME: &str = "config.yml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub llama_bin_path: Option<String>,
    pub model_dir: String,
    pub host: String,
    pub port: u16,
    pub last_model: Option<String>,
    pub last_preset: Option<String>,
    pub debug_logging: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<Preset>,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_model_dir = home.join(DEFAULT_MODEL_DIR);

        Self {
            llama_bin_path: None,
            model_dir: default_model_dir.to_string_lossy().to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            last_model: None,
            last_preset: None,
            debug_logging: false,
            presets: vec![
                Preset {
                    name: "high-throughput".to_string(),
                    ctx_size: Some(8192),
                    n_gpu_layers: None,
                    batch_size: Some(2048),
                    threads: None,
                    description: Some("Maximum throughput, high memory usage".to_string()),
                },
                Preset {
                    name: "low-memory".to_string(),
                    ctx_size: Some(1024),
                    n_gpu_layers: Some(0),
                    batch_size: Some(128),
                    threads: None,
                    description: Some("Minimal memory footprint".to_string()),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub ctx_size: Option<u32>,
    pub n_gpu_layers: Option<u32>,
    pub batch_size: Option<u32>,
    pub threads: Option<u32>,
    pub description: Option<String>,
}

/// Returns the config directory path under the user's home directory.
pub fn config_dir_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(CONFIG_DIR_NAME)
}

/// Returns the full path to the configuration file.
pub fn config_file_path() -> PathBuf {
    config_dir_path().join(CONFIG_FILE_NAME)
}

/// Load the configuration from the default path, or create it with defaults if missing.
pub fn load_or_create_config() -> Result<Config> {
    let path = config_file_path();

    if path.exists() {
        let contents = std::fs::read_to_string(&path).map_err(|e| AppError::ConfigRead {
            path: path.clone(),
            details: e.to_string(),
        })?;
        let config: Config = serde_yaml::from_str(&contents).map_err(|e| AppError::ConfigRead {
            path: path.clone(),
            details: format!("YAML parse error: {e}"),
        })?;
        info!(config_path = %path.display(), "Configuration loaded");
        Ok(config)
    } else {
        info!(
            config_path = %path.display(),
            "No config found, creating default"
        );
        let config = Config::default();
        save_config(&config)?;
        Ok(config)
    }
}

/// Write the configuration to disk.
pub fn save_config(config: &Config) -> Result<()> {
    let path = config_file_path();

    // Ensure the config directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = serde_yaml::to_string(config)?;
    std::fs::write(&path, contents).map_err(|e| AppError::ConfigWrite {
        path: path.clone(),
        details: e.to_string(),
    })?;

    info!(config_path = %path.display(), "Configuration saved");
    Ok(())
}

/// Add or update a preset by name. Returns true if a new preset was added.
#[allow(dead_code)]
pub fn upsert_preset(config: &mut Config, preset: Preset) -> bool {
    if let Some(existing) = config.presets.iter_mut().find(|p| p.name == preset.name) {
        existing.ctx_size = preset.ctx_size;
        existing.n_gpu_layers = preset.n_gpu_layers;
        existing.batch_size = preset.batch_size;
        existing.threads = preset.threads;
        existing.description = preset.description;
        false
    } else {
        config.presets.push(preset);
        true
    }
}

/// Remove a preset by name. Returns true if a preset was removed.
#[allow(dead_code)]
pub fn remove_preset(config: &mut Config, name: &str) -> bool {
    let len = config.presets.len();
    config.presets.retain(|p| p.name != name);
    config.presets.len() < len
}

/// Persist the last model and preset to config and write to disk.
pub fn persist_last_selection(
    config: &mut Config,
    model_path: &str,
    preset_name: Option<&str>,
) -> Result<()> {
    config.last_model = Some(model_path.to_string());
    config.last_preset = preset_name.map(|s| s.to_string());
    save_config(config)
}
