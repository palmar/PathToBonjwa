use std::fs;

fn check_eapm(path: &str) {
    if !std::path::Path::new(path).exists() {
        eprintln!("Skipping: no {} found", path);
        return;
    }
    let data = fs::read(path).expect("Failed to read replay file");
    let replay = pathtobonjwa::parser::parse_replay(&data).expect("Failed to parse replay");

    let mut any_real_player = false;
    for p in &replay.players {
        let apm =
            pathtobonjwa::analytics::compute_apm(&replay.commands, p.player_id, replay.frames);
        let ratio = if apm.avg_apm > 0.0 {
            apm.avg_eapm / apm.avg_apm * 100.0
        } else {
            100.0
        };
        eprintln!(
            "  {}: APM={:.1}, EAPM={:.1} (ratio={:.1}%)",
            p.name, apm.avg_apm, apm.avg_eapm, ratio
        );
        // Skip players with low actions (computers, observers, very short/slow games)
        if apm.avg_apm < 30.0 {
            continue;
        }
        any_real_player = true;
        // EAPM should be strictly less than APM for any real player
        assert!(
            apm.avg_eapm < apm.avg_apm,
            "EAPM ({:.1}) should be less than APM ({:.1}) for player {} in {}",
            apm.avg_eapm,
            apm.avg_apm,
            p.name,
            path
        );
        // EAPM should be a reasonable fraction (typically 20-95% of APM)
        assert!(
            ratio < 100.0,
            "EAPM ratio should be < 100% but was {:.1}% for player {} in {}",
            ratio,
            p.name,
            path
        );
    }
    if !any_real_player {
        eprintln!(
            "  (no players with >10 APM in {} — skipping assertions)",
            path
        );
    }
}

#[test]
fn test_eapm_filters_spam_test_rep() {
    check_eapm("test.rep");
}

#[test]
fn test_eapm_filters_spam_scr() {
    check_eapm("scr_replay.rep");
}

#[test]
fn test_eapm_filters_spam_rep2() {
    check_eapm("rep2.rep");
}

#[test]
fn test_eapm_filters_spam_sb() {
    check_eapm("sb.rep");
}
