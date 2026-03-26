use crate::parser::costs;
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
        CmdType::KeepAlive
            | CmdType::Other { .. }
            | CmdType::Chat { .. }
            | CmdType::LeaveGame { .. }
            | CmdType::MinimapPing { .. }
    )
}

/// Returns true if the command should be filtered out for EAPM.
/// Filters redundant selects, spam clicks, fast cancels, worker training spam,
/// repeated right-clicks, and micro-interval spam.
fn is_ineffective(cmd: &Command, prev: Option<&Command>) -> bool {
    let prev = match prev {
        Some(p) => p,
        None => return false, // First command is always effective
    };

    let delta = cmd.frame.saturating_sub(prev.frame);

    // Micro-interval spam: any two actions within 2 frames (~84ms) are almost
    // certainly double-registrations or hardware artifacts.
    if delta <= 2 {
        return true;
    }

    // Selection cycling spam (< 28 frames, ~1.2s): rapidly switching selections
    // without issuing orders is pure APM padding.
    if delta <= 28 && is_selection_changer(&cmd.cmd) && is_selection_changer(&prev.cmd) {
        // Allow double-tap hotkey for centering camera
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
                return false; // Double-tap same group is intentional (center view)
            }
        }
        return true;
    }

    // Worker training spam (< 36 frames, ~1.5s): rapidly queuing the same worker
    // type (e.g. probe spam). One click per production cycle is sufficient.
    if delta <= 36 {
        if let (CmdType::Train { unit_id: id1 }, CmdType::Train { unit_id: id2 }) =
            (&cmd.cmd, &prev.cmd)
        {
            if id1 == id2 && costs::is_worker(*id1) {
                return true;
            }
        }
    }

    // Right-click position spam (< 12 frames, ~0.5s): repeated right-clicks to
    // the same approximate location (within 4-tile radius).
    if delta <= 12 {
        if let (
            CmdType::RightClick { x: x1, y: y1, .. },
            CmdType::RightClick { x: x2, y: y2, .. },
        ) = (&cmd.cmd, &prev.cmd)
        {
            let dx = (*x1 as i32 - *x2 as i32).abs();
            let dy = (*y1 as i32 - *y2 as i32).abs();
            if dx <= 128 && dy <= 128 {
                // ~4 tiles in BW pixel coords (32px/tile)
                return true;
            }
        }
    }

    // Targeted order spam (< 12 frames): same order type to nearby location.
    if delta <= 12 {
        if let (
            CmdType::TargetedOrder {
                x: x1,
                y: y1,
                order_id: o1,
                ..
            },
            CmdType::TargetedOrder {
                x: x2,
                y: y2,
                order_id: o2,
                ..
            },
        ) = (&cmd.cmd, &prev.cmd)
        {
            if o1 == o2 {
                let dx = (*x1 as i32 - *x2 as i32).abs();
                let dy = (*y1 as i32 - *y2 as i32).abs();
                if dx <= 128 && dy <= 128 {
                    return true;
                }
            }
        }
    }

    // Fast cancel (< 30 frames, ~1.3s): action immediately cancelled = misclick.
    if delta <= 30 {
        match (&prev.cmd, &cmd.cmd) {
            (CmdType::Train { .. } | CmdType::TrainFighter, CmdType::CancelTrain) => return true,
            (CmdType::UnitMorph { .. } | CmdType::BuildingMorph { .. }, CmdType::CancelMorph) => {
                return true
            }
            (CmdType::Build { .. }, CmdType::CancelBuild) => return true,
            (CmdType::Upgrade { .. }, CmdType::CancelUpgrade) => return true,
            (CmdType::Tech { .. }, CmdType::CancelTech) => return true,
            _ => {}
        }
    }

    // Fast repetition of same command (< 20 frames, ~0.84s)
    if delta <= 20 {
        match (&prev.cmd, &cmd.cmd) {
            (CmdType::Stop, CmdType::Stop) | (CmdType::HoldPosition, CmdType::HoldPosition) => {
                return true
            }
            (CmdType::ReturnCargo, CmdType::ReturnCargo) => return true,
            _ => {}
        }
    }

    // Repeated hotkey assign/add to same group
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

/// Dedup window: commands for the same unit within this many frames are collapsed.
/// ~1 second at 23.81 fps — filters hotkey spam while preserving intentional queues.
const BUILD_ORDER_DEDUP_FRAMES: u32 = 24;

pub fn extract_build_order(
    commands: &[Command],
    player_id: u8,
    _race: &Race,
) -> Vec<BuildOrderEntry> {
    let mut entries = Vec::new();
    // Track last frame we accepted each unit_id to deduplicate spam
    let mut last_frame_for_unit: HashMap<u16, u32> = HashMap::new();

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

            // Skip if we already recorded this unit within the dedup window
            if let Some(&last_frame) = last_frame_for_unit.get(&uid) {
                if cmd.frame.saturating_sub(last_frame) < BUILD_ORDER_DEDUP_FRAMES {
                    continue;
                }
            }
            last_frame_for_unit.insert(uid, cmd.frame);

            // Filter out worker production from the build order display
            if costs::is_worker(uid) {
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

// ─── Idle time / macro gap analysis ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IdleGap {
    pub start_secs: f64,
    pub end_secs: f64,
    pub duration_secs: f64,
}

#[derive(Debug, Clone)]
pub struct IdleAnalysis {
    pub gaps: Vec<IdleGap>,
    pub total_idle_secs: f64,
    pub longest_gap_secs: f64,
    pub gap_count: usize,
}

/// Detects periods of inactivity (no commands) longer than `threshold_secs`.
/// Default threshold: 5 seconds (about 119 frames at fastest speed).
pub fn compute_idle_gaps(
    commands: &[Command],
    player_id: u8,
    total_frames: u32,
    threshold_secs: f64,
) -> IdleAnalysis {
    let player_cmds: Vec<f64> = commands
        .iter()
        .filter(|c| c.player_id == player_id && is_player_action(&c.cmd))
        .map(|c| c.frame as f64 / FRAMES_PER_SECOND)
        .collect();

    let mut gaps = Vec::new();
    let mut total_idle = 0.0;
    let mut longest = 0.0;

    if player_cmds.is_empty() {
        let total_secs = total_frames as f64 / FRAMES_PER_SECOND;
        return IdleAnalysis {
            gaps: vec![IdleGap {
                start_secs: 0.0,
                end_secs: total_secs,
                duration_secs: total_secs,
            }],
            total_idle_secs: total_secs,
            longest_gap_secs: total_secs,
            gap_count: 1,
        };
    }

    // Check gap from game start to first command
    if player_cmds[0] > threshold_secs {
        let gap = IdleGap {
            start_secs: 0.0,
            end_secs: player_cmds[0],
            duration_secs: player_cmds[0],
        };
        total_idle += gap.duration_secs;
        if gap.duration_secs > longest {
            longest = gap.duration_secs;
        }
        gaps.push(gap);
    }

    // Check gaps between consecutive commands
    for window in player_cmds.windows(2) {
        let delta = window[1] - window[0];
        if delta > threshold_secs {
            let gap = IdleGap {
                start_secs: window[0],
                end_secs: window[1],
                duration_secs: delta,
            };
            total_idle += gap.duration_secs;
            if gap.duration_secs > longest {
                longest = gap.duration_secs;
            }
            gaps.push(gap);
        }
    }

    let count = gaps.len();
    IdleAnalysis {
        gaps,
        total_idle_secs: total_idle,
        longest_gap_secs: longest,
        gap_count: count,
    }
}

// ─── CSV export ─────────────────────────────────────────────────────────────

/// Generate CSV content for a single replay's analytics.
pub fn export_csv(
    replay: &Replay,
    apm_data: &[(u8, String, ApmData)],
    build_orders: &[(u8, String, Vec<BuildOrderEntry>)],
    idle_analyses: &[(u8, String, IdleAnalysis)],
) -> String {
    let mut csv = String::new();

    // Game info header
    csv.push_str("# Game Info\n");
    csv.push_str("Map,Duration (s),Matchup,Date,Speed,Type\n");
    csv.push_str(&format!(
        "{},{:.0},{},{},{},{}\n\n",
        escape_csv(&replay.map_name),
        replay.duration_secs,
        replay.matchup,
        replay.timestamp,
        replay.game_speed,
        replay.game_type,
    ));

    // APM summary
    csv.push_str("# APM Summary\n");
    csv.push_str("Player,Avg APM,Avg EAPM\n");
    for (_, name, apm) in apm_data {
        csv.push_str(&format!(
            "{},{:.1},{:.1}\n",
            escape_csv(name),
            apm.avg_apm,
            apm.avg_eapm,
        ));
    }
    csv.push('\n');

    // Idle analysis
    csv.push_str("# Idle Analysis\n");
    csv.push_str("Player,Gap Count,Total Idle (s),Longest Gap (s)\n");
    for (_, name, idle) in idle_analyses {
        csv.push_str(&format!(
            "{},{},{:.1},{:.1}\n",
            escape_csv(name),
            idle.gap_count,
            idle.total_idle_secs,
            idle.longest_gap_secs,
        ));
    }
    csv.push('\n');

    // Build orders per player
    for (_, name, entries) in build_orders {
        csv.push_str(&format!("# Build Order — {}\n", name));
        csv.push_str("Time,Unit\n");
        for entry in entries {
            csv.push_str(&format!(
                "{},{}\n",
                entry.time_str,
                escape_csv(&entry.unit_name),
            ));
        }
        csv.push('\n');
    }

    csv
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
