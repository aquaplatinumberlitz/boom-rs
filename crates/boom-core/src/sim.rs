//! World (dữ liệu bất biến của lượt), input và vòng `step` (contracts-r4).

use std::fmt;

use boom_content::map::{CellKind, Terrain};

use crate::event::{EventKind, EventSink, SimulationEvent};
use crate::mode::Mode;
use crate::num::{Cell, Mpx};
use crate::rng::Rng;
use crate::rules::Rules;
use crate::state::{CharacterStats, Field, IdAllocator, Life, Phase, Player, SimulationState};

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
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::None => (0, 0),
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    pub(crate) fn code(self) -> u8 {
        match self {
            Dir::None => 0,
            Dir::Up => 1,
            Dir::Down => 2,
            Dir::Left => 3,
            Dir::Right => 4,
        }
    }
}

/// Input đã được authority chấp nhận cho một người chơi trong một tick (contracts-r4).
/// Authority chuyển `InputCommand` mạng thành giá trị này; input neutral là `default()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerInput {
    /// Hướng đang giữ (authority đã giải quyết ưu tiên khi giữ nhiều phím, V1-011).
    pub dir: Dir,
    /// Cạnh nhấn nút đặt bom trong tick này (không phải trạng thái giữ).
    pub place_bomb: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSetup {
    pub id: u8,
    pub stats: CharacterStats,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CoreError {
    UnknownCells,
    /// Địa hình không tự nhất quán (số ô khác kích thước, spawn ngoài lưới).
    BadTerrain(String),
    EmptyRoster,
    DuplicatePlayer(u8),
    TooManyPlayers {
        players: usize,
        spawns: usize,
    },
    SpawnBlocked(u16, u16),
    SpeedTooHigh(u8),
    InvalidStat {
        player: u8,
        stat: &'static str,
    },
    InputCount {
        expected: usize,
        found: usize,
    },
    RoundOver,
    /// State dựng lượt vi phạm bất biến contract (lỗi trong system hoặc dữ liệu đầu vào).
    Contract(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCells => write!(f, "địa hình còn ô Unknown; cần bảng phân loại ô (Q-002)"),
            Self::BadTerrain(e) => write!(f, "địa hình sai: {e}"),
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
            Self::InvalidStat { player, stat } => {
                write!(f, "người chơi {player}: chỉ số {stat} có giá trị vượt trần")
            }
            Self::Contract(e) => write!(f, "vi phạm contract khi dựng lượt: {e}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Dữ liệu bất biến của một lượt: địa hình gốc đã phân loại, luật và mode.
#[derive(Clone, Debug)]
pub struct World {
    terrain: Terrain,
    rules: Rules,
    mode: &'static dyn Mode,
}

impl World {
    /// Từ chối địa hình không nhất quán hoặc còn ô Unknown thay vì đoán (ARCHITECTURE §4).
    pub fn new(terrain: Terrain, rules: Rules, mode: &'static dyn Mode) -> Result<Self, CoreError> {
        let cells = usize::from(terrain.width) * usize::from(terrain.height);
        if cells == 0 || terrain.kinds.len() != cells {
            return Err(CoreError::BadTerrain(format!(
                "{}x{} cần {cells} ô, có {}",
                terrain.width,
                terrain.height,
                terrain.kinds.len()
            )));
        }
        if let Some(&(x, y)) = terrain
            .spawns
            .iter()
            .find(|&&(x, y)| x >= terrain.width || y >= terrain.height)
        {
            return Err(CoreError::BadTerrain(format!("spawn ({x},{y}) ngoài lưới")));
        }
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

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    /// Chỉ số ô chứa tọa độ trên một trục.
    pub fn cell_index(&self, p: Mpx) -> i32 {
        p.0.div_euclid(self.rules.tile.0)
    }

    /// Ô chứa điểm `(x, y)`.
    pub fn cell_at(&self, x: Mpx, y: Mpx) -> Cell {
        Cell::new(self.cell_index(x), self.cell_index(y))
    }

    /// Tọa độ tâm của chỉ số ô trên một trục.
    pub fn center(&self, index: i32) -> Mpx {
        Mpx(index * self.rules.tile.0 + self.rules.tile.0 / 2)
    }

    /// Tâm ô.
    pub fn center_of(&self, c: Cell) -> (Mpx, Mpx) {
        (self.center(c.x), self.center(c.y))
    }

    /// Tốc độ (mpx/tick) của một cấp.
    pub fn speed(&self, level: u8) -> Mpx {
        Mpx(self.rules.speed_per_level.0 * i32::from(level))
    }
}

impl SimulationState {
    /// Chốt đầu lượt (contracts-r4):
    /// 1. Lưới đổi được lấy từ địa hình gốc.
    /// 2. Hoán vị spawn theo seed, N người lấy N spawn đầu (SCOPE_V1 §3, DESIGN).
    /// 3. `on_round_start` của từng system theo thứ tự mode (vd. giấu item vào khối mềm).
    ///
    /// RNG dùng theo đúng thứ tự trên, nên đổi thứ tự là đổi kết quả của cùng seed.
    /// Từ chối chỉ số vượt trần; state trả về đã qua `check_invariants`.
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
        let field = Field::from_terrain(&world.terrain);
        let mut players = Vec::with_capacity(setups.len());
        for (setup, &spawn) in setups.iter().zip(&order) {
            let (sx, sy) = spawns[spawn];
            let cell = Cell::new(i32::from(sx), i32::from(sy));
            if field.kind(cell) != Some(CellKind::Floor) {
                return Err(CoreError::SpawnBlocked(sx, sy));
            }
            let stats = setup.stats;
            for (stat, s) in [
                ("speed", stats.speed),
                ("bombs", stats.bombs),
                ("range", stats.range),
            ] {
                if s.value > s.max {
                    return Err(CoreError::InvalidStat {
                        player: setup.id,
                        stat,
                    });
                }
            }
            if world.speed(setup.stats.speed.max).0 * 2 >= world.rules.tile.0 {
                return Err(CoreError::SpeedTooHigh(setup.stats.speed.max));
            }
            let (x, y) = world.center_of(cell);
            players.push(Player {
                id: setup.id,
                x,
                y,
                facing: Dir::Down,
                speed: setup.stats.speed,
                bombs: setup.stats.bombs,
                range: setup.stats.range,
                life: Life::Alive,
            });
        }
        let mut state = Self {
            tick: 0,
            phase: Phase::Playing,
            rng,
            ids: IdAllocator::default(),
            players,
            field,
            bombs: Vec::new(),
            flames: Vec::new(),
            items: Vec::new(),
        };
        for system in world.mode.systems() {
            system.on_round_start(world, &mut state);
        }
        state.check_invariants(world).map_err(CoreError::Contract)?;
        Ok(state)
    }

    /// Một tick: chạy các system của mode theo thứ tự, tăng tick, rồi xét kết quả.
    /// `inputs[i]` là input của `players[i]`; event của tick được nối vào `events`.
    ///
    /// **Warmup** (R-100): `round.warmup_ms` tick đầu của lượt bị đóng băng — không system nào
    /// chạy (không movement, không đặt bom, không timer) và đồng hồ lượt chưa chạy, nên thời hạn
    /// lượt đo từ đầu lượt thật (BnB `server/game.js:616–646`). Tick engine vẫn tăng để client
    /// và authority đồng bộ; `mode::outcome` lấy đồng hồ lượt = tick − warmup.
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
        if self.tick < world.rules().warmup_ticks {
            self.tick += 1;
            return Ok(());
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
}

#[cfg(test)]
mod tests;
