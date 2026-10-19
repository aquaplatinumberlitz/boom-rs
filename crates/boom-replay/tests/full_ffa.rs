//! Golden `full-ffa` phải thực sự chạm pipeline FFA, không chỉ so một hash cuối.

use std::path::Path;

use boom_core::{EventKind, Outcome, Phase};
use boom_scenario::setup::{Match, Setup, parse_checked};

#[test]
fn full_ffa_covers_all_v1_systems_and_ends_with_winner() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/replays/full-ffa.toml");
    let text = std::fs::read_to_string(&path).expect("đọc full-ffa");
    let setup: Setup = parse_checked(
        &text,
        &[
            "schema",
            "hash_algo",
            "content_sha256",
            "ticks",
            "checkpoints",
        ],
    )
    .expect("parse full-ffa");
    let mut m = Match::new(&setup, path.parent().unwrap()).expect("dựng full-ffa");

    let mut seen = [false; 9];
    while m.state.phase == Phase::Playing && m.state.tick < 300 {
        m.step().expect("tick full-ffa");
        for event in &m.events {
            match &event.kind {
                EventKind::BombPlaced { .. } => seen[0] = true,
                EventKind::BombExploded { chained: true, .. } => seen[1] = true,
                EventKind::BlockDestroyed { .. } => seen[2] = true,
                EventKind::ItemRevealed { .. } => seen[3] = true,
                EventKind::ItemPicked { kind, .. } => match kind.0 {
                    0 => seen[4] = true,
                    1 => seen[5] = true,
                    2 => seen[6] = true,
                    _ => {}
                },
                EventKind::PlayerTrapped { .. } => seen[7] = true,
                EventKind::PlayerDied { .. } => seen[8] = true,
                _ => {}
            }
        }
        m.events.clear();
    }
    assert_eq!(seen, [true; 9], "thiếu event pipeline: {seen:?}");
    assert_eq!(m.state.phase, Phase::Ended(Outcome::Winner(0)));
}
