use pathtobonjwa::openbw::Simulator;
use pathtobonjwa::parser;
use pathtobonjwa::simulation::{self, SimConfig};

#[test]
fn simulator_lifecycle() {
    let sim = Simulator::new().expect("create failed");
    assert!(sim.is_done(), "unloaded simulator should report done");
    assert_eq!(sim.current_frame(), 0);
    drop(sim);
}

#[test]
fn simulator_load_and_step() {
    let data = std::fs::read("test.rep").expect("test.rep not found");
    let (header, commands) = parser::extract_raw_sections(&data).expect("extract failed");

    let mut sim = Simulator::new().unwrap();
    sim.load_replay(&header, &commands).unwrap();

    assert!(!sim.is_done(), "loaded replay should not be done at frame 0");
    assert_eq!(sim.current_frame(), 0);

    // Step a few frames
    for _ in 0..100 {
        if sim.is_done() {
            break;
        }
        sim.next_frame().unwrap();
    }
    assert!(sim.current_frame() > 0);
}

#[test]
fn simulator_get_units_and_players() {
    let data = std::fs::read("test.rep").expect("test.rep not found");
    let (header, commands) = parser::extract_raw_sections(&data).expect("extract failed");

    let mut sim = Simulator::new().unwrap();
    sim.load_replay(&header, &commands).unwrap();

    let players = sim.get_player_info();
    assert!(!players.is_empty(), "should have at least one player");

    let units = sim.get_units();
    assert!(!units.is_empty(), "should have starting units");

    // All starting units should be alive
    for u in &units {
        assert!(u.alive, "starting units should be alive");
    }
}

#[test]
fn simulator_snapshot() {
    let data = std::fs::read("test.rep").expect("test.rep not found");
    let (header, commands) = parser::extract_raw_sections(&data).expect("extract failed");

    let mut sim = Simulator::new().unwrap();
    sim.load_replay(&header, &commands).unwrap();

    let snap = sim.snapshot();
    assert_eq!(snap.frame, 0);
    assert!(!snap.players.is_empty());
    assert!(!snap.units.is_empty());
}

#[test]
fn simulate_full_replay() {
    let data = std::fs::read("test.rep").expect("test.rep not found");
    let (header, commands) = parser::extract_raw_sections(&data).expect("extract failed");

    let config = SimConfig {
        sample_interval: 240, // ~10 sec intervals for speed
    };
    let result = simulation::simulate_replay(&header, &commands, &config).unwrap();

    assert!(result.total_frames > 0, "should have simulated frames");
    assert!(
        result.snapshots.len() >= 2,
        "should have at least start and end snapshots"
    );

    // Final state should match total frames
    assert_eq!(result.final_state.frame, result.total_frames);
}

#[test]
fn supply_curve_extraction() {
    let data = std::fs::read("test.rep").expect("test.rep not found");
    let (header, commands) = parser::extract_raw_sections(&data).expect("extract failed");

    let config = SimConfig {
        sample_interval: 480,
    };
    let result = simulation::simulate_replay(&header, &commands, &config).unwrap();

    // Get supply curve for player 0 (slot 0 in the replay header)
    let players = &result.snapshots[0].players;
    if !players.is_empty() {
        let pid = players[0].player_id;
        let curve = result.supply_curve(pid);
        assert!(!curve.is_empty(), "supply curve should have data points");

        // Starting supply should be reasonable (4 workers)
        let (_, first_used, first_max) = curve[0];
        assert!(first_used > 0.0, "should start with some supply used");
        assert!(first_max > 0.0, "should start with some supply capacity");
    }
}

#[test]
fn scr_replay_simulation() {
    let data = std::fs::read("scr_replay.rep").expect("scr_replay.rep not found");
    let (header, commands) = parser::extract_raw_sections(&data).expect("extract failed");

    let mut sim = Simulator::new().unwrap();
    sim.load_replay(&header, &commands).unwrap();
    assert!(!sim.is_done());

    // Step through first 50 frames
    for _ in 0..50 {
        if sim.is_done() {
            break;
        }
        sim.next_frame().unwrap();
    }
    assert!(sim.current_frame() > 0);
}
