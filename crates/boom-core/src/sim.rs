use std::fmt;

use boom_content::hash::Fnv1a64;
use boom_content::map::{CellKind, Terrain};

use crate::event::{EventKind, EventSink, SimulationEvent};
use crate::mode::{Mode, Outcome};
use crate::num::{Mpx, Tick};
use crate::rng::Rng;
use crate::rules::Rules;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dir {
    #[default]
    None,
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub(crate) fn delta(self) -> (i32, i32) {
        match self {
            Dir::None => (0, 0),
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    fn code(self) -> u8 {
        match self {
            Dir::None => 0,
            Dir::Up => 1,
            Dir::Down => 2,
            Dir::Left => 3,
            Dir::Right => 4,
        }
    }
}

/// Input đã được authority chấp nhận cho một người chơi trong một tick.
/// Thêm field (vd. cạnh nhấn bom) là thay đổi contract, phải tăng revision replay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerInput {
    pub dir: Dir,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSetup {
    pub id: u8,
    /// Cấp tốc độ ban đầu (BomberAttr f2 theo RULES_V1 R-042, đang là candidate).
    pub speed_level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub id: u8,
    /// Tâm nhân vật.
    pub x: Mpx,
    pub y: Mpx,
    pub speed_level: u8,
    pub facing: Dir,
    /// `false` sau khi chết; system nhốt/chết (V1-020) sẽ đặt.
    pub alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Ended(Outcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationState {
    pub tick: Tick,
    pub phase: Phase,
    pub rng: Rng,
    /// Sắp theo `id` tăng dần; thứ tự này là thứ tự cập nhật.
    pub players: Vec<Player>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CoreError {
    UnknownCells,
    EmptyRoster,
    DuplicatePlayer(u8),
    TooManyPlayers { players: usize, spawns: usize },
    SpawnBlocked(u16, u16),
    SpeedTooHigh(u8),
    InputCount { expected: usize, found: usize },
    RoundOver,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCells => write!(f, "địa hình còn ô Unknown; cần bảng phân loại ô (Q-002)"),
            Self::EmptyRoster => write!(f, "không có người chơi"),
            Self::DuplicatePlayer(id) => write!(f, "người chơi {id} trùng"),
            Self::TooManyPlayers { players, spawns } => {
                write!(f, "{players} người chơi nhưng chỉ có {spawns} spawn")
            }
            Self::SpawnBlocked(x, y) => write!(f, "spawn ({x},{y}) không phải ô nền"),
            Self::SpeedTooHigh(l) => write!(f, "cấp tốc độ {l} vượt nửa ô mỗi tick"),
            Self::InputCount { expected, found } => {
                write!(f, "cần {expected} input, nhận {found}")
            }
            Self::RoundOver => write!(f, "lượt đã kết thúc"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Dữ liệu bất biến của một lượt: địa hình đã phân loại, luật và mode.
#[derive(Clone, Debug)]
pub struct World {
    terrain: Terrain,
    rules: Rules,
    mode: &'static dyn Mode,
}

impl World {
    /// Từ chối địa hình còn ô Unknown thay vì đoán (ARCHITECTURE §4).
    pub fn new(terrain: Terrain, rules: Rules, mode: &'static dyn Mode) -> Result<Self, CoreError> {
        if terrain.has_unknown() {
            return Err(CoreError::UnknownCells);
        }
        Ok(Self {
            terrain,
            rules,
            mode,
        })
    }

    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    pub fn mode(&self) -> &'static dyn Mode {
        self.mode
    }

    pub(crate) fn walkable(&self, cx: i32, cy: i32) -> bool {
        // Ngoài lưới coi như bị chặn; khối mềm chặn như khối cứng.
        self.terrain.kind(cx, cy) == Some(CellKind::Floor)
    }

    pub(crate) fn cell_of(&self, p: Mpx) -> i32 {
        p.0.div_euclid(self.rules.tile.0)
    }

    pub(crate) fn center(&self, cell: i32) -> Mpx {
        Mpx(cell * self.rules.tile.0 + self.rules.tile.0 / 2)
    }

    pub(crate) fn speed(&self, level: u8) -> Mpx {
        Mpx(self.rules.speed_per_level.0 * i32::from(level))
    }
}

impl SimulationState {
    /// Chốt vị trí đầu lượt: hoán vị spawn theo seed, N người lấy N spawn đầu (SCOPE_V1 §3, DESIGN).
    pub fn new_round(world: &World, roster: &[PlayerSetup], seed: u64) -> Result<Self, CoreError> {
        if roster.is_empty() {
            return Err(CoreError::EmptyRoster);
        }
        let mut setups = roster.to_vec();
        setups.sort_by_key(|s| s.id);
        if let Some(w) = setups.windows(2).find(|w| w[0].id == w[1].id) {
            return Err(CoreError::DuplicatePlayer(w[0].id));
        }
        let spawns = &world.terrain.spawns;
        if setups.len() > spawns.len() {
            return Err(CoreError::TooManyPlayers {
                players: setups.len(),
                spawns: spawns.len(),
            });
        }
        let mut rng = Rng::new(seed);
        let mut order: Vec<usize> = (0..spawns.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rng.below(u32::try_from(i + 1).expect("ít spawn")) as usize;
            order.swap(i, j);
        }
        let mut players = Vec::with_capacity(setups.len());
        for (setup, &spawn) in setups.iter().zip(&order) {
            let (sx, sy) = spawns[spawn];
            if !world.walkable(i32::from(sx), i32::from(sy)) {
                return Err(CoreError::SpawnBlocked(sx, sy));
            }
            if world.speed(setup.speed_level).0 * 2 >= world.rules.tile.0 {
                return Err(CoreError::SpeedTooHigh(setup.speed_level));
            }
            players.push(Player {
                id: setup.id,
                x: world.center(i32::from(sx)),
                y: world.center(i32::from(sy)),
                speed_level: setup.speed_level,
                facing: Dir::Down,
                alive: true,
            });
        }
        Ok(Self {
            tick: 0,
            phase: Phase::Playing,
            rng,
            players,
        })
    }

    /// Một tick: chạy các system của mode theo thứ tự, rồi xét kết quả.
    /// `inputs[i]` là input của `players[i]`; event của tick được nối vào `events`.
    pub fn step(
        &mut self,
        world: &World,
        inputs: &[PlayerInput],
        events: &mut Vec<SimulationEvent>,
    ) -> Result<(), CoreError> {
        if self.phase != Phase::Playing {
            return Err(CoreError::RoundOver);
        }
        if inputs.len() != self.players.len() {
            return Err(CoreError::InputCount {
                expected: self.players.len(),
                found: inputs.len(),
            });
        }
        let mut sink = EventSink::new(self.tick, events);
        for system in world.mode.systems() {
            system.run(world, self, inputs, &mut sink);
        }
        self.tick += 1;
        if let Some(outcome) = world.mode.outcome(world, self) {
            self.phase = Phase::Ended(outcome);
            sink.emit(EventKind::RoundEnded { outcome });
        }
        Ok(())
    }

    /// State hash `fnv1a64-v0` trên encoding chuẩn (ADR 0004).
    pub fn hash(&self) -> u64 {
        let mut h = Fnv1a64::default();
        h.u32(self.tick);
        match self.phase {
            Phase::Playing => h.u8(0),
            Phase::Ended(Outcome::Draw) => h.u8(1),
            Phase::Ended(Outcome::Winner(id)) => h.u8(2).u8(id),
        };
        self.rng.hash_into(&mut h);
        h.u32(u32::try_from(self.players.len()).expect("ít người chơi"));
        for p in &self.players {
            h.u8(p.id)
                .i32(p.x.0)
                .i32(p.y.0)
                .u8(p.speed_level)
                .u8(p.facing.code())
                .u8(u8::from(p.alive));
        }
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventId;
    use crate::mode::mode_by_id;
    use boom_content::rules::RulesProfile;

    const PROFILE: &str = include_str!("../../../fixtures/rules/skeleton-test.toml");

    fn rules() -> Rules {
        Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
    }

    fn world(rows: &[&str]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules(), ffa).unwrap()
    }

    fn solo(w: &World) -> SimulationState {
        SimulationState::new_round(
            w,
            &[PlayerSetup {
                id: 0,
                speed_level: 5,
            }],
            1,
        )
        .unwrap()
    }

    fn run(w: &World, s: &mut SimulationState, dir: Dir, ticks: u32) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        for _ in 0..ticks {
            s.step(w, &[PlayerInput { dir }], &mut events).unwrap();
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
        let roster = [0, 1].map(|id| PlayerSetup { id, speed_level: 5 });
        let mut s = SimulationState::new_round(&w, &roster, 0).unwrap();
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
    fn last_player_alive_wins() {
        let w = world(&["0.1"]);
        let roster = [0, 1].map(|id| PlayerSetup { id, speed_level: 5 });
        let mut s = SimulationState::new_round(&w, &roster, 0).unwrap();
        s.players[1].alive = false; // thay cho system nhốt/chết chưa có
        let mut events = Vec::new();
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
        let p = |id| PlayerSetup { id, speed_level: 5 };
        assert_eq!(
            SimulationState::new_round(&w, &[p(1), p(1)], 0).err(),
            Some(CoreError::DuplicatePlayer(1))
        );
        assert_eq!(
            SimulationState::new_round(&w, &[p(0), p(1), p(2)], 0).err(),
            Some(CoreError::TooManyPlayers {
                players: 3,
                spawns: 2
            })
        );
        assert_eq!(
            SimulationState::new_round(
                &w,
                &[PlayerSetup {
                    id: 0,
                    speed_level: 60
                }],
                0
            )
            .err(),
            Some(CoreError::SpeedTooHigh(60))
        );
    }

    #[test]
    fn spawn_assignment_depends_on_seed_only() {
        let w = world(&["0.1", "...", "2.3"]);
        let roster: Vec<_> = (0..4)
            .map(|id| PlayerSetup { id, speed_level: 5 })
            .collect();
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

    #[test]
    fn same_inputs_same_hash() {
        let w = world(&["0....", ".#.#.", "....1"]);
        let roster = [
            PlayerSetup {
                id: 0,
                speed_level: 5,
            },
            PlayerSetup {
                id: 1,
                speed_level: 4,
            },
        ];
        let script = [Dir::Right, Dir::Down, Dir::Left, Dir::Up, Dir::None];
        let play = || {
            let mut s = SimulationState::new_round(&w, &roster, 3).unwrap();
            let mut events = Vec::new();
            for t in 0..300usize {
                let a = script[t / 37 % script.len()];
                let b = script[t / 23 % script.len()];
                s.step(
                    &w,
                    &[PlayerInput { dir: a }, PlayerInput { dir: b }],
                    &mut events,
                )
                .unwrap();
            }
            (s, events)
        };
        let (a, ea) = play();
        let (b, eb) = play();
        assert_eq!(a.hash(), b.hash());
        assert_eq!(ea, eb);
        assert_eq!(a.tick, 300);
    }
}
