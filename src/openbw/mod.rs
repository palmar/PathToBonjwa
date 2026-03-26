//! Safe Rust wrapper around the OpenBW C FFI.
//!
//! Provides [`Simulator`] — a RAII handle that owns a simulation instance
//! and exposes safe methods for loading replays, stepping frames, and
//! querying game state.

pub mod ffi;

use std::ffi::CStr;
use std::fmt;

/// Maximum number of units BW can have simultaneously.
const MAX_UNITS: usize = 1700;
/// Maximum player slots in a BW game.
const MAX_PLAYERS: usize = 8;

/// BW race identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Race {
    Zerg,
    Terran,
    Protoss,
}

impl Race {
    fn from_raw(r: u8) -> Self {
        match r {
            0 => Race::Zerg,
            1 => Race::Terran,
            2 => Race::Protoss,
            _ => Race::Terran,
        }
    }
}

impl fmt::Display for Race {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Race::Zerg => write!(f, "Zerg"),
            Race::Terran => write!(f, "Terran"),
            Race::Protoss => write!(f, "Protoss"),
        }
    }
}

/// A snapshot of a single unit at a point in time.
#[derive(Debug, Clone)]
pub struct Unit {
    pub unit_id: u16,
    pub player_id: u8,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub shields: i32,
    pub energy: i32,
    pub alive: bool,
}

impl From<ffi::ObwUnit> for Unit {
    fn from(u: ffi::ObwUnit) -> Self {
        Unit {
            unit_id: u.unit_id,
            player_id: u.player_id,
            x: u.x,
            y: u.y,
            hp: u.hp / 256,
            shields: u.shields / 256,
            energy: u.energy / 256,
            alive: u.is_alive != 0,
        }
    }
}

/// Resource and supply snapshot for one player.
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub player_id: u8,
    pub race: Race,
    pub minerals: i32,
    pub gas: i32,
    /// Supply used in display units (e.g. 4 workers = 4.0).
    pub supply_used: f64,
    /// Supply max in display units.
    pub supply_max: f64,
}

impl From<ffi::ObwPlayerInfo> for PlayerState {
    fn from(p: ffi::ObwPlayerInfo) -> Self {
        PlayerState {
            player_id: p.player_id,
            race: Race::from_raw(p.race),
            minerals: p.minerals,
            gas: p.gas,
            supply_used: p.supply_used as f64 / 2.0,
            supply_max: p.supply_max as f64 / 2.0,
        }
    }
}

/// Error type for simulation operations.
#[derive(Debug)]
pub struct SimError(pub String);

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpenBW error: {}", self.0)
    }
}

impl std::error::Error for SimError {}

/// Complete game state at a single frame.
#[derive(Debug, Clone)]
pub struct FrameState {
    pub frame: u32,
    pub players: Vec<PlayerState>,
    pub units: Vec<Unit>,
}

/// RAII wrapper around an OpenBW simulation instance.
///
/// # Safety
/// The inner pointer is managed exclusively by this struct.
/// `Drop` calls `obw_destroy_player`.
pub struct Simulator {
    ptr: *mut ffi::ObwPlayer,
}

// The C stub is single-threaded; mark Send but not Sync.
unsafe impl Send for Simulator {}

impl Simulator {
    /// Create a new simulation instance.
    pub fn new() -> Result<Self, SimError> {
        let ptr = unsafe { ffi::obw_create_player() };
        if ptr.is_null() {
            return Err(SimError("failed to create simulation instance".into()));
        }
        Ok(Simulator { ptr })
    }

    /// Load a replay from raw decompressed sections.
    ///
    /// - `header`: decompressed section 1 (633 bytes)
    /// - `commands`: decompressed section 2 (command stream)
    pub fn load_replay(&mut self, header: &[u8], commands: &[u8]) -> Result<(), SimError> {
        let rc = unsafe {
            ffi::obw_load_replay(
                self.ptr,
                header.as_ptr(),
                header.len(),
                commands.as_ptr(),
                commands.len(),
            )
        };
        if rc != 0 {
            return Err(self.last_error_or("load_replay failed"));
        }
        Ok(())
    }

    /// Advance the simulation by one frame.
    pub fn next_frame(&mut self) -> Result<(), SimError> {
        let rc = unsafe { ffi::obw_next_frame(self.ptr) };
        if rc != 0 {
            return Err(self.last_error_or("next_frame failed"));
        }
        Ok(())
    }

    /// Check if the replay simulation has finished.
    pub fn is_done(&self) -> bool {
        unsafe { ffi::obw_is_done(self.ptr) != 0 }
    }

    /// Current frame number.
    pub fn current_frame(&self) -> u32 {
        unsafe { ffi::obw_current_frame(self.ptr) }
    }

    /// Get all live units in the simulation.
    pub fn get_units(&self) -> Vec<Unit> {
        let mut buf = vec![
            ffi::ObwUnit {
                unit_id: 0,
                player_id: 0,
                x: 0,
                y: 0,
                hp: 0,
                shields: 0,
                energy: 0,
                is_alive: 0,
            };
            MAX_UNITS
        ];
        let n = unsafe { ffi::obw_get_units(self.ptr, buf.as_mut_ptr(), MAX_UNITS) };
        buf.truncate(n);
        buf.into_iter().map(Unit::from).collect()
    }

    /// Get resource/supply info for all active players.
    pub fn get_player_info(&self) -> Vec<PlayerState> {
        let mut buf = vec![
            ffi::ObwPlayerInfo {
                player_id: 0,
                minerals: 0,
                gas: 0,
                supply_used: 0,
                supply_max: 0,
                race: 0,
            };
            MAX_PLAYERS
        ];
        let n = unsafe { ffi::obw_get_player_info(self.ptr, buf.as_mut_ptr(), MAX_PLAYERS) };
        buf.truncate(n);
        buf.into_iter().map(PlayerState::from).collect()
    }

    /// Get supply for a specific player.
    pub fn get_supply(&self, player_id: u8) -> Result<(f64, f64), SimError> {
        let mut used: i32 = 0;
        let mut max: i32 = 0;
        let rc =
            unsafe { ffi::obw_get_supply(self.ptr, player_id, &mut used, &mut max) };
        if rc != 0 {
            return Err(SimError(format!("invalid player_id {}", player_id)));
        }
        Ok((used as f64 / 2.0, max as f64 / 2.0))
    }

    /// Capture a complete snapshot of the current frame.
    pub fn snapshot(&self) -> FrameState {
        FrameState {
            frame: self.current_frame(),
            players: self.get_player_info(),
            units: self.get_units(),
        }
    }

    fn last_error_or(&self, fallback: &str) -> SimError {
        let msg = unsafe {
            let ptr = ffi::obw_last_error(self.ptr);
            if ptr.is_null() {
                fallback.to_string()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        SimError(msg)
    }
}

impl Drop for Simulator {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::obw_destroy_player(self.ptr) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_destroy() {
        let sim = Simulator::new().expect("create failed");
        drop(sim);
    }

    #[test]
    fn unloaded_is_done() {
        let sim = Simulator::new().unwrap();
        assert!(sim.is_done());
    }
}
