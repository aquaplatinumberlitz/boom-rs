//! Nghiệm thu adapter `boom-sync` (ADR 0009 §4):
//!
//! - snapshot → state → snapshot khôi phục **cùng state hash** trên các trận scenario;
//! - kích thước snapshot Village00 4 người nằm trong bảng trần ADR 0009 §4.2;
//! - miền toạ độ và miền lưới được kiểm trước khi dựng lại state.

use std::collections::BTreeSet;
use std::path::PathBuf;

use boom_content::map::{CellKind, Terrain};
use boom_content::rules::RulesProfile;
use boom_core::{
    Bomb, Cell, CharacterStats, EntityId, PlayerInput, PlayerSetup, Rules, Stat, World, mode_by_id,
};
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

        // **Một tick sau khi khôi phục** phải cho cùng hash và cùng event: state khôi phục
        // không được mất dữ liệu nào của mốc tick (kênh blast nằm ngoài state nên không có
        // gì để bỏ; xem `restored_state_plays_the_same_tick_including_block_destruction`).
        if m.state.phase == boom_core::Phase::Playing {
            let inputs = vec![boom_core::PlayerInput::default(); m.state.players.len()];
            let mut original = m.state.clone();
            let mut events_original = Vec::new();
            original
                .step(&m.world, &inputs, &mut events_original)
                .expect("step state gốc");
            let mut events_restored = Vec::new();
            let mut restored = restored;
            restored
                .step(&m.world, &inputs, &mut events_restored)
                .expect("step state khôi phục");
            assert_eq!(
                events_original,
                events_restored,
                "event ở tick sau khi khôi phục phải khớp ở {}",
                path.display()
            );
            assert_eq!(
                original.hash(),
                restored.hash(),
                "state hash ở tick sau khi khôi phục phải khớp ở {}",
                path.display()
            );
        }
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

/// Bản đồ và lượt solo đúng như ca reviewer Codex (PR #143): terrain `["0+.", "..."]`, người
/// chơi tại spawn 0, seed 1.
fn solo_world_and_state() -> (World, boom_core::SimulationState) {
    let profile = RulesProfile::from_toml(
        &std::fs::read_to_string(repo_path("fixtures/rules/skeleton-test.toml"))
            .expect("đọc rules fixture"),
    )
    .expect("profile hợp lệ");
    let rules = Rules::from_profile(&profile).expect("rules hợp lệ");
    let mode = mode_by_id("battle_ffa").expect("mode battle_ffa");
    let terrain = Terrain::from_ascii(&["0+.", "..."]).expect("bản đồ hợp lệ");
    let world = World::new(terrain, rules, mode).expect("world hợp lệ");
    let roster = vec![PlayerSetup {
        id: 0,
        stats: CharacterStats {
            speed: Stat::new(5, 7),
            bombs: Stat::new(1, 10),
            range: Stat::new(1, 7),
        },
    }];
    let state = boom_core::SimulationState::new_round(&world, &roster, 1).expect("dựng lượt");
    (world, state)
}

fn ctx() -> SnapshotContext {
    SnapshotContext {
        snapshot_seq: 1,
        round_id: 7,
        state_schema: 1,
        rules_hash: 0x0123_4567_89ab_cdef,
        content_epoch: 3,
        acks: Vec::new(),
    }
}

/// Tái hiện ca reviewer Codex (PR #143): **state hợp lệ mất dữ liệu khi snapshot và phân kỳ ở
/// tick kế tiếp**.
///
/// Kênh blast (ô bị vụ nổ phủ) từng là field của `SimulationState` nhưng không nằm trong DTO
/// snapshot: `to_snapshot` bỏ nó, `from_snapshot` dựng lại rỗng, nên state khôi phục lệch với
/// state gốc ở tick kế tiếp (một bên phá gạch, bên kia không → hash khác nhau).
///
/// Nay kênh blast **không nằm trong state** mà ở `TickScratch` của tick: state không mang nó
/// nên không có gì để bỏ khi đóng gói, và kết quả tick luôn suy ra từ state.
#[test]
fn restored_state_plays_the_same_tick_including_block_destruction() {
    let (world, mut state) = solo_world_and_state();
    // Bom trên ô người chơi, nổ ở tick 0, tầm 2: vụ nổ phủ ô (1,0) — khối mềm.
    let id = state.ids.next_id();
    state.bombs.push(Bomb {
        id,
        owner: 0,
        cell: Cell::new(0, 0),
        range: 2,
        explodes_at: 0,
        grace: vec![],
    });
    state.check_invariants(&world).expect("state hợp lệ");

    let snapshot = to_snapshot(&state, &ctx()).expect("dựng DTO");
    let mut restored = from_snapshot(&snapshot, &world).expect("khôi phục");
    assert_eq!(
        restored.hash(),
        state.hash(),
        "round-trip phải giữ state hash"
    );
    // DTO dựng lại từ state khôi phục phải **giống hệt** DTO gốc: không có trường nào bị bỏ
    // âm thầm (trước đây kênh blast nằm trong state nhưng không có trong DTO).
    assert_eq!(
        to_snapshot(&restored, &ctx()).expect("dựng lại DTO"),
        snapshot,
        "DTO phải giống hệt: state khôi phục không mất trường nào"
    );

    let mut original = state.clone();
    let mut events_original = Vec::new();
    original
        .step(&world, &[PlayerInput::default()], &mut events_original)
        .expect("step state gốc");
    let mut events_restored = Vec::new();
    restored
        .step(&world, &[PlayerInput::default()], &mut events_restored)
        .expect("step state khôi phục");

    assert_eq!(
        events_original, events_restored,
        "cùng state + cùng input ⇒ cùng event ở tick kế tiếp"
    );
    assert_eq!(
        original.hash(),
        restored.hash(),
        "cùng state + cùng input ⇒ cùng state hash ở tick kế tiếp"
    );
    // Khối mềm (1,0) phải vỡ giống nhau ở cả hai bên (trước đây chỉ một bên vỡ).
    for s in [&original, &restored] {
        assert_eq!(
            s.field.kind(Cell::new(1, 0)),
            Some(CellKind::Floor),
            "gạch trong blast phải vỡ ở cả state gốc lẫn state khôi phục"
        );
    }
}

/// State hợp lệ ở biên tick không mang kênh blast: không có gì để snapshot âm thầm bỏ.
#[test]
fn state_at_tick_boundary_carries_no_blast_channel() {
    let (world, state) = solo_world_and_state();
    state.check_invariants(&world).expect("state hợp lệ");
    // DTO không có trường blast: số phần tử của DTO là hằng, đổi profile không sinh thêm.
    let snapshot = to_snapshot(&state, &ctx()).expect("dựng DTO");
    assert_eq!(
        snapshot.flames.len(),
        state.flames.len(),
        "lửa thì có trong DTO (không phải kênh trong tick)"
    );
    // Khôi phục rồi step: không mất dữ liệu nào, nên hai bên phải trùng nhau.
    let restored = from_snapshot(&snapshot, &world).expect("khôi phục");
    let mut a = state.clone();
    let mut b = restored;
    let mut ea = Vec::new();
    let mut eb = Vec::new();
    a.step(&world, &[PlayerInput::default()], &mut ea)
        .expect("step");
    b.step(&world, &[PlayerInput::default()], &mut eb)
        .expect("step");
    assert_eq!(a.hash(), b.hash());
    assert_eq!(ea, eb);
}

/// ID entity cấp cho bom phải đi kèm state: round-trip không được cấp lại ID đã dùng.
#[test]
fn restored_state_keeps_entity_id_allocator() {
    let (world, mut state) = solo_world_and_state();
    let id = state.ids.next_id();
    assert_eq!(id, EntityId(1));
    state.bombs.push(Bomb {
        id,
        owner: 0,
        cell: Cell::new(0, 0),
        range: 2,
        explodes_at: 0,
        grace: vec![],
    });
    let snapshot = to_snapshot(&state, &ctx()).expect("dựng DTO");
    let restored = from_snapshot(&snapshot, &world).expect("khôi phục");
    assert_eq!(restored.ids.peek_next(), state.ids.peek_next());
}

/// Tái hiện **đúng ca reviewer Codex** (PR #143) qua đường fixture của repo (khóa `[[blast]]`
/// của scenario `blocks/blast_breaks_block.toml`), tức đường đã làm state mang dữ liệu mà
/// snapshot bỏ mất:
///
/// 1. Scenario dựng sẵn ô bị vụ nổ phủ trên khối mềm.
/// 2. `to_snapshot` bỏ kênh blast, `from_snapshot` dựng lại không có ⇒ state khôi phục khác.
/// 3. Step một tick: một bên phá gạch, bên kia không ⇒ **hash lệch**.
///
/// Nay kênh blast là dữ liệu **của tick** (nằm ngoài state), nên state khôi phục phải cho cùng
/// hash và cùng event với state gốc ở tick kế tiếp.
#[test]
fn fixture_with_prestate_blast_survives_a_snapshot_round_trip() {
    let path = repo_path("fixtures/scenarios/blocks/blast_breaks_block.toml");
    let text = std::fs::read_to_string(&path).expect("đọc scenario");
    let setup: Setup = parse_checked(&text, SCENARIO_KEYS).expect("scenario hợp lệ");
    let base = path.parent().expect("scenario có thư mục cha");
    let m = boom_scenario::setup::Match::new(&setup, base).expect("dựng trận");

    let snapshot = to_snapshot(&m.state, &ctx()).expect("dựng DTO");
    let mut restored = from_snapshot(&snapshot, &m.world).expect("khôi phục");
    assert_eq!(
        restored.hash(),
        m.state.hash(),
        "round-trip phải giữ state hash"
    );

    let inputs = vec![boom_core::PlayerInput::default(); m.state.players.len()];
    let mut original = m.state.clone();
    let mut events_original = Vec::new();
    original
        .step(&m.world, &inputs, &mut events_original)
        .expect("step state gốc");
    let mut events_restored = Vec::new();
    restored
        .step(&m.world, &inputs, &mut events_restored)
        .expect("step state khôi phục");

    assert_eq!(
        events_original, events_restored,
        "cùng state + cùng input ⇒ cùng event ở tick kế tiếp"
    );
    assert_eq!(
        original.hash(),
        restored.hash(),
        "cùng state + cùng input ⇒ cùng state hash ở tick kế tiếp"
    );
    // Ô (2,1) phải ở cùng trạng thái ở cả hai bên (trước đây một bên vỡ, bên kia không).
    assert_eq!(
        original.field.kind(Cell::new(2, 1)),
        restored.field.kind(Cell::new(2, 1))
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
