//! Raw FFI bindings to the OpenBW C wrapper.
//!
//! These declarations mirror `openbw_wrapper.h`. Do not use directly —
//! use the safe wrapper in `super` instead.

use std::os::raw::c_char;

/// Opaque handle to a simulation instance.
#[repr(C)]
pub struct ObwPlayer {
    _opaque: [u8; 0],
}

/// Per-unit snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObwUnit {
    pub unit_id: u16,
    pub player_id: u8,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub shields: i32,
    pub energy: i32,
    pub is_alive: u8,
}

/// Per-player resource & supply snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObwPlayerInfo {
    pub player_id: u8,
    pub minerals: i32,
    pub gas: i32,
    pub supply_used: i32,
    pub supply_max: i32,
    pub race: u8,
}

extern "C" {
    pub fn obw_create_player() -> *mut ObwPlayer;

    pub fn obw_load_replay(
        p: *mut ObwPlayer,
        header_data: *const u8,
        header_len: usize,
        cmd_data: *const u8,
        cmd_len: usize,
    ) -> i32;

    pub fn obw_destroy_player(p: *mut ObwPlayer);

    pub fn obw_next_frame(p: *mut ObwPlayer) -> i32;

    pub fn obw_is_done(p: *const ObwPlayer) -> i32;

    pub fn obw_current_frame(p: *const ObwPlayer) -> u32;

    pub fn obw_get_units(
        p: *const ObwPlayer,
        out_units: *mut ObwUnit,
        max_units: usize,
    ) -> usize;

    pub fn obw_get_player_info(
        p: *const ObwPlayer,
        out_info: *mut ObwPlayerInfo,
        max_players: usize,
    ) -> usize;

    pub fn obw_get_supply(
        p: *const ObwPlayer,
        player_id: u8,
        used: *mut i32,
        max: *mut i32,
    ) -> i32;

    pub fn obw_last_error(p: *const ObwPlayer) -> *const c_char;
}
