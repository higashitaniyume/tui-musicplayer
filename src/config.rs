use std::path::PathBuf;

use anyhow::Context;
use log::{debug, info};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub music_folders: Vec<PathBuf>,
    #[serde(default)]
    pub xspf_playlists: Vec<PathBuf>,
    /// "en" or "zh" — None means auto-detect from environment
    #[serde(default)]
    pub language: Option<String>,
    /// Audio host name (e.g. "WASAPI", "DirectSound") — None means system default
    #[serde(default)]
    pub audio_host: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            info!("Loading config from: {}", path.display());
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => {
                        debug!("Config loaded successfully");
                        return config;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse config: {e}, using defaults");
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read config file: {e}, using defaults");
                }
            }
        } else {
            info!("No config file found at: {}", path.display());
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create config dir: {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)?;
        debug!("Saving config to: {}", path.display());
        std::fs::write(&path, json)
            .with_context(|| format!("Cannot write config: {}", path.display()))?;
        info!("Config saved: {} folders, {} xspf playlists",
            self.music_folders.len(), self.xspf_playlists.len());
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_folders: Vec::new(),
            xspf_playlists: Vec::new(),
            language: None,
            audio_host: None,
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tui-musicplayer")
        .join("config.json")
}
