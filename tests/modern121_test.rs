use std::io::Write;

/// Build a zlib-compressed section chunk from raw data
fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Build a section with checksum + 1 chunk (Modern format)
fn build_section(data: &[u8]) -> Vec<u8> {
    let compressed = zlib_compress(data);
    let mut out = Vec::new();
    out.extend_from_slice(&0xDEADBEEFu32.to_le_bytes()); // checksum
    out.extend_from_slice(&1u32.to_le_bytes()); // chunk_count = 1
    out.extend_from_slice(&(compressed.len() as u32).to_le_bytes()); // compressed_len
    out.extend_from_slice(&compressed); // compressed data
    out
}

/// Build a minimal valid BW header (0x279 bytes)
fn build_header() -> Vec<u8> {
    let mut h = vec![0u8; 0x279];
    h[0x00] = 1; // Engine: BroodWar
    let frames: u32 = 1000;
    h[0x01..0x05].copy_from_slice(&frames.to_le_bytes());
    h[0x08..0x0C].copy_from_slice(&1000i32.to_le_bytes());
    h[0x18] = b'T';
    h[0x34..0x36].copy_from_slice(&128u16.to_le_bytes());
    h[0x36..0x38].copy_from_slice(&128u16.to_le_bytes());
    h[0x3A] = 6; // Fastest
    h[0x3C..0x3E].copy_from_slice(&2u16.to_le_bytes()); // Melee
    h[0x48] = b'H';
    h[0x61] = b'M';
    let p1_offset = 0xA1;
    h[p1_offset + 2] = 2; // Human
    h[p1_offset + 3] = 1; // Terran
    h[0x171] = b'P';
    let p2_offset = p1_offset + 36;
    h[p2_offset] = 1;
    h[p2_offset + 2] = 2;
    h[p2_offset + 3] = 2; // Protoss
    h[0x171 + 36] = b'Q';
    h
}

/// Build a minimal commands section with a few known commands
fn build_commands() -> Vec<u8> {
    let mut cmds = Vec::new();
    // Frame 0: KeepAlive
    cmds.extend_from_slice(&0u32.to_le_bytes());
    cmds.push(2);
    cmds.push(0);
    cmds.push(0x05);
    // Frame 10: Select
    cmds.extend_from_slice(&10u32.to_le_bytes());
    cmds.push(5);
    cmds.push(0);
    cmds.push(0x09);
    cmds.push(1);
    cmds.extend_from_slice(&42u16.to_le_bytes());
    // Frame 20: RightClick
    cmds.extend_from_slice(&20u32.to_le_bytes());
    cmds.push(11);
    cmds.push(0);
    cmds.push(0x14);
    cmds.extend_from_slice(&100u16.to_le_bytes());
    cmds.extend_from_slice(&200u16.to_le_bytes());
    cmds.extend_from_slice(&0u16.to_le_bytes());
    cmds.extend_from_slice(&0u16.to_le_bytes());
    cmds.push(0);
    // Frame 30: Train
    cmds.extend_from_slice(&30u32.to_le_bytes());
    cmds.push(4);
    cmds.push(1);
    cmds.push(0x1F);
    cmds.extend_from_slice(&0x40u16.to_le_bytes());
    cmds
}

#[test]
fn test_parse_modern121_replay() {
    // In real 1.21 replays, the SCR offset appears once per logical section
    // (between S0-S1 and before S2), not per sub-section.
    let replay_id = b"seRS";
    let header = build_header();
    let commands = build_commands();

    let mut replay_data = Vec::new();

    // S0: Replay ID (no SCR offset before S0)
    replay_data.extend_from_slice(&build_section(replay_id));

    // SCR offset before S1 (once per logical section)
    replay_data.extend_from_slice(&0u32.to_le_bytes());
    // S1: Header
    replay_data.extend_from_slice(&build_section(&header));

    // No SCR offset before S2 — it only appears between S0 and S1
    // S2 size sub-section (no extra prefix)
    let cmd_len = commands.len() as u32;
    replay_data.extend_from_slice(&build_section(&cmd_len.to_le_bytes()));
    // S2 data sub-section (no extra prefix)
    replay_data.extend_from_slice(&build_section(&commands));

    eprintln!("Synthetic replay: {} bytes total", replay_data.len());

    let replay = pathtobonjwa::parser::parse_replay(&replay_data)
        .expect("Failed to parse synthetic Modern121 replay");

    eprintln!(
        "Result: {} frames, {} commands",
        replay.frames,
        replay.commands.len()
    );

    assert_eq!(replay.frames, 1000);
    assert_eq!(
        replay.commands.len(),
        4,
        "Expected 4 commands (KeepAlive, Select, RightClick, Train)"
    );
}
