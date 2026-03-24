use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

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
}

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
            1 => PlayerType::Computer,
            2 => PlayerType::Human,
            3 => PlayerType::RescuePassive,
            5 => PlayerType::Computer,
            6 => PlayerType::Human,
            7 => PlayerType::Computer,
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
    Slowest,
    Slower,
    Slow,
    Normal,
    Fast,
    Faster,
    Fastest,
    Unknown(u8),
}

impl GameSpeed {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => GameSpeed::Slowest,
            1 => GameSpeed::Slower,
            2 => GameSpeed::Slow,
            3 => GameSpeed::Normal,
            4 => GameSpeed::Fast,
            5 => GameSpeed::Faster,
            6 => GameSpeed::Fastest,
            _ => GameSpeed::Unknown(b),
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
    Melee,
    FreeForAll,
    OneOnOne,
    CaptureTheFlag,
    Greed,
    Slaughter,
    SuddenDeath,
    Ladder,
    UseMapSettings,
    TeamMelee,
    TeamFreeForAll,
    TeamCaptureTheFlag,
    TopVsBottom,
    Unknown(u16),
}

impl GameType {
    fn from_u16(v: u16) -> Self {
        match v {
            0x02 => GameType::Melee,
            0x03 => GameType::FreeForAll,
            0x04 => GameType::OneOnOne,
            0x05 => GameType::CaptureTheFlag,
            0x06 => GameType::Greed,
            0x07 => GameType::Slaughter,
            0x08 => GameType::SuddenDeath,
            0x09 => GameType::Ladder,
            0x0A => GameType::UseMapSettings,
            0x0B => GameType::TeamMelee,
            0x0C => GameType::TeamFreeForAll,
            0x0D => GameType::TeamCaptureTheFlag,
            0x0F => GameType::TopVsBottom,
            _ => GameType::Unknown(v),
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

const FRAMES_PER_SECOND: f64 = 23.81;
const HEADER_SIZE: usize = 0x279;

fn read_null_terminated(data: &[u8], max_len: usize) -> String {
    let len = max_len.min(data.len());
    let end = data.iter().take(len).position(|&b| b == 0).unwrap_or(len);
    String::from_utf8_lossy(&data[..end]).to_string()
}

pub fn parse_replay(data: &[u8]) -> Result<Replay, String> {
    if data.len() < 4 {
        return Err("File too small to be a replay".into());
    }

    let magic = &data[0..4];
    let is_modern = magic == b"seRS";
    let is_legacy = magic == b"reRS";

    if !is_modern && !is_legacy {
        return Err(format!(
            "Invalid replay signature: {:02X} {:02X} {:02X} {:02X} (expected 'seRS' or 'reRS')",
            magic[0], magic[1], magic[2], magic[3]
        ));
    }

    if data.len() < 8 {
        return Err("File too small for header size".into());
    }

    let mut cur = Cursor::new(&data[4..8]);
    let section_size = cur.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;

    let header_offset = 8;
    if data.len() < header_offset + section_size.max(1) {
        return Err("File truncated in header section".into());
    }

    let header_data = if section_size > 0 && data[header_offset] == 0x78 {
        zlib_decompress(&data[header_offset..header_offset + section_size])?
    } else {
        data[header_offset..header_offset + section_size].to_vec()
    };

    if header_data.len() < HEADER_SIZE {
        return Err(format!(
            "Header too small: {} bytes (need {})",
            header_data.len(),
            HEADER_SIZE
        ));
    }

    parse_header(&header_data)
}

fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| format!("zlib decompress failed: {}", e))?;
    Ok(out)
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
