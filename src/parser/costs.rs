/// Brood War unit/building cost data: (minerals, gas, supply)
/// Supply is in half-units internally in BW; we store the display value (e.g. Zergling = 0.5).
/// For supply providers we store negative supply to indicate they ADD supply.

#[derive(Debug, Clone, Copy)]
pub struct UnitCost {
    pub minerals: u32,
    pub gas: u32,
    /// Supply consumed (positive) or provided (negative). Zerglings use 0.5 each.
    pub supply: f64,
}

/// Returns (minerals, gas, supply_cost) for a given unit_id.
/// Supply providers return negative supply (they add capacity).
/// Returns None for unknown units.
pub fn unit_cost(id: u16) -> Option<UnitCost> {
    let (m, g, s) = match id {
        // ─── Terran units ───────────────────────────────────────────
        0x00 => (50, 0, 1.0),    // Marine
        0x01 => (25, 75, 1.0),   // Ghost
        0x02 => (75, 0, 2.0),    // Vulture
        0x03 => (100, 50, 2.0),  // Goliath
        0x05 => (150, 100, 2.0), // Siege Tank
        0x07 => (50, 0, 1.0),    // SCV
        0x08 => (150, 100, 2.0), // Wraith
        0x09 => (100, 225, 2.0), // Science Vessel
        0x0B => (100, 100, 2.0), // Dropship
        0x0C => (400, 300, 6.0), // Battlecruiser
        0x0E => (200, 200, 8.0), // Nuclear Missile
        0x20 => (50, 25, 1.0),   // Firebat
        0x22 => (50, 25, 1.0),   // Medic
        0x3A => (250, 125, 3.0), // Valkyrie

        // ─── Zerg units ─────────────────────────────────────────────
        0x25 => (25, 0, 0.5),    // Zergling (pair = 50/0/1, but per unit 0.5)
        0x26 => (75, 25, 1.0),   // Hydralisk
        0x27 => (200, 200, 4.0), // Ultralisk
        0x29 => (50, 0, 1.0),    // Drone
        0x2A => (100, 0, -8.0),  // Overlord (supply provider)
        0x2B => (100, 100, 2.0), // Mutalisk
        0x2C => (0, 0, 0.0),     // Guardian (morph from muta, cost handled by morph)
        0x2D => (100, 100, 2.0), // Queen
        0x2E => (50, 150, 2.0),  // Defiler
        0x2F => (12, 38, 0.5),   // Scourge (pair = 25/75/1)
        0x32 => (100, 50, 1.0),  // Infested Terran
        0x3C => (150, 100, 2.0), // Corsair
        0x3E => (0, 0, 0.0),     // Devourer (morph from muta)
        0x67 => (0, 0, 0.0),     // Lurker (morph from hydra)
        0x61 => (50, 100, 0.0),  // Lurker Egg (morph cost)

        // ─── Protoss units ──────────────────────────────────────────
        0x3D => (125, 100, 2.0), // Dark Templar
        0x3F => (0, 0, 0.0),     // Dark Archon (merge cost 0, from 2 DTs)
        0x40 => (50, 0, 1.0),    // Probe
        0x41 => (100, 0, 2.0),   // Zealot
        0x42 => (125, 50, 2.0),  // Dragoon
        0x43 => (50, 150, 2.0),  // High Templar
        0x44 => (0, 0, 0.0),     // Archon (merge cost 0, from 2 HTs)
        0x45 => (200, 0, 2.0),   // Shuttle
        0x46 => (275, 125, 3.0), // Scout
        0x47 => (100, 350, 4.0), // Arbiter
        0x48 => (350, 250, 6.0), // Carrier
        0x49 => (25, 0, 0.0),    // Interceptor
        0x53 => (200, 100, 4.0), // Reaver
        0x54 => (25, 75, 1.0),   // Observer
        0x55 => (15, 0, 0.0),    // Scarab

        // ─── Terran buildings ───────────────────────────────────────
        0x6A => (400, 0, -10.0), // Command Center (supply provider)
        0x6B => (50, 50, 0.0),   // ComSat
        0x6C => (100, 100, 0.0), // Nuclear Silo
        0x6D => (100, 0, -8.0),  // Supply Depot (supply provider)
        0x6E => (75, 0, 0.0),    // Refinery
        0x6F => (150, 0, 0.0),   // Barracks
        0x70 => (150, 0, 0.0),   // Academy
        0x71 => (200, 100, 0.0), // Factory
        0x72 => (150, 100, 0.0), // Starport
        0x73 => (50, 50, 0.0),   // Control Tower
        0x74 => (100, 150, 0.0), // Science Facility
        0x75 => (50, 50, 0.0),   // Covert Ops
        0x76 => (50, 50, 0.0),   // Physics Lab
        0x78 => (50, 50, 0.0),   // Machine Shop
        0x7A => (125, 0, 0.0),   // Engineering Bay
        0x7B => (100, 50, 0.0),  // Armory
        0x7C => (75, 0, 0.0),    // Missile Turret
        0x7D => (100, 0, 0.0),   // Bunker

        // ─── Zerg buildings ─────────────────────────────────────────
        0x82 => (0, 0, 0.0),     // Infested CC
        0x83 => (300, 0, -1.0),  // Hatchery (1 larva supply provider)
        0x84 => (150, 100, 0.0), // Lair (morph)
        0x85 => (200, 150, 0.0), // Hive (morph)
        0x86 => (160, 0, 0.0),   // Nydus Canal
        0x87 => (100, 50, 0.0),  // Hydralisk Den
        0x88 => (100, 100, 0.0), // Defiler Mound
        0x89 => (100, 150, 0.0), // Greater Spire (morph)
        0x8A => (150, 100, 0.0), // Queens Nest
        0x8B => (75, 0, 0.0),    // Evolution Chamber
        0x8C => (150, 200, 0.0), // Ultralisk Cavern
        0x8D => (200, 150, 0.0), // Spire
        0x8E => (200, 0, 0.0),   // Spawning Pool
        0x8F => (75, 0, 0.0),    // Creep Colony
        0x90 => (50, 0, 0.0),    // Spore Colony (morph)
        0x92 => (50, 0, 0.0),    // Sunken Colony (morph)
        0x95 => (25, 0, 0.0),    // Extractor

        // ─── Protoss buildings ──────────────────────────────────────
        0x9A => (400, 0, -9.0),  // Nexus (supply provider)
        0x9B => (200, 200, 0.0), // Robotics Facility
        0x9C => (100, 0, -8.0),  // Pylon (supply provider)
        0x9D => (100, 0, 0.0),   // Assimilator
        0x9F => (50, 100, 0.0),  // Observatory
        0xA0 => (150, 0, 0.0),   // Gateway
        0xA2 => (150, 0, 0.0),   // Photon Cannon
        0xA3 => (150, 100, 0.0), // Citadel of Adun
        0xA4 => (200, 0, 0.0),   // Cybernetics Core
        0xA5 => (150, 200, 0.0), // Templar Archives
        0xA6 => (150, 0, 0.0),   // Forge
        0xA7 => (150, 150, 0.0), // Stargate
        0xA9 => (300, 200, 0.0), // Fleet Beacon
        0xAA => (200, 150, 0.0), // Arbiter Tribunal
        0xAB => (150, 100, 0.0), // Robotics Support Bay
        0xAC => (100, 0, 0.0),   // Shield Battery

        _ => return None,
    };
    Some(UnitCost {
        minerals: m,
        gas: g,
        supply: s,
    })
}

/// Returns the starting supply for a race (workers + base).
/// Terran: 4 SCVs + CC = 4 used / 10 max
/// Zerg: 4 Drones + Overlord + Hatchery = 4 used / 9 max
/// Protoss: 4 Probes + Nexus = 4 used / 9 max
pub fn starting_supply(race: &super::Race) -> (f64, f64) {
    match race {
        super::Race::Terran => (4.0, 10.0),
        super::Race::Zerg => (4.0, 9.0),
        super::Race::Protoss => (4.0, 9.0),
        _ => (4.0, 10.0),
    }
}

/// Upgrade costs: (minerals, gas) — level 1 costs only for simplicity
pub fn upgrade_cost(upgrade_id: u8) -> (u32, u32) {
    match upgrade_id {
        0 => (100, 100),  // Terran Infantry Armor
        1 => (100, 100),  // Terran Vehicle Plating
        2 => (150, 150),  // Terran Ship Plating
        3 => (150, 150),  // Zerg Carapace
        4 => (150, 150),  // Zerg Flyer Carapace
        5 => (100, 100),  // Protoss Ground Armor
        6 => (150, 150),  // Protoss Air Armor
        7 => (100, 100),  // Terran Infantry Weapons
        8 => (100, 100),  // Terran Vehicle Weapons
        9 => (100, 100),  // Terran Ship Weapons
        10 => (100, 100), // Zerg Melee Attacks
        11 => (100, 100), // Zerg Missile Attacks
        12 => (100, 100), // Zerg Flyer Attacks
        13 => (100, 100), // Protoss Ground Weapons
        14 => (100, 100), // Protoss Air Weapons
        15 => (200, 200), // Protoss Plasma Shields
        _ => (150, 150),  // Default estimate
    }
}

/// Tech research costs: (minerals, gas)
pub fn tech_cost(tech_id: u8) -> (u32, u32) {
    match tech_id {
        0 => (200, 200),  // Stim Packs
        1 => (100, 100),  // Lockdown
        2 => (200, 200),  // EMP Shockwave
        3 => (100, 100),  // Spider Mines
        5 => (150, 150),  // Siege Mode
        7 => (200, 200),  // Irradiate
        9 => (100, 100),  // Yamato Gun
        10 => (100, 100), // Cloaking Field
        11 => (25, 75),   // Personnel Cloaking
        13 => (100, 100), // Burrowing
        15 => (200, 200), // Spawn Broodlings
        17 => (100, 100), // Plague
        19 => (100, 100), // Consume
        20 => (150, 150), // Ensnare
        22 => (150, 100), // Psionic Storm
        24 => (150, 150), // Hallucination
        25 => (200, 200), // Recall
        27 => (100, 100), // Stasis Field
        30 => (200, 200), // Restoration
        32 => (100, 100), // Disruption Web
        34 => (200, 200), // Mind Control
        36 => (100, 100), // Feedback
        38 => (150, 150), // Optical Flare
        39 => (200, 200), // Maelstrom
        40 => (100, 100), // Lurker Aspect
        _ => (150, 150),  // Default estimate
    }
}
