/* openbw_wrapper.h — C-compatible wrapper around OpenBW's replay simulation API.
 *
 * This header defines the interface between PathToBonjwa (Rust) and OpenBW (C++).
 * The stub implementation (openbw_stub.c) provides a development/test shim;
 * the real implementation will link against OpenBW once the license clears.
 */

#ifndef OPENBW_WRAPPER_H
#define OPENBW_WRAPPER_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a simulation instance. */
typedef struct ObwPlayer ObwPlayer;

/* Per-unit snapshot returned by obw_get_units(). */
typedef struct {
    uint16_t unit_id;       /* BW unit type ID (e.g. 0x00 = Marine) */
    uint8_t  player_id;     /* Owning player slot (0-7) */
    int32_t  x;             /* Position in pixels */
    int32_t  y;
    int32_t  hp;            /* Current hit points (×256 internally in BW) */
    int32_t  shields;       /* Current shields (×256), 0 for non-Protoss */
    int32_t  energy;        /* Current energy (×256), 0 if N/A */
    uint8_t  is_alive;      /* 1 if alive, 0 if dead/removed */
} ObwUnit;

/* Per-player resource & supply snapshot. */
typedef struct {
    uint8_t  player_id;
    int32_t  minerals;
    int32_t  gas;
    int32_t  supply_used;   /* In half-supply units (BW internal) */
    int32_t  supply_max;    /* In half-supply units */
    uint8_t  race;          /* 0=Zerg, 1=Terran, 2=Protoss */
} ObwPlayerInfo;

/* ── Lifecycle ────────────────────────────────────────────────── */

/* Create a new simulation instance. Returns NULL on failure. */
ObwPlayer *obw_create_player(void);

/* Load a replay file into the simulation. The replay data is the raw
 * decompressed command stream (section 2 of a .rep file).
 * header_data/header_len: raw decompressed header (section 1, 633 bytes).
 * cmd_data/cmd_len: raw decompressed command section.
 * Returns 0 on success, non-zero on error. */
int obw_load_replay(ObwPlayer *p,
                    const uint8_t *header_data, size_t header_len,
                    const uint8_t *cmd_data, size_t cmd_len);

/* Destroy the simulation and free all resources. */
void obw_destroy_player(ObwPlayer *p);

/* ── Simulation stepping ──────────────────────────────────────── */

/* Advance the simulation by one frame. Returns 0 on success. */
int obw_next_frame(ObwPlayer *p);

/* Returns 1 if the replay has finished (no more frames), 0 otherwise. */
int obw_is_done(const ObwPlayer *p);

/* Returns the current frame number. */
uint32_t obw_current_frame(const ObwPlayer *p);

/* ── State queries ────────────────────────────────────────────── */

/* Write all live units into `out_units` (up to max_units).
 * Returns the number of units actually written.
 * If the buffer is too small, extra units are silently dropped. */
size_t obw_get_units(const ObwPlayer *p, ObwUnit *out_units, size_t max_units);

/* Write supply/resource info for all active players into `out_info`
 * (up to max_players, typically 8).
 * Returns the number of player records written. */
size_t obw_get_player_info(const ObwPlayer *p,
                           ObwPlayerInfo *out_info, size_t max_players);

/* Convenience: get supply for a single player.
 * Returns supply_used via *used and supply_max via *max (half-supply units).
 * Returns 0 on success, -1 if player_id is invalid. */
int obw_get_supply(const ObwPlayer *p, uint8_t player_id,
                   int32_t *used, int32_t *max);

/* ── Error handling ───────────────────────────────────────────── */

/* Returns a human-readable error string for the last failed operation,
 * or NULL if no error. The returned pointer is valid until the next
 * obw_* call on the same player. */
const char *obw_last_error(const ObwPlayer *p);

#ifdef __cplusplus
}
#endif

#endif /* OPENBW_WRAPPER_H */
