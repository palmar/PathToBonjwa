use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_FILE: &str = "pathtobonjwa.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Directory to scan recursively for .rep files
    #[serde(default)]
    pub replay_folder: Option<String>,

    /// Player name for win/loss detection (matched against replay player names)
    #[serde(default)]
    pub player_name: Option<String>,

    /// Advanced mode: shows player ID config + auto-folder scanning
    #[serde(default)]
    pub advanced_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            replay_folder: None,
            player_name: None,
            advanced_mode: false,
        }
    }
}

impl Settings {
    fn config_path() -> PathBuf {
        // Store next to the executable, or fall back to current dir
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join(CONFIG_FILE);
            }
        }
        PathBuf::from(CONFIG_FILE)
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
