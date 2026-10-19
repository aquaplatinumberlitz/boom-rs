use boom_content::map::{CellKind, Terrain};
use boom_content::rules::RulesProfile;

use crate::event::{EventId, EventKind, SimulationEvent};
use crate::mode::{Outcome, mode_by_id};
use crate::num::{Cell, Mpx};
use crate::rules::{ItemEffect, Rules};
use crate::sim::{CoreError, Dir, PlayerInput, PlayerSetup, World};
use crate::state::{
    Bomb, CharacterStats, EntityId, Field, Flame, FloorItem, ItemKind, Life, Phase,
    SimulationState, Stat,
};

const PROFILE: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

fn rules() -> Rules {
    Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
}

fn world(rows: &[&str]) -> World {
    let ffa = mode_by_id("battle_ffa").unwrap();
    World::new(Terrain::from_ascii(rows).unwrap(), rules(), ffa).unwrap()
}

fn setup(id: u8) -> PlayerSetup {
    PlayerSetup {
        id,
        stats: CharacterStats {
            speed: Stat::new(5, 7),
            bombs: Stat::new(1, 10),
            range: Stat::new(1, 7),
        },
    }
}

fn walk(dir: Dir) -> PlayerInput {
    PlayerInput {
        dir,
        place_bomb: false,
    }
}

fn solo(w: &World) -> SimulationState {
    SimulationState::new_round(w, &[setup(0)], 1).unwrap()
}

fn run(w: &World, s: &mut SimulationState, dir: Dir, ticks: u32) -> Vec<SimulationEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        s.step(w, &[walk(dir)], &mut events).unwrap();
        s.check_invariants(w).unwrap();
    }
    events
}

#[test]
fn rules_convert_units() {
    let r = rules();
    // 24 px/s ở 60 Hz = 0,4 px/tick = 400 mpx/tick.
    assert_eq!(r.speed_per_level, Mpx(400));
    assert_eq!(r.tile, Mpx(40_000));
    assert_eq!(r.round_ticks, 7200);
    assert_eq!((r.fuse_ticks, r.flame_ticks, r.trap_ticks), (150, 30, 378));
    let keys: Vec<_> = r.items.iter().map(|i| (i.key, i.effect)).collect();
    assert_eq!(
        keys,
        vec![
            ("Bomb", ItemEffect::BombCapacity),
            ("Fire", ItemEffect::BlastRange),
            ("Roller", ItemEffect::SpeedLevel)
        ]
    );
}

#[test]
fn pipeline_order_is_the_contract() {
    let names: Vec<_> = mode_by_id("battle_ffa")
        .unwrap()
        .systems()
        .iter()
        .map(|s| s.name())
        .collect();
    assert_eq!(
        names,
        [
            "movement",
            "bomb_place",
            "explosion",
            "blocks",
            "pickup",
            "life"
        ]
    );
}

#[test]
fn walks_until_wall_and_stops_at_cell_center() {
    let w = world(&["#####", "#0..#", "#####"]);
    let mut s = solo(&w);
    assert_eq!((s.players[0].x, s.players[0].y), (Mpx(60_000), Mpx(60_000)));
    // 2 px/tick; hai ô = 80 px → 40 tick; thêm tick không vượt tường.
    run(&w, &mut s, Dir::Right, 100);
    assert_eq!(s.players[0].x, Mpx(140_000));
    run(&w, &mut s, Dir::Up, 10);
    assert_eq!(s.players[0].y, Mpx(60_000));
    assert_eq!(s.players[0].facing, Dir::Up);
}

#[test]
fn breakable_blocks_like_solid() {
    let w = world(&["0+."]);
    let mut s = solo(&w);
    run(&w, &mut s, Dir::Right, 50);
    assert_eq!(s.players[0].x, Mpx(20_000));
}

#[test]
fn aligns_before_turning() {
    let w = world(&["0..", "...", "..."]);
    let mut s = solo(&w);
    run(&w, &mut s, Dir::Down, 5); // lệch 10 px khỏi tâm hàng 0, tâm vẫn trong ô hàng 0
    assert_eq!(s.players[0].y, Mpx(30_000));
    run(&w, &mut s, Dir::Right, 5); // căn 10 px về tâm ô đang đứng, chưa đi ngang
    assert_eq!((s.players[0].x, s.players[0].y), (Mpx(20_000), Mpx(20_000)));
    run(&w, &mut s, Dir::Right, 1);
    assert_eq!(s.players[0].x, Mpx(22_000));
}

#[test]
fn turning_emits_events_with_stable_ids() {
    let w = world(&["0.."]);
    let mut s = solo(&w);
    let events = run(&w, &mut s, Dir::Right, 3);
    assert_eq!(
        events,
        vec![SimulationEvent {
            id: EventId { tick: 0, seq: 0 },
            kind: EventKind::PlayerTurned { player: 0 },
        }]
    );
}

#[test]
fn round_ends_on_time_limit_as_draw() {
    let w = world(&["0.1"]);
    let mut s = SimulationState::new_round(&w, &[setup(0), setup(1)], 0).unwrap();
    let mut events = Vec::new();
    let idle = [PlayerInput::default(); 2];
    for _ in 0..w.rules().round_ticks {
        s.step(&w, &idle, &mut events).unwrap();
    }
    assert_eq!(s.phase, Phase::Ended(Outcome::Draw));
    assert_eq!(
        events.last().map(|e| &e.kind),
        Some(&EventKind::RoundEnded {
            outcome: Outcome::Draw
        })
    );
    assert_eq!(s.step(&w, &idle, &mut events), Err(CoreError::RoundOver));
}

#[test]
fn last_player_not_dead_wins_and_trapped_still_counts() {
    let w = world(&["0.1"]);
    let mut s = SimulationState::new_round(&w, &[setup(0), setup(1)], 0).unwrap();
    let mut events = Vec::new();
    s.players[1].life = Life::Trapped { since: 0 };
    s.step(&w, &[PlayerInput::default(); 2], &mut events)
        .unwrap();
    assert_eq!(s.phase, Phase::Playing, "bị nhốt chưa phải chết");
    s.players[1].life = Life::Dead; // thay cho system life chưa có
    s.step(&w, &[PlayerInput::default(); 2], &mut events)
        .unwrap();
    assert_eq!(s.phase, Phase::Ended(Outcome::Winner(0)));
}

#[test]
fn rejects_unknown_cells_and_bad_rosters() {
    let ffa = mode_by_id("battle_ffa").unwrap();
    assert_eq!(
        World::new(Terrain::from_ascii(&["0?"]).unwrap(), rules(), ffa).err(),
        Some(CoreError::UnknownCells)
    );
    let w = world(&["0.1"]);
    assert_eq!(
        SimulationState::new_round(&w, &[setup(1), setup(1)], 0).err(),
        Some(CoreError::DuplicatePlayer(1))
    );
    assert_eq!(
        SimulationState::new_round(&w, &[setup(0), setup(1), setup(2)], 0).err(),
        Some(CoreError::TooManyPlayers {
            players: 3,
            spawns: 2
        })
    );
    let mut fast = setup(0);
    fast.stats.speed = Stat::new(5, 60);
    assert_eq!(
        SimulationState::new_round(&w, &[fast], 0).err(),
        Some(CoreError::SpeedTooHigh(60))
    );
}

#[test]
fn spawn_assignment_depends_on_seed_only() {
    let w = world(&["0.1", "...", "2.3"]);
    let roster: Vec<_> = (0..4).map(setup).collect();
    let a = SimulationState::new_round(&w, &roster, 7).unwrap();
    let b = SimulationState::new_round(&w, &roster, 7).unwrap();
    assert_eq!(a, b);
    let mut spots: Vec<_> = a.players.iter().map(|p| (p.x, p.y)).collect();
    spots.sort();
    spots.dedup();
    assert_eq!(spots.len(), 4, "mỗi người một spawn");
    let differs = (0..20).any(|seed| {
        SimulationState::new_round(&w, &roster, seed)
            .unwrap()
            .players
            != a.players
    });
    assert!(differs, "seed phải ảnh hưởng hoán vị spawn");
}

fn bomb(id: u32, cell: Cell, owner: u8) -> Bomb {
    Bomb {
        id: EntityId(id),
        owner,
        cell,
        range: 1,
        explodes_at: 10,
        grace: vec![],
    }
}

/// State đã cấp `n` ID, cho các ca kiểm bất biến.
fn with_ids(w: &World, n: u32) -> SimulationState {
    let mut s = solo(w);
    for _ in 0..n {
        s.ids.next_id();
    }
    s
}

fn violation(w: &World, s: &SimulationState) -> String {
    s.check_invariants(w).expect_err("phải vi phạm contract")
}

#[test]
fn invariants_catch_basic_violations() {
    let w = world(&["0.."]);
    let mut s = with_ids(&w, 2);
    s.check_invariants(&w).unwrap();
    s.bombs = vec![bomb(1, Cell::new(1, 0), 0), bomb(2, Cell::new(1, 0), 0)];
    assert!(violation(&w, &s).contains("hai bom"));
    s.bombs = vec![bomb(9, Cell::new(1, 0), 0)];
    assert!(violation(&w, &s).contains("IdAllocator"));
    s.bombs.clear();
    s.players[0].range = Stat { value: 9, max: 7 };
    assert!(violation(&w, &s).contains("vượt trần"));
}

#[test]
fn invariants_require_unique_ids_across_entity_kinds() {
    // Review contracts-r1 (#2): bom và item cùng ID từng lọt qua.
    let w = world(&["0.."]);
    let mut s = with_ids(&w, 1);
    s.bombs = vec![bomb(1, Cell::new(1, 0), 0)];
    s.items = vec![FloorItem {
        id: EntityId(1),
        kind: ItemKind(0),
        cell: Cell::new(2, 0),
    }];
    assert!(violation(&w, &s).contains("hai entity"));
}

#[test]
fn invariants_check_references_against_world() {
    // Review contracts-r1 (#2): ô ngoài lưới, owner lạ, ItemKind lạ từng lọt qua.
    let w = world(&["0.#"]);
    let base = with_ids(&w, 1);

    let mut s = base.clone();
    s.bombs = vec![bomb(1, Cell::new(999, 999), 0)];
    assert!(violation(&w, &s).contains("không phải nền"));
    s.bombs = vec![bomb(1, Cell::new(2, 0), 0)];
    assert!(
        violation(&w, &s).contains("không phải nền"),
        "bom trên khối cứng"
    );
    s.bombs = vec![bomb(1, Cell::new(1, 0), 7)];
    assert!(violation(&w, &s).contains("owner 7"));
    let mut b = bomb(1, Cell::new(1, 0), 0);
    b.grace = vec![5];
    s.bombs = vec![b];
    assert!(violation(&w, &s).contains("grace"));

    let mut s = base.clone();
    s.items = vec![FloorItem {
        id: EntityId(1),
        kind: ItemKind(65535),
        cell: Cell::new(1, 0),
    }];
    assert!(violation(&w, &s).contains("ItemKind(65535)"));

    let mut s = base.clone();
    s.flames = vec![Flame {
        cell: Cell::new(1, 0),
        until: 5,
        source: EntityId(0),
        owner: 9,
    }];
    assert!(violation(&w, &s).contains("owner 9"));

    let mut s = base;
    s.players[0].x = Mpx(-1);
    assert!(violation(&w, &s).contains("ngoài lưới"));
}

#[test]
fn invariants_check_field_shape_before_cell_access() {
    // Review contracts-r2 (#2): lưới 3 ô chứa 2 ô từng trả Ok; chứa 0 ô thì panic.
    let w = world(&["0.."]);
    let mut s = solo(&w);
    for kinds in [vec![CellKind::Floor; 2], vec![]] {
        let bad = Terrain {
            width: 3,
            height: 1,
            kinds,
            spawns: vec![],
        };
        s.field = Field::from_terrain(&bad);
        assert!(violation(&w, &s).contains("cần 3 ô"));
    }
}

#[test]
fn world_rejects_inconsistent_terrain() {
    let ffa = mode_by_id("battle_ffa").unwrap();
    let make = |kinds: usize, spawns: Vec<(u16, u16)>| Terrain {
        width: 3,
        height: 1,
        kinds: vec![CellKind::Floor; kinds],
        spawns,
    };
    for t in [make(2, vec![]), make(0, vec![]), make(3, vec![(3, 0)])] {
        assert!(matches!(
            World::new(t, rules(), ffa),
            Err(CoreError::BadTerrain(_))
        ));
    }
    assert!(World::new(make(3, vec![(2, 0)]), rules(), ffa).is_ok());
}

#[test]
fn new_round_rejects_stats_above_cap() {
    // Review contracts-r1 (#2): speed 255 / trần 1 từng tạo được lượt.
    let w = world(&["0.."]);
    let mut bad = setup(0);
    bad.stats.speed = Stat::new(255, 1);
    assert_eq!(
        SimulationState::new_round(&w, &[bad], 0).err(),
        Some(CoreError::InvalidStat {
            player: 0,
            stat: "speed"
        })
    );
    let mut bad = setup(0);
    bad.stats.range = Stat::new(3, 2);
    assert!(matches!(
        SimulationState::new_round(&w, &[bad], 0),
        Err(CoreError::InvalidStat { stat: "range", .. })
    ));
}

#[test]
fn same_inputs_same_hash() {
    let w = world(&["0....", ".#.#.", "....1"]);
    let mut slow = setup(1);
    slow.stats.speed = Stat::new(4, 8);
    let roster = [setup(0), slow];
    let script = [Dir::Right, Dir::Down, Dir::Left, Dir::Up, Dir::None];
    let play = || {
        let mut s = SimulationState::new_round(&w, &roster, 3).unwrap();
        let mut events = Vec::new();
        for t in 0..300usize {
            let a = script[t / 37 % script.len()];
            let b = script[t / 23 % script.len()];
            s.step(&w, &[walk(a), walk(b)], &mut events).unwrap();
        }
        (s, events)
    };
    let (a, ea) = play();
    let (b, eb) = play();
    assert_eq!(a.hash(), b.hash());
    assert_eq!(ea, eb);
    assert_eq!(a.tick, 300);
}
