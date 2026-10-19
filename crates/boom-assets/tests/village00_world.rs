//! Nghiệm thu V1-030: phân loại ô theme Village và dựng `World` từ Village00 KR.
//!
//! Dữ liệu game có bản quyền không nằm trong repo: đặt đường dẫn file `.map` đã giải nén vào
//! `BOOM_KR_VILLAGE00_MAP`; thiếu biến thì test tự bỏ qua. Bảng phân loại và rules profile là
//! dữ liệu nguồn trong repo nên luôn được kiểm.

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;

use boom_assets::legacy_map::{Build, LegacyMap, parse};
use boom_content::map::{CellKind, MapProfile, Terrain};
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, PlayerSetup, Rules, SimulationState, Stat, World, mode_by_id};

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn profile() -> MapProfile {
    let path = repo_path("data/maps/village00-kr.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    MapProfile::from_toml(&text).unwrap()
}

fn rules() -> Rules {
    let path = repo_path("data/rules/v1-dev.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let profile = RulesProfile::from_toml(&text).unwrap();
    Rules::from_profile(&profile).unwrap()
}

fn load_kr() -> Option<LegacyMap> {
    let path = std::env::var_os("BOOM_KR_VILLAGE00_MAP")?;
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("BOOM_KR_VILLAGE00_MAP: {e}"));
    Some(parse(&data, Build::Kr).unwrap())
}

fn stats() -> CharacterStats {
    CharacterStats {
        speed: Stat::new(1, 5),
        bombs: Stat::new(1, 5),
        range: Stat::new(1, 5),
    }
}

fn reachable_without_solid(terrain: &Terrain, start: (u16, u16)) -> BTreeSet<(u16, u16)> {
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in [
            (i32::from(x) + 1, i32::from(y)),
            (i32::from(x) - 1, i32::from(y)),
            (i32::from(x), i32::from(y) + 1),
            (i32::from(x), i32::from(y) - 1),
        ] {
            let Some(kind) = terrain.kind(nx, ny) else {
                continue;
            };
            if kind == CellKind::Solid {
                continue;
            }
            let cell = (
                u16::try_from(nx).expect("trong lưới"),
                u16::try_from(ny).expect("trong lưới"),
            );
            if seen.insert(cell) {
                queue.push_back(cell);
            }
        }
    }
    seen
}

#[test]
fn profile_is_consistent_and_covers_every_grid_id() {
    let Some(map) = load_kr() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_VILLAGE00_MAP");
        return;
    };
    // RULES_V1 R-002, R-005: lưới và spawn đã xác nhận.
    assert_eq!((map.grid.width, map.grid.height), (15, 13));
    assert_eq!(map.grid.spawns, vec![(13, 11), (1, 11), (1, 1), (13, 1)]);

    // Không còn ô Unknown: mọi tile/obj ID của Village00 có tên và lớp trong bảng.
    let terrain = profile().classify(&map.grid).unwrap();
    assert!(!terrain.has_unknown());
    assert_eq!(terrain.kinds.len(), 15 * 13);
    for &(x, y) in &map.grid.spawns {
        assert_eq!(
            terrain.kind(i32::from(x), i32::from(y)),
            Some(CellKind::Floor),
            "spawn ({x},{y})"
        );
    }
}

#[test]
fn world_and_round_are_built_from_village00_kr() {
    let Some(map) = load_kr() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_VILLAGE00_MAP");
        return;
    };
    let terrain = profile().classify(&map.grid).unwrap();
    let mode = mode_by_id("battle_ffa").expect("mode battle_ffa");
    let world = World::new(terrain, rules(), mode).expect("World từ Village00 KR");

    let roster: Vec<PlayerSetup> = (0..4)
        .map(|id| PlayerSetup { id, stats: stats() })
        .collect();
    let state = SimulationState::new_round(&world, &roster, 0xC0FFEE).expect("round 4 người");
    assert_eq!(state.players.len(), 4);
    for player in &state.players {
        let cell = world.cell_at(player.x, player.y);
        assert_eq!(world.terrain().kind(cell.x, cell.y), Some(CellKind::Floor));
    }
}

#[test]
fn every_spawn_has_two_adjacent_empty_cells_and_an_escape() {
    let Some(map) = load_kr() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_VILLAGE00_MAP");
        return;
    };
    let terrain = profile().classify(&map.grid).unwrap();
    let spawns: BTreeSet<(u16, u16)> = map.grid.spawns.iter().copied().collect();

    for &(x, y) in &map.grid.spawns {
        let mut horizontal = false;
        let mut vertical = false;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = i32::from(x) + dx;
            let ny = i32::from(y) + dy;
            if terrain.kind(nx, ny) == Some(CellKind::Floor) {
                horizontal |= dx != 0;
                vertical |= dy != 0;
            }
        }
        // R-007: ít nhất hai ô trống liền kề theo hình chữ L (ngang + dọc).
        assert!(horizontal && vertical, "spawn ({x},{y}) không có góc thoát");

        // Đường thoát ra khỏi góc spawn: tính cả gạch có thể phá, chỉ chặn ô Solid.
        let reachable = reachable_without_solid(&terrain, (x, y));
        assert_eq!(
            reachable.intersection(&spawns).count(),
            4,
            "spawn ({x},{y}) không nối được tới cả bốn góc"
        );
        assert!(
            reachable.len() > 4,
            "spawn ({x},{y}) vẫn bị nhốt trong góc sau khi phá gạch"
        );
    }
}
