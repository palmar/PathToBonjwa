use crate::parser::*;
use std::collections::HashMap;

const FRAMES_PER_SECOND: f64 = 23.81;
const FRAMES_PER_MINUTE: f64 = FRAMES_PER_SECOND * 60.0;

// ─── APM / EAPM ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ApmData {
    /// Per-minute buckets: (minute_index, action_count)
    pub apm_per_minute: Vec<(f64, f64)>,
    pub eapm_per_minute: Vec<(f64, f64)>,
    pub avg_apm: f64,
    pub avg_eapm: f64,
}

/// Returns true if the command is a "real" player action (not KeepAlive, sync, etc.)
fn is_player_action(cmd: &CmdType) -> bool {
    !matches!(
        cmd,
        CmdType::KeepAlive | CmdType::Other { .. } | CmdType::Chat { .. }
    )
}

/// Returns true if the command should be filtered out for EAPM.
/// Simplified EAPM: filters redundant selects, spam clicks, fast cancels.
fn is_ineffective(cmd: &Command, prev: Option<&Command>) -> bool {
    let prev = match prev {
        Some(p) => p,
        None => return false, // First command is always effective
    };

    let delta = cmd.frame.saturating_sub(prev.frame);

    // Fast selection spam (< 8 frames apart, both selections)
    if delta <= 8 && is_selection_changer(&cmd.cmd) && is_selection_changer(&prev.cmd) {
        // Allow double-tap hotkey for centering
        if let (
            CmdType::Hotkey {
                hotkey_type: HotkeyAction::Select,
                group: g1,
            },
            CmdType::Hotkey {
                hotkey_type: HotkeyAction::Select,
                group: g2,
            },
        ) = (&cmd.cmd, &prev.cmd)
        {
            if g1 == g2 {
                return false; // Double-tap is OK
            }
        }
        // Allow SelectAdd/SelectRemove near other selections
        if matches!(cmd.cmd, CmdType::Select { .. }) {
            // Only raw Select is ineffective here, not hotkey selects
        }
        return true;
    }

    // Fast cancel (< 20 frames)
    if delta <= 20 {
        match (&prev.cmd, &cmd.cmd) {
            (CmdType::Train { .. } | CmdType::TrainFighter, CmdType::CancelTrain) => return true,
            (CmdType::UnitMorph { .. } | CmdType::BuildingMorph { .. }, CmdType::CancelMorph) => {
                return true
            }
            (CmdType::Upgrade { .. }, CmdType::CancelUpgrade) => return true,
            (CmdType::Tech { .. }, CmdType::CancelTech) => return true,
            _ => {}
        }
    }

    // Fast repetition of same command (< 10 frames)
    if delta <= 10 {
        match (&prev.cmd, &cmd.cmd) {
            (CmdType::Stop, CmdType::Stop) | (CmdType::HoldPosition, CmdType::HoldPosition) => {
                return true
            }
            _ => {}
        }
    }

    // Repeated hotkey assign/add
    if let (
        CmdType::Hotkey {
            hotkey_type: ht1,
            group: g1,
        },
        CmdType::Hotkey {
            hotkey_type: ht2,
            group: g2,
        },
    ) = (&cmd.cmd, &prev.cmd)
    {
        if g1 == g2 && *ht1 != HotkeyAction::Select && ht1 == ht2 {
            return true;
        }
    }

    false
}

fn is_selection_changer(cmd: &CmdType) -> bool {
    matches!(
        cmd,
        CmdType::Select { .. }
            | CmdType::Hotkey {
                hotkey_type: HotkeyAction::Select,
                ..
            }
    )
}

pub fn compute_apm(commands: &[Command], player_id: u8, total_frames: u32) -> ApmData {
    let total_minutes = total_frames as f64 / FRAMES_PER_MINUTE;
    let num_buckets = (total_minutes.ceil() as usize).max(1);

    let player_cmds: Vec<&Command> = commands
        .iter()
        .filter(|c| c.player_id == player_id && is_player_action(&c.cmd))
        .collect();

    // APM per minute
    let mut apm_buckets = vec![0.0f64; num_buckets];
    let mut eapm_buckets = vec![0.0f64; num_buckets];

    for (i, cmd) in player_cmds.iter().enumerate() {
        let minute = (cmd.frame as f64 / FRAMES_PER_MINUTE) as usize;
        let bucket = minute.min(num_buckets - 1);
        apm_buckets[bucket] += 1.0;

        let prev: Option<&Command> = if i > 0 {
            Some(player_cmds[i - 1])
        } else {
            None
        };
        if !is_ineffective(cmd, prev) {
            eapm_buckets[bucket] += 1.0;
        }
    }

    let apm_per_minute: Vec<(f64, f64)> = apm_buckets
        .iter()
        .enumerate()
        .map(|(i, &count)| (i as f64, count))
        .collect();

    let eapm_per_minute: Vec<(f64, f64)> = eapm_buckets
        .iter()
        .enumerate()
        .map(|(i, &count)| (i as f64, count))
        .collect();

    let total_actions: f64 = apm_buckets.iter().sum();
    let total_effective: f64 = eapm_buckets.iter().sum();

    let avg_apm = if total_minutes > 0.0 {
        total_actions / total_minutes
    } else {
        0.0
    };
    let avg_eapm = if total_minutes > 0.0 {
        total_effective / total_minutes
    } else {
        0.0
    };

    ApmData {
        apm_per_minute,
        eapm_per_minute,
        avg_apm,
        avg_eapm,
    }
}

// ─── Build order ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BuildOrderEntry {
    pub frame: u32,
    pub time_str: String,
    pub unit_name: String,
    pub unit_id: u16,
}

pub fn extract_build_order(commands: &[Command], player_id: u8) -> Vec<BuildOrderEntry> {
    let mut entries = Vec::new();

    for cmd in commands {
        if cmd.player_id != player_id {
            continue;
        }

        let unit_id = match &cmd.cmd {
            CmdType::Build { unit_id, .. } => Some(*unit_id),
            CmdType::Train { unit_id } => Some(*unit_id),
            CmdType::UnitMorph { unit_id } => Some(*unit_id),
            CmdType::BuildingMorph { unit_id } => Some(*unit_id),
            _ => None,
        };

        if let Some(uid) = unit_id {
            let name = unit_name(uid);
            if name == "Unknown" {
                continue;
            }
            let secs = cmd.frame as f64 / FRAMES_PER_SECOND;
            let mins = (secs / 60.0) as u32;
            let s = (secs % 60.0) as u32;
            entries.push(BuildOrderEntry {
                frame: cmd.frame,
                time_str: format!("{}:{:02}", mins, s),
                unit_name: name.to_string(),
                unit_id: uid,
            });
        }
    }

    entries
}

// ─── Unit production counts ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UnitCount {
    pub unit_name: String,
    pub unit_id: u16,
    pub count: u32,
    pub is_building: bool,
}

pub fn compute_unit_counts(commands: &[Command], player_id: u8) -> Vec<UnitCount> {
    let mut counts: HashMap<u16, u32> = HashMap::new();

    for cmd in commands {
        if cmd.player_id != player_id {
            continue;
        }

        let unit_id = match &cmd.cmd {
            CmdType::Build { unit_id, .. } => Some(*unit_id),
            CmdType::Train { unit_id } => Some(*unit_id),
            CmdType::UnitMorph { unit_id } => Some(*unit_id),
            CmdType::BuildingMorph { unit_id } => Some(*unit_id),
            _ => None,
        };

        if let Some(uid) = unit_id {
            if unit_name(uid) != "Unknown" {
                *counts.entry(uid).or_insert(0) += 1;
            }
        }
    }

    let mut result: Vec<UnitCount> = counts
        .into_iter()
        .map(|(uid, count)| UnitCount {
            unit_name: unit_name(uid).to_string(),
            unit_id: uid,
            count,
            is_building: is_building(uid),
        })
        .collect();

    // Sort: buildings first, then units, each by count descending
    result.sort_by(|a, b| {
        b.is_building
            .cmp(&a.is_building)
            .then(b.count.cmp(&a.count))
    });

    result
}

// ─── Hotkey stats ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HotkeyStats {
    pub groups: [HotkeyGroup; 10],
}

#[derive(Debug, Clone, Default)]
pub struct HotkeyGroup {
    pub assigns: u32,
    pub selects: u32,
    pub adds: u32,
}

impl HotkeyGroup {
    pub fn total(&self) -> u32 {
        self.assigns + self.selects + self.adds
    }
}

pub fn compute_hotkey_stats(commands: &[Command], player_id: u8) -> HotkeyStats {
    let mut stats = HotkeyStats {
        groups: Default::default(),
    };

    for cmd in commands {
        if cmd.player_id != player_id {
            continue;
        }

        if let CmdType::Hotkey { hotkey_type, group } = &cmd.cmd {
            let idx = (*group as usize).min(9);
            match hotkey_type {
                HotkeyAction::Assign => stats.groups[idx].assigns += 1,
                HotkeyAction::Select => stats.groups[idx].selects += 1,
                HotkeyAction::Add => stats.groups[idx].adds += 1,
                _ => {}
            }
        }
    }

    stats
}
