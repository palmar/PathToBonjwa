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
    #[allow(dead_code)]
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

    // Full parse to get commands for win/loss detection
    let replay = parser::parse_replay(&data).ok()?;

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

    let my_player = match my_player {
        Some(p) => p,
        None => return "Not in game".to_string(),
    };

    let human_players: Vec<_> = replay
        .players
        .iter()
        .filter(|p| matches!(p.player_type, parser::PlayerType::Human))
        .collect();

    // Collect all LeaveGame commands
    let leave_cmds: Vec<_> = replay
        .commands
        .iter()
        .filter(|cmd| matches!(cmd.cmd, parser::CmdType::LeaveGame { .. }))
        .collect();

    if !leave_cmds.is_empty() && human_players.len() == 2 {
        let opponent = human_players
            .iter()
            .find(|p| p.player_id != my_player.player_id);

        if let Some(opponent) = opponent {
            // Check if only one player has a LeaveGame — they left the game (loser).
            let my_leaves: Vec<_> = leave_cmds
                .iter()
                .filter(|c| c.player_id == my_player.player_id)
                .collect();
            let opp_leaves: Vec<_> = leave_cmds
                .iter()
                .filter(|c| c.player_id == opponent.player_id)
                .collect();

            match (my_leaves.first(), opp_leaves.first()) {
                (Some(_), None) => return "Loss".to_string(), // Only I left
                (None, Some(_)) => return "Win".to_string(),  // Only opponent left
                (Some(my_leave), Some(opp_leave)) => {
                    // Both players have LeaveGame. The one at the earlier frame
                    // left first and lost.
                    if my_leave.frame < opp_leave.frame {
                        return "Loss".to_string();
                    } else if opp_leave.frame < my_leave.frame {
                        return "Win".to_string();
                    }
                    // Same frame — fall through to action-based heuristic
                }
                (None, None) => {} // LeaveGame from non-player? Fall through
            }
        }
    } else if !leave_cmds.is_empty() {
        // Non-1v1: the first player to LeaveGame with reason=1 (quit) lost
        let first_quit = leave_cmds.iter().find(|cmd| {
            if let parser::CmdType::LeaveGame { reason } = &cmd.cmd {
                *reason == 1 || *reason == 6
            } else {
                false
            }
        });
        if let Some(cmd) = first_quit {
            return if cmd.player_id == my_player.player_id {
                "Loss".to_string()
            } else {
                "Win".to_string()
            };
        }
    }

    // Fallback: no LeaveGame or inconclusive. For 1v1, compare each player's
    // last gameplay command. The player who stopped issuing commands first
    // likely quit/disconnected (loser). Exclude KeepAlive/Chat/LeaveGame
    // since those aren't active gameplay actions.
    if human_players.len() == 2 {
        let is_gameplay = |cmd: &parser::Command| -> bool {
            !matches!(
                cmd.cmd,
                parser::CmdType::KeepAlive
                    | parser::CmdType::Chat { .. }
                    | parser::CmdType::LeaveGame { .. }
                    | parser::CmdType::Other { .. }
            )
        };

        let my_last_frame = replay
            .commands
            .iter()
            .rev()
            .find(|c| c.player_id == my_player.player_id && is_gameplay(c))
            .map(|c| c.frame);

        let opp_last_frame = replay
            .commands
            .iter()
            .rev()
            .find(|c| {
                c.player_id != my_player.player_id
                    && human_players.iter().any(|p| p.player_id == c.player_id)
                    && is_gameplay(c)
            })
            .map(|c| c.frame);

        if let (Some(my_frame), Some(opp_frame)) = (my_last_frame, opp_last_frame) {
            // Require a meaningful gap (>= 24 frames ≈ 1 second) to be confident
            if opp_frame + 24 <= my_frame {
                return "Win".to_string(); // Opponent stopped first → I won
            } else if my_frame + 24 <= opp_frame {
                return "Loss".to_string(); // I stopped first → I lost
            }
        }
    }

    "Undetermined".to_string()
}
