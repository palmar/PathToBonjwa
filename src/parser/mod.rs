use std::io::Read;

// ─── Constants ───────────────────────────────────────────────────────────────

const FRAMES_PER_SECOND: f64 = 23.81;
const HEADER_SIZE: usize = 0x279;

const BW_COLORS: [PlayerColor; 8] = [
    PlayerColor { r: 244, g: 4, b: 4 },
    PlayerColor { r: 12, g: 72, b: 204 },
    PlayerColor { r: 44, g: 180, b: 148 },
    PlayerColor { r: 136, g: 64, b: 156 },
    PlayerColor { r: 248, g: 140, b: 20 },
    PlayerColor { r: 112, g: 48, b: 20 },
    PlayerColor { r: 204, g: 224, b: 208 },
    PlayerColor { r: 252, g: 252, b: 56 },
];

// ─── Replay struct ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Replay {
    pub engine: Engine,
    pub frames: u32,
    pub duration_secs: f64,
    pub timestamp: i64,
    pub title: String,
    pub map_width: u16,
    pub map_height: u16,
    pub map_name: String,
    pub host_name: String,
    pub game_speed: GameSpeed,
    pub game_type: GameType,
    pub players: Vec<Player>,
    pub matchup: String,
    pub commands: Vec<Command>,
}

// ─── Player ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Player {
    pub slot_id: u16,
    pub player_id: u8,
    pub player_type: PlayerType,
    pub race: Race,
    pub team: u8,
    pub name: String,
    pub color: PlayerColor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Race {
    Zerg,
    Terran,
    Protoss,
    Unknown(u8),
}

impl Race {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => Race::Zerg,
            1 => Race::Terran,
            2 => Race::Protoss,
            _ => Race::Unknown(b),
        }
    }

    pub fn short(&self) -> &str {
        match self {
            Race::Zerg => "Z",
            Race::Terran => "T",
            Race::Protoss => "P",
            Race::Unknown(_) => "?",
        }
    }
}

impl std::fmt::Display for Race {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Race::Zerg => write!(f, "Zerg"),
            Race::Terran => write!(f, "Terran"),
            Race::Protoss => write!(f, "Protoss"),
            Race::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerType {
    Inactive,
    Computer,
    Human,
    RescuePassive,
    Unknown(u8),
}

impl PlayerType {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => PlayerType::Inactive,
            1 | 5 | 7 => PlayerType::Computer,
            2 | 6 => PlayerType::Human,
            3 => PlayerType::RescuePassive,
            _ => PlayerType::Unknown(b),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, PlayerType::Human | PlayerType::Computer)
    }
}

impl std::fmt::Display for PlayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerType::Human => write!(f, "Human"),
            PlayerType::Computer => write!(f, "Computer"),
            PlayerType::Inactive => write!(f, "Inactive"),
            PlayerType::RescuePassive => write!(f, "Rescue"),
            PlayerType::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

// ─── Enums ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum Engine {
    StarCraft,
    BroodWar,
    Unknown(u8),
}

impl Engine {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => Engine::StarCraft,
            1 => Engine::BroodWar,
            _ => Engine::Unknown(b),
        }
    }
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine::StarCraft => write!(f, "StarCraft"),
            Engine::BroodWar => write!(f, "Brood War"),
            Engine::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameSpeed {
    Slowest, Slower, Slow, Normal, Fast, Faster, Fastest, Unknown(u8),
}

impl GameSpeed {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => GameSpeed::Slowest, 1 => GameSpeed::Slower,
            2 => GameSpeed::Slow, 3 => GameSpeed::Normal,
            4 => GameSpeed::Fast, 5 => GameSpeed::Faster,
            6 => GameSpeed::Fastest, _ => GameSpeed::Unknown(b),
        }
    }
}

impl std::fmt::Display for GameSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameSpeed::Slowest => write!(f, "Slowest"),
            GameSpeed::Slower => write!(f, "Slower"),
            GameSpeed::Slow => write!(f, "Slow"),
            GameSpeed::Normal => write!(f, "Normal"),
            GameSpeed::Fast => write!(f, "Fast"),
            GameSpeed::Faster => write!(f, "Faster"),
            GameSpeed::Fastest => write!(f, "Fastest"),
            GameSpeed::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameType {
    Melee, FreeForAll, OneOnOne, CaptureTheFlag, Greed, Slaughter,
    SuddenDeath, Ladder, UseMapSettings, TeamMelee, TeamFreeForAll,
    TeamCaptureTheFlag, TopVsBottom, Unknown(u16),
}

impl GameType {
    fn from_u16(v: u16) -> Self {
        match v {
            0x02 => GameType::Melee, 0x03 => GameType::FreeForAll,
            0x04 => GameType::OneOnOne, 0x05 => GameType::CaptureTheFlag,
            0x06 => GameType::Greed, 0x07 => GameType::Slaughter,
            0x08 => GameType::SuddenDeath, 0x09 => GameType::Ladder,
            0x0A => GameType::UseMapSettings, 0x0B => GameType::TeamMelee,
            0x0C => GameType::TeamFreeForAll, 0x0D => GameType::TeamCaptureTheFlag,
            0x0F => GameType::TopVsBottom, _ => GameType::Unknown(v),
        }
    }
}

impl std::fmt::Display for GameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameType::Melee => write!(f, "Melee"),
            GameType::FreeForAll => write!(f, "Free For All"),
            GameType::OneOnOne => write!(f, "1on1"),
            GameType::CaptureTheFlag => write!(f, "Capture The Flag"),
            GameType::Greed => write!(f, "Greed"),
            GameType::Slaughter => write!(f, "Slaughter"),
            GameType::SuddenDeath => write!(f, "Sudden Death"),
            GameType::Ladder => write!(f, "Ladder"),
            GameType::UseMapSettings => write!(f, "Use Map Settings"),
            GameType::TeamMelee => write!(f, "Team Melee"),
            GameType::TeamFreeForAll => write!(f, "Team FFA"),
            GameType::TeamCaptureTheFlag => write!(f, "Team CTF"),
            GameType::TopVsBottom => write!(f, "Top vs Bottom"),
            GameType::Unknown(v) => write!(f, "Unknown(0x{:02X})", v),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl PlayerColor {
    pub fn to_egui(&self) -> eframe::egui::Color32 {
        eframe::egui::Color32::from_rgb(self.r, self.g, self.b)
    }
}

// ─── Commands ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Command {
    pub frame: u32,
    pub player_id: u8,
    pub cmd: CmdType,
}

#[derive(Debug, Clone)]
pub enum CmdType {
    Select { count: u8 },
    Hotkey { hotkey_type: HotkeyAction, group: u8 },
    Build { unit_id: u16, x: u16, y: u16 },
    Train { unit_id: u16 },
    UnitMorph { unit_id: u16 },
    BuildingMorph { unit_id: u16 },
    RightClick { x: u16, y: u16, queued: bool },
    TargetedOrder { x: u16, y: u16, order_id: u8, queued: bool },
    Stop,
    HoldPosition,
    ReturnCargo,
    Burrow,
    Unburrow,
    Siege,
    Unsiege,
    Cloak,
    Decloak,
    LiftOff { x: u16, y: u16 },
    UnloadAll,
    Unload,
    MergeArchon,
    MergeDarkArchon,
    TrainFighter,
    CancelBuild,
    CancelTrain,
    CancelMorph,
    CancelTech,
    CancelUpgrade,
    CancelAddon,
    CancelNuke,
    Tech { tech_id: u8 },
    Upgrade { upgrade_id: u8 },
    Stim,
    LeaveGame { reason: u8 },
    MinimapPing { x: u16, y: u16 },
    Chat { message: String },
    KeepAlive,
    Other { type_id: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotkeyAction {
    Assign,
    Select,
    Add,
    Unknown(u8),
}

impl HotkeyAction {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => HotkeyAction::Assign,
            1 => HotkeyAction::Select,
            2 => HotkeyAction::Add,
            _ => HotkeyAction::Unknown(b),
        }
    }
}

impl std::fmt::Display for HotkeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HotkeyAction::Assign => write!(f, "Assign"),
            HotkeyAction::Select => write!(f, "Select"),
            HotkeyAction::Add => write!(f, "Add"),
            HotkeyAction::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

// ─── Unit names lookup ───────────────────────────────────────────────────────

pub fn unit_name(id: u16) -> &'static str {
    match id {
        0x00 => "Marine", 0x01 => "Ghost", 0x02 => "Vulture",
        0x03 => "Goliath", 0x05 => "Siege Tank", 0x07 => "SCV",
        0x08 => "Wraith", 0x09 => "Science Vessel", 0x0B => "Dropship",
        0x0C => "Battlecruiser", 0x0E => "Nuclear Missile",
        0x1E => "Siege Tank (Siege)", 0x20 => "Firebat",
        0x22 => "Medic", 0x23 => "Larva", 0x24 => "Egg",
        0x25 => "Zergling", 0x26 => "Hydralisk", 0x27 => "Ultralisk",
        0x29 => "Drone", 0x2A => "Overlord", 0x2B => "Mutalisk",
        0x2C => "Guardian", 0x2D => "Queen", 0x2E => "Defiler",
        0x2F => "Scourge", 0x32 => "Infested Terran",
        0x3A => "Valkyrie", 0x3C => "Corsair",
        0x3D => "Dark Templar", 0x3E => "Devourer",
        0x3F => "Dark Archon", 0x40 => "Probe", 0x41 => "Zealot",
        0x42 => "Dragoon", 0x43 => "High Templar", 0x44 => "Archon",
        0x45 => "Shuttle", 0x46 => "Scout", 0x47 => "Arbiter",
        0x48 => "Carrier", 0x49 => "Interceptor",
        0x53 => "Reaver", 0x54 => "Observer", 0x55 => "Scarab",
        0x61 => "Lurker Egg", 0x67 => "Lurker",
        0x6A => "Command Center", 0x6B => "ComSat",
        0x6C => "Nuclear Silo", 0x6D => "Supply Depot",
        0x6E => "Refinery", 0x6F => "Barracks",
        0x70 => "Academy", 0x71 => "Factory",
        0x72 => "Starport", 0x73 => "Control Tower",
        0x74 => "Science Facility", 0x75 => "Covert Ops",
        0x76 => "Physics Lab", 0x78 => "Machine Shop",
        0x7A => "Engineering Bay", 0x7B => "Armory",
        0x7C => "Missile Turret", 0x7D => "Bunker",
        0x82 => "Infested CC", 0x83 => "Hatchery",
        0x84 => "Lair", 0x85 => "Hive",
        0x86 => "Nydus Canal", 0x87 => "Hydralisk Den",
        0x88 => "Defiler Mound", 0x89 => "Greater Spire",
        0x8A => "Queens Nest", 0x8B => "Evolution Chamber",
        0x8C => "Ultralisk Cavern", 0x8D => "Spire",
        0x8E => "Spawning Pool", 0x8F => "Creep Colony",
        0x90 => "Spore Colony", 0x92 => "Sunken Colony",
        0x95 => "Extractor", 0x9A => "Nexus",
        0x9B => "Robotics Facility", 0x9C => "Pylon",
        0x9D => "Assimilator", 0x9F => "Observatory",
        0xA0 => "Gateway", 0xA2 => "Photon Cannon",
        0xA3 => "Citadel of Adun", 0xA4 => "Cybernetics Core",
        0xA5 => "Templar Archives", 0xA6 => "Forge",
        0xA7 => "Stargate", 0xA9 => "Fleet Beacon",
        0xAA => "Arbiter Tribunal", 0xAB => "Robotics Support Bay",
        0xAC => "Shield Battery",
        _ => "Unknown",
    }
}

pub fn is_building(id: u16) -> bool {
    matches!(id,
        0x6A..=0x7D | 0x82..=0x95 | 0x9A..=0xAC
    )
}

// ─── Section reader ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum RepFormat {
    Legacy,
    Modern,
    Modern121,
}

fn detect_format(data: &[u8]) -> Result<RepFormat, String> {
    if data.len() < 30 {
        return Err("File too small to detect format".into());
    }
    // byte 12 is the first byte of the Replay ID section data
    if data[12] == b's' {
        Ok(RepFormat::Modern121)
    } else if data[28] != 0x78 {
        Ok(RepFormat::Legacy)
    } else {
        Ok(RepFormat::Modern)
    }
}

struct SectionReader<'a> {
    data: &'a [u8],
    pos: usize,
    format: RepFormat,
    sections_read: usize,
}

impl<'a> SectionReader<'a> {
    fn new(data: &'a [u8]) -> Result<Self, String> {
        let format = detect_format(data)?;
        Ok(Self { data, pos: 0, format, sections_read: 0 })
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_u32_raw(&mut self) -> Result<u32, String> {
        if self.remaining() < 4 {
            return Err("Unexpected end of data reading u32".into());
        }
        let val = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(val)
    }

    fn read_section_modern(&mut self, expected_size: usize) -> Result<Vec<u8>, String> {
        // checksum (4 bytes) - skip
        let _checksum = self.read_u32_raw()?;
        // chunk count (4 bytes)
        let chunk_count = self.read_u32_raw()?;

        let mut result = Vec::with_capacity(expected_size);
        for _ in 0..chunk_count {
            let compressed_len = self.read_u32_raw()? as usize;
            if self.remaining() < compressed_len {
                return Err("Truncated chunk data".into());
            }
            let chunk_data = &self.data[self.pos..self.pos + compressed_len];
            self.pos += compressed_len;

            if compressed_len > 4 && chunk_data[0] == 0x78 {
                let decompressed = zlib_decompress(chunk_data)?;
                result.extend_from_slice(&decompressed);
            } else {
                result.extend_from_slice(chunk_data);
            }
        }
        Ok(result)
    }

    fn read_section(&mut self, fixed_size: Option<usize>) -> Result<Vec<u8>, String> {
        self.sections_read += 1;

        // For 1.21, extra 4 bytes between sections 1 and 2
        if self.format == RepFormat::Modern121 && self.sections_read == 3 {
            if self.remaining() >= 4 {
                self.pos += 4;
            }
        }

        match self.format {
            RepFormat::Modern | RepFormat::Modern121 => {
                let size = match fixed_size {
                    Some(s) => s,
                    None => {
                        // Read size from a mini-section (4 bytes decompressed)
                        let size_data = self.read_section_modern(4)?;
                        if size_data.len() < 4 {
                            return Err("Failed to read section size".into());
                        }
                        u32::from_le_bytes([size_data[0], size_data[1], size_data[2], size_data[3]])
                            as usize
                    }
                };
                self.read_section_modern(size)
            }
            RepFormat::Legacy => {
                self.read_section_legacy(fixed_size)
            }
        }
    }

    fn read_section_legacy(&mut self, fixed_size: Option<usize>) -> Result<Vec<u8>, String> {
        let size = match fixed_size {
            Some(s) => s,
            None => {
                // For variable sections, read the size first
                let _checksum = self.read_u32_raw()?;
                let chunk_count = self.read_u32_raw()?;
                // Read the size section chunks
                let mut size_buf = Vec::new();
                for _ in 0..chunk_count {
                    let len = self.read_u32_raw()? as usize;
                    if self.remaining() < len {
                        return Err("Truncated legacy section".into());
                    }
                    size_buf.extend_from_slice(&self.data[self.pos..self.pos + len]);
                    self.pos += len;
                }
                if size_buf.len() >= 4 {
                    u32::from_le_bytes([size_buf[0], size_buf[1], size_buf[2], size_buf[3]]) as usize
                } else {
                    return Err("Failed to read legacy section size".into());
                }
            }
        };

        // Now read the actual section
        let _checksum = self.read_u32_raw()?;
        let chunk_count = self.read_u32_raw()?;

        let mut result = Vec::with_capacity(size);
        for _ in 0..chunk_count {
            let compressed_len = self.read_u32_raw()? as usize;
            if self.remaining() < compressed_len {
                return Err("Truncated legacy chunk".into());
            }
            let chunk = &self.data[self.pos..self.pos + compressed_len];
            self.pos += compressed_len;

            // Legacy uses PKWARE compression, but some chunks may be uncompressed
            // if compressed_len == expected output size for this chunk.
            // Try zlib first, fall back to raw
            if compressed_len > 2 && chunk[0] == 0x78 {
                match zlib_decompress(chunk) {
                    Ok(decompressed) => result.extend_from_slice(&decompressed),
                    Err(_) => result.extend_from_slice(chunk),
                }
            } else {
                // PKWARE or raw - for now treat as raw
                result.extend_from_slice(chunk);
            }
        }
        Ok(result)
    }
}

// ─── Parsing ─────────────────────────────────────────────────────────────────

fn read_null_terminated(data: &[u8], max_len: usize) -> String {
    let len = max_len.min(data.len());
    let end = data.iter().take(len).position(|&b| b == 0).unwrap_or(len);
    String::from_utf8_lossy(&data[..end]).to_string()
}

fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| format!("zlib decompress failed: {}", e))?;
    Ok(out)
}

pub fn parse_replay(data: &[u8]) -> Result<Replay, String> {
    let mut reader = SectionReader::new(data)?;

    // Section 0: Replay ID (4 bytes)
    let _replay_id = reader.read_section(Some(4))?;

    // Section 1: Header (0x279 bytes)
    let header_data = reader.read_section(Some(HEADER_SIZE))?;
    if header_data.len() < HEADER_SIZE {
        return Err(format!(
            "Header too small: {} bytes (need {})",
            header_data.len(),
            HEADER_SIZE
        ));
    }

    let mut replay = parse_header(&header_data)?;

    // Section 2: Commands (variable size)
    match reader.read_section(None) {
        Ok(cmd_data) => {
            replay.commands = parse_commands(&cmd_data);
        }
        Err(e) => {
            eprintln!("Warning: failed to parse commands section: {}", e);
            replay.commands = Vec::new();
        }
    }

    Ok(replay)
}

fn parse_header(h: &[u8]) -> Result<Replay, String> {
    let engine = Engine::from_byte(h[0x00]);
    let frames = u32::from_le_bytes([h[0x01], h[0x02], h[0x03], h[0x04]]);
    let duration_secs = frames as f64 / FRAMES_PER_SECOND;
    let timestamp = i32::from_le_bytes([h[0x08], h[0x09], h[0x0A], h[0x0B]]) as i64;

    let title = read_null_terminated(&h[0x18..], 28);
    let map_width = u16::from_le_bytes([h[0x34], h[0x35]]);
    let map_height = u16::from_le_bytes([h[0x36], h[0x37]]);
    let game_speed = GameSpeed::from_byte(h[0x3A]);
    let game_type = GameType::from_u16(u16::from_le_bytes([h[0x3C], h[0x3D]]));
    let host_name = read_null_terminated(&h[0x48..], 24);
    let map_name = read_null_terminated(&h[0x61..], 26);

    let player_data = &h[0xA1..];
    let color_data = &h[0x251..];

    let mut players = Vec::new();
    for i in 0..12 {
        let base = i * 36;
        if base + 36 > player_data.len() {
            break;
        }
        let ps = &player_data[base..base + 36];

        let slot_id = u16::from_le_bytes([ps[0], ps[1]]);
        let player_id = ps[4];
        let player_type = PlayerType::from_byte(ps[8]);
        let race = Race::from_byte(ps[9]);
        let team = ps[10];
        let name = read_null_terminated(&ps[11..], 25);

        if !player_type.is_active() {
            continue;
        }

        let color = if i < 8 {
            let cb = i * 4;
            if cb + 3 < color_data.len() {
                let color_id =
                    u32::from_le_bytes([color_data[cb], color_data[cb + 1], color_data[cb + 2], color_data[cb + 3]])
                        as usize;
                if color_id < BW_COLORS.len() {
                    BW_COLORS[color_id]
                } else {
                    BW_COLORS[i % BW_COLORS.len()]
                }
            } else {
                BW_COLORS[i % BW_COLORS.len()]
            }
        } else {
            BW_COLORS[0]
        };

        players.push(Player {
            slot_id,
            player_id,
            player_type,
            race,
            team,
            name,
            color,
        });
    }

    let matchup = build_matchup(&players);

    Ok(Replay {
        engine,
        frames,
        duration_secs,
        timestamp,
        title,
        map_width,
        map_height,
        map_name,
        host_name,
        game_speed,
        game_type,
        players,
        matchup,
        commands: Vec::new(),
    })
}

fn build_matchup(players: &[Player]) -> String {
    if players.is_empty() {
        return String::new();
    }

    let mut teams: std::collections::BTreeMap<u8, Vec<&Player>> = std::collections::BTreeMap::new();
    for p in players {
        teams.entry(p.team).or_default().push(p);
    }

    if teams.len() == 1 || teams.len() == players.len() {
        let races: Vec<&str> = players.iter().map(|p| p.race.short()).collect();
        return races.join("v");
    }

    let team_strs: Vec<String> = teams
        .values()
        .map(|team_players| {
            let mut races: Vec<&str> = team_players.iter().map(|p| p.race.short()).collect();
            races.sort();
            races.join("")
        })
        .collect();

    team_strs.join("v")
}

// ─── Command parsing ─────────────────────────────────────────────────────────

fn parse_commands(data: &[u8]) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut pos: usize = 0;
    let len = data.len();

    while pos + 5 <= len {
        // Frame (u32)
        let frame = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        // Block size (u8)
        let block_size = data[pos] as usize;
        pos += 1;

        let block_end = (pos + block_size).min(len);

        while pos < block_end {
            if pos + 2 > block_end {
                break;
            }

            let player_id = data[pos];
            pos += 1;
            let type_id = data[pos];
            pos += 1;

            let remaining = block_end - pos;

            let cmd = match type_id {
                // Select / SelectAdd / SelectRemove
                0x09 | 0x0A | 0x0B => {
                    if remaining < 1 { pos = block_end; continue; }
                    let count = data[pos];
                    pos += 1;
                    let skip = count as usize * 2;
                    if remaining < 1 + skip { pos = block_end; continue; }
                    pos += skip;
                    CmdType::Select { count }
                }
                // Build
                0x0C => {
                    if remaining < 7 { pos = block_end; continue; }
                    let _order = data[pos]; pos += 1;
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let unit_id = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    CmdType::Build { unit_id, x, y }
                }
                // Vision
                0x0D => { if remaining < 2 { pos = block_end; continue; } pos += 2; CmdType::Other { type_id } }
                // Alliance
                0x0E => { if remaining < 4 { pos = block_end; continue; } pos += 4; CmdType::Other { type_id } }
                // GameSpeed
                0x0F => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Other { type_id } }
                // Pause, Resume
                0x10 | 0x11 => CmdType::Other { type_id },
                // Cheat
                0x12 => { if remaining < 4 { pos = block_end; continue; } pos += 4; CmdType::Other { type_id } }
                // Hotkey
                0x13 => {
                    if remaining < 2 { pos = block_end; continue; }
                    let hotkey_type = HotkeyAction::from_byte(data[pos]); pos += 1;
                    let group = data[pos]; pos += 1;
                    CmdType::Hotkey { hotkey_type, group }
                }
                // RightClick
                0x14 => {
                    if remaining < 9 { pos = block_end; continue; }
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    pos += 2; // unit_tag
                    pos += 2; // unit_type
                    let queued = data[pos] != 0; pos += 1;
                    CmdType::RightClick { x, y, queued }
                }
                // TargetedOrder
                0x15 => {
                    if remaining < 10 { pos = block_end; continue; }
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    pos += 2; // unit_tag
                    pos += 2; // unit_type
                    let order_id = data[pos]; pos += 1;
                    let queued = data[pos] != 0; pos += 1;
                    CmdType::TargetedOrder { x, y, order_id, queued }
                }
                // CancelBuild
                0x18 => CmdType::CancelBuild,
                // CancelMorph
                0x19 => CmdType::CancelMorph,
                // Stop, Burrow, Unburrow, ReturnCargo, HoldPosition, UnloadAll,
                // Unsiege, Siege, Cloack, Decloack
                0x1A => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Stop }
                0x1B => CmdType::Other { type_id }, // CarrierStop
                0x1C => CmdType::Other { type_id }, // ReaverStop
                0x1D => CmdType::Other { type_id }, // OrderNothing
                0x1E => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::ReturnCargo }
                // Train
                0x1F => {
                    if remaining < 2 { pos = block_end; continue; }
                    let unit_id = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    CmdType::Train { unit_id }
                }
                // CancelTrain
                0x20 => { if remaining < 2 { pos = block_end; continue; } pos += 2; CmdType::CancelTrain }
                // Cloack
                0x21 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Cloak }
                // Decloack
                0x22 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Decloak }
                // UnitMorph
                0x23 => {
                    if remaining < 2 { pos = block_end; continue; }
                    let unit_id = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    CmdType::UnitMorph { unit_id }
                }
                // Unsiege
                0x25 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Unsiege }
                // Siege
                0x26 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Siege }
                // TrainFighter
                0x27 => CmdType::TrainFighter,
                // UnloadAll
                0x28 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::UnloadAll }
                // Unload
                0x29 => { if remaining < 2 { pos = block_end; continue; } pos += 2; CmdType::Unload }
                // MergeArchon
                0x2A => CmdType::MergeArchon,
                // HoldPosition
                0x2B => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::HoldPosition }
                // Burrow
                0x2C => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Burrow }
                // Unburrow
                0x2D => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Unburrow }
                // CancelNuke
                0x2E => CmdType::CancelNuke,
                // LiftOff
                0x2F => {
                    if remaining < 4 { pos = block_end; continue; }
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    CmdType::LiftOff { x, y }
                }
                // Tech
                0x30 => {
                    if remaining < 1 { pos = block_end; continue; }
                    let tech_id = data[pos]; pos += 1;
                    CmdType::Tech { tech_id }
                }
                // CancelTech
                0x31 => CmdType::CancelTech,
                // Upgrade
                0x32 => {
                    if remaining < 1 { pos = block_end; continue; }
                    let upgrade_id = data[pos]; pos += 1;
                    CmdType::Upgrade { upgrade_id }
                }
                // CancelUpgrade
                0x33 => CmdType::CancelUpgrade,
                // CancelAddon
                0x34 => CmdType::CancelAddon,
                // BuildingMorph
                0x35 => {
                    if remaining < 2 { pos = block_end; continue; }
                    let unit_id = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    CmdType::BuildingMorph { unit_id }
                }
                // Stim
                0x36 => CmdType::Stim,
                // Sync
                0x37 => { if remaining < 6 { pos = block_end; continue; } pos += 6; CmdType::Other { type_id } }
                // VoiceEnable, VoiceDisable
                0x38 | 0x39 => CmdType::Other { type_id },
                // VoiceSquelch, VoiceUnsquelch
                0x3A | 0x3B => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Other { type_id } }
                // StartGame
                0x3C => CmdType::Other { type_id },
                // DownloadPercentage
                0x3D => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Other { type_id } }
                // ChangeGameSlot
                0x3E => { if remaining < 5 { pos = block_end; continue; } pos += 5; CmdType::Other { type_id } }
                // NewNetPlayer
                0x3F => { if remaining < 7 { pos = block_end; continue; } pos += 7; CmdType::Other { type_id } }
                // JoinedGame
                0x40 => { if remaining < 17 { pos = block_end; continue; } pos += 17; CmdType::Other { type_id } }
                // ChangeRace
                0x41 => { if remaining < 2 { pos = block_end; continue; } pos += 2; CmdType::Other { type_id } }
                // TeamGameTeam, UMSTeam
                0x42 | 0x43 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Other { type_id } }
                // MeleeTeam, SwapPlayers
                0x44 | 0x45 => { if remaining < 2 { pos = block_end; continue; } pos += 2; CmdType::Other { type_id } }
                // SavedData
                0x48 => { if remaining < 12 { pos = block_end; continue; } pos += 12; CmdType::Other { type_id } }
                // BriefingStart
                0x54 => CmdType::Other { type_id },
                // Latency
                0x55 => { if remaining < 1 { pos = block_end; continue; } pos += 1; CmdType::Other { type_id } }
                // ReplaySpeed
                0x56 => { if remaining < 9 { pos = block_end; continue; } pos += 9; CmdType::Other { type_id } }
                // LeaveGame
                0x57 => {
                    if remaining < 1 { pos = block_end; continue; }
                    let reason = data[pos]; pos += 1;
                    CmdType::LeaveGame { reason }
                }
                // MinimapPing
                0x58 => {
                    if remaining < 4 { pos = block_end; continue; }
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    CmdType::MinimapPing { x, y }
                }
                // MergeDarkArchon
                0x5A => CmdType::MergeDarkArchon,
                // MakeGamePublic
                0x5B => CmdType::Other { type_id },
                // Chat
                0x5C => {
                    if remaining < 81 { pos = block_end; continue; }
                    let _sender = data[pos]; pos += 1;
                    let msg = read_null_terminated(&data[pos..], 80);
                    pos += 80;
                    CmdType::Chat { message: msg }
                }
                // ─── 1.21 commands ───
                // RightClick121
                0x60 => {
                    if remaining < 11 { pos = block_end; continue; }
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    pos += 2; // unit_tag
                    pos += 2; // unknown
                    pos += 2; // unit_type
                    let queued = data[pos] != 0; pos += 1;
                    CmdType::RightClick { x, y, queued }
                }
                // TargetedOrder121
                0x61 => {
                    if remaining < 12 { pos = block_end; continue; }
                    let x = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    let y = u16::from_le_bytes([data[pos], data[pos + 1]]); pos += 2;
                    pos += 2; // unit_tag
                    pos += 2; // unknown
                    pos += 2; // unit_type
                    let order_id = data[pos]; pos += 1;
                    let queued = data[pos] != 0; pos += 1;
                    CmdType::TargetedOrder { x, y, order_id, queued }
                }
                // Unload121
                0x62 => { if remaining < 4 { pos = block_end; continue; } pos += 4; CmdType::Unload }
                // Select121, SelectAdd121, SelectRemove121
                0x63 | 0x64 | 0x65 => {
                    if remaining < 1 { pos = block_end; continue; }
                    let count = data[pos]; pos += 1;
                    let skip = count as usize * 4; // each unit: tag(2) + unknown(2)
                    if remaining < 1 + skip { pos = block_end; continue; }
                    pos += skip;
                    CmdType::Select { count }
                }
                // SaveGame, LoadGame
                0x06 | 0x07 => {
                    if remaining < 4 { pos = block_end; continue; }
                    let str_len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4;
                    if remaining < 4 + str_len { pos = block_end; continue; }
                    pos += str_len;
                    CmdType::Other { type_id }
                }
                // KeepAlive, RestartGame
                0x05 | 0x08 => CmdType::KeepAlive,
                // Unknown command - skip to end of block
                _ => {
                    pos = block_end;
                    continue;
                }
            };

            commands.push(Command {
                frame,
                player_id,
                cmd,
            });
        }

        pos = block_end;
    }

    commands
}
