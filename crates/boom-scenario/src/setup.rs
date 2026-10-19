//! Định dạng dựng một trận từ TOML, dùng chung cho replay và scenario.
//!
//! ```toml
//! rules = "../../rules/skeleton-test.toml"   # tương đối file; scenario có mặc định
//! overlays = []
//! mode = "battle_ffa"
//! seed = 1
//! map = ["#####", "#0..#", "#####"]          # '#' cứng, '+' mềm, '.' nền, chữ số = spawn
//!
//! [[player]]
//! id = 0
//! speed = [5, 7]        # [giá trị đầu, trần]
//! bombs = [1, 10]
//! range = [1, 7]
//! cell = [1, 1]         # tùy chọn: đặt thẳng vào ô thay vì spawn theo seed
//! life = "alive"        # tùy chọn: alive | trapped | dead
//!
//! [[input]]
//! tick = 0
//! player = 0
//! dir = "right"         # giữ tới khi đổi; tùy chọn
//! bomb = true           # nhấn đặt bom đúng tick này; tùy chọn
//!
//! # Dựng sẵn entity để test một system mà không cần system khác
//! # (viết dạng [[bomb]] nhiều dòng hoặc mảng inline như dưới):
//! bomb = [{ cell = [2, 1], owner = 0, range = 2, fuse = 10, grace = [] }]  # fuse: tick nổ
//! item = [{ cell = [3, 1], kind = "Fire" }]
//! flame = [{ cell = [2, 1], ticks = 5, owner = 0 }]
//! hidden = [{ cell = [3, 2], kind = "Bomb" }]   # item giấu trong khối mềm
//! ```

use std::path::{Path, PathBuf};

use boom_content::map::{CellKind, Terrain};
use boom_content::rules::{RulesOverlay, RulesProfile};
use boom_core::{
    Bomb, Cell, CharacterStats, Dir, EntityId, Flame, FloorItem, ItemKind, Life, PlayerInput,
    PlayerSetup, Rules, SimulationEvent, SimulationState, Stat, World, mode_by_id,
};
use serde::Deserialize;

use crate::Error;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerEntry {
    pub id: u8,
    pub speed: [u8; 2],
    pub bombs: [u8; 2],
    pub range: [u8; 2],
    #[serde(default)]
    pub cell: Option<[i32; 2]>,
    #[serde(default)]
    pub life: Option<LifeName>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeName {
    Alive,
    Trapped,
    Dead,
}

impl LifeName {
    pub fn matches(self, life: Life) -> bool {
        matches!(
            (self, life),
            (Self::Alive, Life::Alive)
                | (Self::Trapped, Life::Trapped { .. })
                | (Self::Dead, Life::Dead)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirName {
    None,
    Up,
    Down,
    Left,
    Right,
}

impl From<DirName> for Dir {
    fn from(d: DirName) -> Self {
        match d {
            DirName::None => Dir::None,
            DirName::Up => Dir::Up,
            DirName::Down => Dir::Down,
            DirName::Left => Dir::Left,
            DirName::Right => Dir::Right,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputEntry {
    pub tick: u32,
    pub player: u8,
    #[serde(default)]
    pub dir: Option<DirName>,
    #[serde(default)]
    pub bomb: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BombEntry {
    pub cell: [i32; 2],
    pub owner: u8,
    pub range: u8,
    /// Số tick tới khi nổ, tính từ tick 0.
    pub fuse: u32,
    #[serde(default)]
    pub grace: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemEntry {
    pub cell: [i32; 2],
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlameEntry {
    pub cell: [i32; 2],
    pub ticks: u32,
    pub owner: u8,
}

/// Phần dựng trận chung của file replay/scenario.
#[derive(Clone, Debug, Deserialize)]
pub struct Setup {
    pub rules: Option<PathBuf>,
    #[serde(default)]
    pub overlays: Vec<PathBuf>,
    pub profile_hash: Option<String>,
    pub mode: Option<String>,
    pub seed: Option<u64>,
    pub map: Vec<String>,
    pub player: Vec<PlayerEntry>,
    #[serde(default)]
    pub input: Vec<InputEntry>,
    #[serde(default)]
    pub bomb: Vec<BombEntry>,
    #[serde(default)]
    pub item: Vec<ItemEntry>,
    #[serde(default)]
    pub flame: Vec<FlameEntry>,
    #[serde(default)]
    pub hidden: Vec<ItemEntry>,
}

/// Khóa gốc mà [`Setup`] dùng; file replay/scenario cộng thêm khóa riêng.
pub const SETUP_KEYS: &[&str] = &[
    "rules",
    "overlays",
    "profile_hash",
    "mode",
    "seed",
    "map",
    "player",
    "input",
    "bomb",
    "item",
    "flame",
    "hidden",
];

/// Profile cố định của fixtures, dùng khi scenario không ghi `rules`.
pub fn default_rules() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rules/skeleton-test.toml")
}

/// Parse TOML và từ chối khóa gốc lạ (serde không cho `deny_unknown_fields` cùng `flatten`).
pub fn parse_checked<T: serde::de::DeserializeOwned>(
    text: &str,
    extra_keys: &[&str],
) -> Result<T, Error> {
    let table: toml::Table = toml::from_str(text)?;
    for key in table.keys() {
        if !SETUP_KEYS.contains(&key.as_str()) && !extra_keys.contains(&key.as_str()) {
            return Err(format!("khóa lạ `{key}`").into());
        }
    }
    Ok(toml::from_str(text)?)
}

pub fn load_profile(setup: &Setup, base: &Path) -> Result<RulesProfile, Error> {
    let path = setup
        .rules
        .as_ref()
        .map_or_else(default_rules, |p| base.join(p));
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let mut profile = RulesProfile::from_toml(&read(&path)?)?;
    for overlay in &setup.overlays {
        profile = profile.with_overlay(&RulesOverlay::from_toml(&read(&base.join(overlay))?)?)?;
    }
    if let Some(expected) = &setup.profile_hash {
        let actual = format!("{:#018x}", profile.identity_hash());
        if *expected != actual {
            return Err(format!(
                "profile {} có định danh {actual}, file ghi {expected}; từ chối chạy với luật khác",
                profile.profile_id
            )
            .into());
        }
    }
    Ok(profile)
}

fn item_kind(rules: &Rules, name: &str) -> Result<ItemKind, Error> {
    let index = rules
        .items
        .iter()
        .position(|i| i.key == name)
        .ok_or_else(|| format!("item `{name}` không có trong rules"))?;
    Ok(ItemKind(u16::try_from(index)?))
}

fn cell(c: [i32; 2]) -> Cell {
    Cell::new(c[0], c[1])
}

/// Một trận đang chạy theo input của file.
pub struct Match {
    pub world: World,
    pub state: SimulationState,
    pub events: Vec<SimulationEvent>,
    inputs: Vec<InputEntry>,
    next_input: usize,
    held: Vec<PlayerInput>,
}

impl Match {
    pub fn new(setup: &Setup, base: &Path) -> Result<Self, Error> {
        let profile = load_profile(setup, base)?;
        let rules = Rules::from_profile(&profile)?;
        let mode_id = setup.mode.as_deref().unwrap_or("battle_ffa");
        let mode = mode_by_id(mode_id).ok_or_else(|| format!("mode `{mode_id}` chưa đăng ký"))?;
        let mut terrain = Terrain::from_ascii(&setup.map)?;
        // Người chơi có `cell` được đặt thẳng vào ô sau khi tạo lượt; bổ sung ô đó vào danh sách
        // spawn khi map không đủ chữ số spawn, để scenario không phải vẽ spawn giả.
        for p in &setup.player {
            if terrain.spawns.len() >= setup.player.len() {
                break;
            }
            if let Some([x, y]) = p.cell {
                let c = (u16::try_from(x)?, u16::try_from(y)?);
                if !terrain.spawns.contains(&c) {
                    terrain.spawns.push(c);
                }
            }
        }
        let world = World::new(terrain, rules, mode)?;
        let roster: Vec<PlayerSetup> = setup
            .player
            .iter()
            .map(|p| PlayerSetup {
                id: p.id,
                stats: CharacterStats {
                    speed: Stat::new(p.speed[0], p.speed[1]),
                    bombs: Stat::new(p.bombs[0], p.bombs[1]),
                    range: Stat::new(p.range[0], p.range[1]),
                },
            })
            .collect();
        let mut state = SimulationState::new_round(&world, &roster, setup.seed.unwrap_or(1))?;
        Self::apply_prestate(setup, &world, &mut state)?;
        state
            .check_invariants()
            .map_err(|e| format!("state dựng sẵn sai contract: {e}"))?;
        let mut inputs = setup.input.clone();
        inputs.sort_by_key(|i| i.tick);
        let held = vec![PlayerInput::default(); state.players.len()];
        Ok(Self {
            world,
            state,
            events: Vec::new(),
            inputs,
            next_input: 0,
            held,
        })
    }

    fn apply_prestate(setup: &Setup, world: &World, s: &mut SimulationState) -> Result<(), Error> {
        for p in &setup.player {
            let player = s
                .players
                .iter_mut()
                .find(|x| x.id == p.id)
                .expect("người chơi vừa tạo");
            if let Some(c) = p.cell {
                let c = cell(c);
                if s.field.kind(c) != Some(CellKind::Floor) {
                    return Err(
                        format!("người chơi {} đặt vào ô không phải nền {c:?}", p.id).into(),
                    );
                }
                (player.x, player.y) = world.center_of(c);
            }
            player.life = match p.life {
                None | Some(LifeName::Alive) => player.life,
                Some(LifeName::Trapped) => Life::Trapped { since: 0 },
                Some(LifeName::Dead) => Life::Dead,
            };
        }
        for b in &setup.bomb {
            let id = s.ids.next_id();
            s.bombs.push(Bomb {
                id,
                owner: b.owner,
                cell: cell(b.cell),
                range: b.range,
                explodes_at: b.fuse,
                grace: b.grace.clone(),
            });
        }
        for i in &setup.item {
            let id = s.ids.next_id();
            s.items.push(FloorItem {
                id,
                kind: item_kind(world.rules(), &i.kind)?,
                cell: cell(i.cell),
            });
        }
        let mut flames: Vec<Flame> = setup
            .flame
            .iter()
            .map(|f| Flame {
                cell: cell(f.cell),
                until: f.ticks.saturating_sub(1),
                source: EntityId(0),
                owner: f.owner,
            })
            .collect();
        flames.sort_by_key(|f| f.cell);
        s.flames = flames;
        for h in &setup.hidden {
            let c = cell(h.cell);
            if s.field.kind(c) != Some(CellKind::Breakable) {
                return Err(format!("item giấu ở ô không phải khối mềm {c:?}").into());
            }
            s.field
                .set_hidden(c, Some(item_kind(world.rules(), &h.kind)?));
        }
        Ok(())
    }

    /// Chạy một tick với input của file; kiểm bất biến contract sau tick.
    pub fn step(&mut self) -> Result<(), Error> {
        let tick = self.state.tick;
        for h in &mut self.held {
            h.place_bomb = false;
        }
        while let Some(i) = self.inputs.get(self.next_input).filter(|i| i.tick <= tick) {
            if i.tick < tick {
                return Err(format!("input ở tick {} nằm sau khi trận dừng", i.tick).into());
            }
            let idx = self
                .state
                .players
                .iter()
                .position(|p| p.id == i.player)
                .ok_or_else(|| format!("input cho người chơi lạ {}", i.player))?;
            if let Some(d) = i.dir {
                self.held[idx].dir = d.into();
            }
            self.held[idx].place_bomb |= i.bomb;
            self.next_input += 1;
        }
        self.state.step(&self.world, &self.held, &mut self.events)?;
        self.state
            .check_invariants()
            .map_err(|e| format!("tick {tick}: vi phạm contract: {e}"))?;
        Ok(())
    }

    /// Input chưa dùng (vd. ghi cho tick sau khi lượt đã kết thúc).
    pub fn unused_input(&self) -> Option<&InputEntry> {
        self.inputs.get(self.next_input)
    }
}
