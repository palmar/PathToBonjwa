use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::parser;

/// Lightweight replay metadata for the list view (no commands parsed).
#[derive(Debug, Clone)]
pub struct ReplayEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub map_name: String,
    pub matchup: String,
    pub duration_secs: f64,
    pub timestamp: i64,
    /// "Win", "Loss", or "Undetermined"
    pub result: String,
    pub player_names: Vec<String>,
}

impl ReplayEntry {
    /// Format duration as M:SS
    pub fn duration_str(&self) -> String {
        let mins = (self.duration_secs / 60.0) as u32;
        let secs = (self.duration_secs % 60.0) as u32;
        format!("{}:{:02}", mins, secs)
    }

    /// Format timestamp as date string
    pub fn date_str(&self) -> String {
        chrono::DateTime::from_timestamp(self.timestamp, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

/// Scan a directory recursively for .rep files and parse headers.
/// Returns entries sorted by timestamp descending (newest first).
pub fn scan_folder(folder: &Path, player_name: Option<&str>) -> Vec<ReplayEntry> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(folder)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("rep") {
                if let Some(re) = parse_entry(path, player_name) {
                    entries.push(re);
                }
            }
        }
    }

    // Sort newest first
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries
}

fn parse_entry(path: &Path, player_name: Option<&str>) -> Option<ReplayEntry> {
    let data = std::fs::read(path).ok()?;
    let replay = parser::parse_replay_header(&data).ok()?;

    let player_names: Vec<String> = replay.players.iter().map(|p| p.name.clone()).collect();

    let result = match player_name {
        Some(name) if !name.is_empty() => determine_result(&replay, name),
        _ => "Undetermined".to_string(),
    };

    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();

    Some(ReplayEntry {
        path: path.to_path_buf(),
        file_name,
        map_name: replay.map_name,
        matchup: replay.matchup,
        duration_secs: replay.duration_secs,
        timestamp: replay.timestamp,
        result,
        player_names,
    })
}

fn determine_result(replay: &parser::Replay, player_name: &str) -> String {
    let lower = player_name.to_lowercase();

    // Find the player in this replay by name (case-insensitive substring match)
    let my_player = replay
        .players
        .iter()
        .find(|p| p.name.to_lowercase().contains(&lower));

    match my_player {
        Some(_p) => {
            // Without commands we can't detect LeaveGame — header only.
            // We'd need to do a full parse to know who left first.
            // For now: if the player is in the replay we note it, but can't
            // determine win/loss from header alone.
            "Undetermined".to_string()
        }
        None => "Not in game".to_string(),
    }
}
