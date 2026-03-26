//! Headless replay re-simulation using the OpenBW bridge.
//!
//! Loads a .rep file, feeds its header + command stream to the OpenBW
//! simulator, advances frame-by-frame, and captures per-frame game state
//! (units, supply, resources) for analytics.

use crate::openbw::{FrameState, SimError, Simulator};

/// How often to sample full game state (every N frames).
/// At ~23.81 fps, sampling every 24 frames ≈ 1 sample/sec.
const DEFAULT_SAMPLE_INTERVAL: u32 = 24;

/// Result of a full replay simulation.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Total frames simulated.
    pub total_frames: u32,
    /// Sampled frame states at the configured interval.
    pub snapshots: Vec<FrameState>,
    /// Final game state at the last frame.
    pub final_state: FrameState,
}

impl SimulationResult {
    /// Get supply curve data for a given player: Vec<(frame, supply_used, supply_max)>.
    pub fn supply_curve(&self, player_id: u8) -> Vec<(u32, f64, f64)> {
        self.snapshots
            .iter()
            .filter_map(|snap| {
                snap.players
                    .iter()
                    .find(|p| p.player_id == player_id)
                    .map(|p| (snap.frame, p.supply_used, p.supply_max))
            })
            .collect()
    }

    /// Get resource curve for a given player: Vec<(frame, minerals, gas)>.
    pub fn resource_curve(&self, player_id: u8) -> Vec<(u32, i32, i32)> {
        self.snapshots
            .iter()
            .filter_map(|snap| {
                snap.players
                    .iter()
                    .find(|p| p.player_id == player_id)
                    .map(|p| (snap.frame, p.minerals, p.gas))
            })
            .collect()
    }

    /// Get unit count over time for a player: Vec<(frame, alive_count)>.
    pub fn unit_count_curve(&self, player_id: u8) -> Vec<(u32, usize)> {
        self.snapshots
            .iter()
            .map(|snap| {
                let count = snap
                    .units
                    .iter()
                    .filter(|u| u.player_id == player_id && u.alive)
                    .count();
                (snap.frame, count)
            })
            .collect()
    }

    /// Get army composition at a specific frame (nearest snapshot).
    pub fn army_at_frame(&self, frame: u32, player_id: u8) -> Vec<(u16, usize)> {
        let snap = self
            .snapshots
            .iter()
            .min_by_key(|s| (s.frame as i64 - frame as i64).unsigned_abs());
        match snap {
            Some(s) => {
                let mut counts: std::collections::HashMap<u16, usize> =
                    std::collections::HashMap::new();
                for u in &s.units {
                    if u.player_id == player_id && u.alive {
                        *counts.entry(u.unit_id).or_insert(0) += 1;
                    }
                }
                let mut v: Vec<_> = counts.into_iter().collect();
                v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
                v
            }
            None => Vec::new(),
        }
    }
}

/// Configuration for a simulation run.
pub struct SimConfig {
    /// How many frames between state snapshots.
    pub sample_interval: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            sample_interval: DEFAULT_SAMPLE_INTERVAL,
        }
    }
}

/// Run a headless simulation of a replay from raw section data.
///
/// - `header`: decompressed section 1 (633 bytes)
/// - `commands`: decompressed section 2 (command stream)
/// - `config`: sampling configuration
pub fn simulate_replay(
    header: &[u8],
    commands: &[u8],
    config: &SimConfig,
) -> Result<SimulationResult, SimError> {
    let mut sim = Simulator::new()?;
    sim.load_replay(header, commands)?;

    let mut snapshots = Vec::new();

    // Capture initial state (frame 0)
    snapshots.push(sim.snapshot());

    while !sim.is_done() {
        sim.next_frame()?;
        let frame = sim.current_frame();
        if frame % config.sample_interval == 0 {
            snapshots.push(sim.snapshot());
        }
    }

    let final_state = sim.snapshot();
    let total_frames = sim.current_frame();

    // Ensure final frame is captured
    if snapshots.last().map_or(true, |s| s.frame != total_frames) {
        snapshots.push(final_state.clone());
    }

    Ok(SimulationResult {
        total_frames,
        snapshots,
        final_state,
    })
}

/// Extract raw decompressed header and command sections from a .rep file.
///
/// This duplicates some work from `parser::parse_replay` but returns the
/// raw bytes needed by the simulator rather than parsed structures.
pub fn extract_replay_sections(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    // Re-use the parser's section reader logic. We need to make it accessible.
    // For now, we call into the parser module's extraction helper.
    crate::parser::extract_raw_sections(data)
}
