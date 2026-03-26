/* openbw_stub.c — Development stub for the OpenBW wrapper.
 *
 * This provides a minimal simulation that:
 * - Tracks frame count and replay length
 * - Parses enough of the header to report player info
 * - Maintains a simple unit list (starting workers + buildings)
 * - Advances frames and reports completion
 *
 * Replace this file with the real OpenBW-backed implementation
 * once the license audit (Phase 2a) clears.
 */

#include "openbw_wrapper.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define MAX_PLAYERS 8
#define MAX_UNITS   1700  /* BW hard limit */

struct ObwPlayer {
    /* Replay metadata */
    uint32_t total_frames;
    uint32_t current_frame;
    int      loaded;

    /* Player state */
    uint8_t  num_players;
    ObwPlayerInfo players[MAX_PLAYERS];

    /* Unit state (simplified) */
    ObwUnit  units[MAX_UNITS];
    size_t   num_units;

    /* Error buffer */
    char     error_buf[256];
    int      has_error;
};

/* ── Helpers ──────────────────────────────────────────────────── */

static uint32_t read_u32_le(const uint8_t *p) {
    return (uint32_t)p[0]
         | ((uint32_t)p[1] << 8)
         | ((uint32_t)p[2] << 16)
         | ((uint32_t)p[3] << 24);
}

static uint16_t read_u16_le(const uint8_t *p) {
    return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}

static void set_error(ObwPlayer *p, const char *msg) {
    snprintf(p->error_buf, sizeof(p->error_buf), "%s", msg);
    p->has_error = 1;
}

/* Map race byte to our enum: 0=Zerg, 1=Terran, 2=Protoss */
static uint8_t parse_race(uint8_t r) {
    switch (r) {
        case 0: return 0; /* Zerg */
        case 1: return 1; /* Terran */
        case 2: return 2; /* Protoss */
        default: return 1; /* fallback */
    }
}

/* Add starting units for a player based on race (stub approximation). */
static void add_starting_units(ObwPlayer *sim, uint8_t pid, uint8_t race) {
    /* Base building */
    uint16_t base_id;
    uint16_t worker_id;
    int32_t  base_supply_max;

    switch (race) {
        case 0: /* Zerg */
            base_id = 0x83;  /* Hatchery */
            worker_id = 0x29; /* Drone */
            base_supply_max = 18; /* 9 supply × 2 (half-units) */
            break;
        case 2: /* Protoss */
            base_id = 0x9A;  /* Nexus */
            worker_id = 0x40; /* Probe */
            base_supply_max = 18;
            break;
        default: /* Terran */
            base_id = 0x6A;  /* Command Center */
            worker_id = 0x07; /* SCV */
            base_supply_max = 20;
            break;
    }

    /* Add base */
    if (sim->num_units < MAX_UNITS) {
        ObwUnit *u = &sim->units[sim->num_units++];
        u->unit_id = base_id;
        u->player_id = pid;
        u->x = 100 + pid * 200;
        u->y = 100 + pid * 200;
        u->hp = 1500 * 256;
        u->shields = (race == 2) ? 500 * 256 : 0;
        u->energy = 0;
        u->is_alive = 1;
    }

    /* Add 4 workers */
    for (int i = 0; i < 4; i++) {
        if (sim->num_units < MAX_UNITS) {
            ObwUnit *u = &sim->units[sim->num_units++];
            u->unit_id = worker_id;
            u->player_id = pid;
            u->x = 120 + pid * 200 + i * 16;
            u->y = 120 + pid * 200;
            u->hp = 60 * 256;
            u->shields = (race == 2) ? 20 * 256 : 0;
            u->energy = 0;
            u->is_alive = 1;
        }
    }

    /* Zerg gets an Overlord */
    if (race == 0 && sim->num_units < MAX_UNITS) {
        ObwUnit *u = &sim->units[sim->num_units++];
        u->unit_id = 0x2A; /* Overlord */
        u->player_id = pid;
        u->x = 140 + pid * 200;
        u->y = 80 + pid * 200;
        u->hp = 200 * 256;
        u->shields = 0;
        u->energy = 0;
        u->is_alive = 1;
    }

    /* Set player supply */
    sim->players[pid].supply_used = 8; /* 4 workers × 2 half-units */
    sim->players[pid].supply_max = base_supply_max;
}

/* ── Public API ───────────────────────────────────────────────── */

ObwPlayer *obw_create_player(void) {
    ObwPlayer *p = (ObwPlayer *)calloc(1, sizeof(ObwPlayer));
    return p;
}

int obw_load_replay(ObwPlayer *p,
                    const uint8_t *header_data, size_t header_len,
                    const uint8_t *cmd_data, size_t cmd_len) {
    if (!p) return -1;
    p->has_error = 0;

    if (header_len < 0x279) {
        set_error(p, "header too short (need 633 bytes)");
        return -1;
    }

    /* Parse frame count from header offset 0x01 */
    p->total_frames = read_u32_le(header_data + 0x01);
    p->current_frame = 0;
    p->num_units = 0;
    p->num_players = 0;

    /* Parse player slots from header offset 0xA1 (12 slots × 36 bytes) */
    for (int i = 0; i < 12 && p->num_players < MAX_PLAYERS; i++) {
        size_t off = 0xA1 + (size_t)i * 36;
        if (off + 36 > header_len) break;

        uint8_t player_type = header_data[off + 8];
        /* player_type: 1 = Computer, 2 = Human */
        if (player_type != 1 && player_type != 2) continue;

        uint8_t pid = p->num_players;
        uint8_t race = parse_race(header_data[off + 9]);

        p->players[pid].player_id = (uint8_t)i;
        p->players[pid].minerals = 50;
        p->players[pid].gas = 0;
        p->players[pid].race = race;

        add_starting_units(p, pid, race);
        p->num_players++;
    }

    p->loaded = 1;
    (void)cmd_data;
    (void)cmd_len;
    return 0;
}

void obw_destroy_player(ObwPlayer *p) {
    free(p);
}

int obw_next_frame(ObwPlayer *p) {
    if (!p || !p->loaded) return -1;
    if (p->current_frame >= p->total_frames) return -1;
    p->current_frame++;
    return 0;
}

int obw_is_done(const ObwPlayer *p) {
    if (!p || !p->loaded) return 1;
    return p->current_frame >= p->total_frames ? 1 : 0;
}

uint32_t obw_current_frame(const ObwPlayer *p) {
    if (!p) return 0;
    return p->current_frame;
}

size_t obw_get_units(const ObwPlayer *p, ObwUnit *out_units, size_t max_units) {
    if (!p || !p->loaded || !out_units) return 0;
    size_t n = p->num_units < max_units ? p->num_units : max_units;
    memcpy(out_units, p->units, n * sizeof(ObwUnit));
    return n;
}

size_t obw_get_player_info(const ObwPlayer *p,
                           ObwPlayerInfo *out_info, size_t max_players) {
    if (!p || !p->loaded || !out_info) return 0;
    size_t n = p->num_players < max_players ? p->num_players : max_players;
    memcpy(out_info, p->players, n * sizeof(ObwPlayerInfo));
    return n;
}

int obw_get_supply(const ObwPlayer *p, uint8_t player_id,
                   int32_t *used, int32_t *max) {
    if (!p || !p->loaded) return -1;
    for (size_t i = 0; i < p->num_players; i++) {
        if (p->players[i].player_id == player_id) {
            if (used) *used = p->players[i].supply_used;
            if (max)  *max  = p->players[i].supply_max;
            return 0;
        }
    }
    return -1;
}

const char *obw_last_error(const ObwPlayer *p) {
    if (!p || !p->has_error) return NULL;
    return p->error_buf;
}
