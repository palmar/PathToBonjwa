use std::fs;

fn test_replay(path: &str) {
    if !std::path::Path::new(path).exists() {
        eprintln!("Skipping: no {} found", path);
        return;
    }
    let data = fs::read(path).expect("Failed to read replay file");
    eprintln!("\n=== {} ({} bytes) ===", path, data.len());
    let replay = pathtobonjwa::parser::parse_replay(&data).expect("Failed to parse replay");
    eprintln!(
        "Result: {} — {} players, {} frames, {} commands",
        replay.matchup,
        replay.players.len(),
        replay.frames,
        replay.commands.len()
    );
    assert!(
        replay.commands.len() > 0,
        "Expected commands but got 0 for a {} frame game in {}",
        replay.frames,
        path
    );
}

#[test]
fn test_parse_modern_replay() {
    test_replay("test.rep");
}

#[test]
fn test_parse_scr_replay() {
    test_replay("scr_replay.rep");
}

#[test]
fn test_parse_rep2() {
    test_replay("rep2.rep");
}

#[test]
fn test_parse_sb() {
    test_replay("sb.rep");
}
