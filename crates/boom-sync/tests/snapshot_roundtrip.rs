//! Nghiệm thu adapter `boom-sync` (ADR 0009 §4):
//!
//! - snapshot → state → snapshot khôi phục **cùng state hash** trên các trận scenario;
//! - kích thước snapshot Village00 4 người nằm trong bảng trần ADR 0009 §4.2;
//! - miền toạ độ và miền lưới được kiểm trước khi dựng lại state.

use std::collections::BTreeSet;
use std::path::PathBuf;

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, PlayerSetup, Rules, Stat, World, mode_by_id};
use boom_proto::{ProtocolLimits, Snapshot, decode_frame, encode_frame};
use boom_scenario::setup::{Setup, parse_checked};
use boom_sync::{SnapshotContext, from_snapshot, to_snapshot};

const LIMITS: ProtocolLimits = ProtocolLimits::v1();

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn scenario_files() -> Vec<PathBuf> {
    let dir = repo_path("fixtures/scenarios");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("thư mục scenario")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .flat_map(|d| {
            std::fs::read_dir(d)
                .expect("thư mục scenario")
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "phải có scenario trong fixtures/scenarios"
    );
    files
}

/// Khóa gốc của scenario trên cùng bộ khoá với `boom_scenario::scenario` (private).
const SCENARIO_KEYS: &[&str] = &[
    "name", "features", "claims", "status", "ticks", "expect", "end",
];

#[test]
fn snapshot_round_trip_keeps_state_hash_on_every_scenario() {
    let files = scenario_files();
    let mut checked = 0usize;
    for path in &files {
        let text = std::fs::read_to_string(path).expect("đọc scenario");
        let setup: Setup = parse_checked(&text, SCENARIO_KEYS)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let base = path.parent().expect("scenario có thư mục cha");
        let mut m = boom_scenario::setup::Match::new(&setup, base)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));

        // Chạy vài tick để trận có bom/lửa/item rồi mới chụp snapshot.
        for _ in 0..5 {
            if m.step().is_err() {
                break;
            }
        }

        let ctx = SnapshotContext {
            snapshot_seq: 1,
            round_id: 7,
            state_schema: 1,
            rules_hash: 0x0123_4567_89ab_cdef,
            content_epoch: 3,
            acks: Vec::new(),
        };
        let snapshot =
            to_snapshot(&m.state, &ctx).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let frame =
            encode_frame(&snapshot, &LIMITS).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let decoded: Snapshot =
            decode_frame(&frame, &LIMITS).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let restored =
            from_snapshot(&decoded, &m.world).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

        assert_eq!(
            restored.hash(),
            m.state.hash(),
            "state hash phải khôi phục đúng ở {}",
            path.display()
        );
        // Dựng lại lần hai phải cho cùng DTO (codec ổn định).
        let again = to_snapshot(&restored, &ctx).expect("dựng lại DTO");
        assert_eq!(again, snapshot, "DTO phải ổn định ở {}", path.display());
        checked += 1;
    }
    assert_eq!(
        checked,
        files.len(),
        "mọi scenario trong fixtures/scenarios phải qua round-trip state hash"
    );
}

/// Bản đồ 15×13 kiểu Village00: viền cứng, một số khối mềm, bốn spawn.
fn village00_like() -> Terrain {
    let rows = [
        "###############",
        "#0..+..+..2..+#",
        "#.+..+...+...+#",
        "#..+..1..+..+.#",
        "#+....+......+#",
        "#..+..+..+...+#",
        "#....3..+....+#",
        "#.+..+...+..+.#",
        "#..+..+..+..+.#",
        "#....+....+..+#",
        "#.+..+...+...+#",
        "#..+.....+..+.#",
        "###############",
    ];
    Terrain::from_ascii(&rows).expect("bản đồ hợp lệ")
}

fn world_and_state() -> (World, boom_core::SimulationState) {
    let profile = RulesProfile::from_toml(
        &std::fs::read_to_string(repo_path("fixtures/rules/skeleton-test.toml"))
            .expect("đọc rules fixture"),
    )
    .expect("profile hợp lệ");
    let rules = Rules::from_profile(&profile).expect("rules hợp lệ");
    let mode = mode_by_id("battle_ffa").expect("mode battle_ffa");
    let terrain = village00_like();
    let world = World::new(terrain, rules, mode).expect("world hợp lệ");
    let roster = vec![
        PlayerSetup {
            id: 0,
            stats: CharacterStats {
                speed: Stat::new(1, 5),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        },
        PlayerSetup {
            id: 1,
            stats: CharacterStats {
                speed: Stat::new(1, 5),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        },
        PlayerSetup {
            id: 2,
            stats: CharacterStats {
                speed: Stat::new(1, 5),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        },
        PlayerSetup {
            id: 3,
            stats: CharacterStats {
                speed: Stat::new(1, 5),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        },
    ];
    let state = boom_core::SimulationState::new_round(&world, &roster, 7).expect("dựng lượt");
    (world, state)
}

#[test]
fn village00_four_players_snapshot_is_within_adr_ceiling() {
    let (world, state) = world_and_state();
    let ctx = SnapshotContext {
        snapshot_seq: 1,
        round_id: 1,
        state_schema: 1,
        rules_hash: 0,
        content_epoch: 1,
        acks: (0..4)
            .map(|p| boom_proto::InputAck {
                player: p,
                last_seq: 0,
                applied_tick: 0,
            })
            .collect(),
    };
    let snapshot = to_snapshot(&state, &ctx).expect("dựng DTO");
    let frame = encode_frame(&snapshot, &LIMITS).expect("encode");

    // Bảng trần ADR 0009 §4.2: 5 692 byte cho frame đầy đủ 4 người.
    assert!(
        frame.len() <= 5_692,
        "snapshot 4 người phải nằm trong trần 5 692 byte, thực tế {}",
        frame.len()
    );
    assert!(frame.len() <= LIMITS.max_snapshot_bytes);

    // Và phải khôi phục lại đúng state.
    let decoded: Snapshot = decode_frame(&frame, &LIMITS).expect("decode");
    let restored = from_snapshot(&decoded, &world).expect("khôi phục");
    assert_eq!(restored.hash(), state.hash());
}

#[test]
fn snapshot_of_village00_uses_mpx_domain_of_the_session() {
    let (world, state) = world_and_state();
    let ctx = SnapshotContext {
        snapshot_seq: 1,
        round_id: 1,
        state_schema: 1,
        rules_hash: 0,
        content_epoch: 1,
        acks: Vec::new(),
    };
    let snapshot = to_snapshot(&state, &ctx).expect("dựng DTO");
    let tile_mpx = world.rules().tile.0;
    for p in &snapshot.players {
        assert!(p.x >= 0 && p.x < 15 * tile_mpx, "x ngoài miền Mpx");
        assert!(p.y >= 0 && p.y < 13 * tile_mpx, "y ngoài miền Mpx");
    }
}

#[test]
fn snapshot_with_position_outside_domain_is_rejected() {
    let (world, state) = world_and_state();
    let ctx = SnapshotContext {
        snapshot_seq: 1,
        round_id: 1,
        state_schema: 1,
        rules_hash: 0,
        content_epoch: 1,
        acks: Vec::new(),
    };
    let mut snapshot = to_snapshot(&state, &ctx).expect("dựng DTO");
    snapshot.players[0].x = 15 * world.rules().tile.0;
    let err = from_snapshot(&snapshot, &world).unwrap_err();
    assert!(
        matches!(err, boom_sync::SyncError::PositionOutOfRange { .. }),
        "thực tế: {err}"
    );
}

#[test]
fn snapshot_with_other_grid_size_is_rejected() {
    let (world, state) = world_and_state();
    let ctx = SnapshotContext {
        snapshot_seq: 1,
        round_id: 1,
        state_schema: 1,
        rules_hash: 0,
        content_epoch: 1,
        acks: Vec::new(),
    };
    let mut snapshot = to_snapshot(&state, &ctx).expect("dựng DTO");
    snapshot.field.height = 12;
    let err = from_snapshot(&snapshot, &world).unwrap_err();
    assert!(
        matches!(err, boom_sync::SyncError::GridMismatch { .. }),
        "thực tế: {err}"
    );
}

#[test]
fn scenario_set_is_not_empty_and_covers_every_system() {
    let files = scenario_files();
    let systems: BTreeSet<String> = files
        .iter()
        .filter_map(|p| p.parent())
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    for system in [
        "movement",
        "bomb_place",
        "explosion",
        "blocks",
        "pickup",
        "life",
        "round",
    ] {
        assert!(systems.contains(system), "thiếu scenario của {system}");
    }
}
