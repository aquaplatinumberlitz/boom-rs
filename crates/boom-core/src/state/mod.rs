//! `SimulationState` theo contracts-r1 (ARCHITECTURE §5).
//!
//! Mỗi loại dữ liệu nằm trong file riêng và tự đóng góp vào state hash qua `hash_into`.
//! Field công khai là contract: thêm/đổi field công khai phải qua integrator; field riêng tư mà một
//! system cần thì thêm trong file của loại đó và nhớ đưa vào `hash_into` của nó.
//! Ma trận ai được ghi field nào: ARCHITECTURE §5.

mod bomb;
mod field;
mod flame;
mod item;
mod player;

pub use bomb::Bomb;
pub use field::Field;
pub use flame::Flame;
pub use item::{FloorItem, ItemKind};
pub use player::{CharacterStats, Life, Player, Stat};

use boom_content::hash::Fnv1a64;

use crate::mode::Outcome;
use crate::num::Tick;
use crate::rng::Rng;

/// ID entity ổn định trong một lượt (bom, item…), cấp tăng dần từ 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdAllocator {
    next: u32,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl IdAllocator {
    pub fn next_id(&mut self) -> EntityId {
        let id = EntityId(self.next);
        self.next = self.next.checked_add(1).expect("hết ID entity");
        id
    }
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
    pub ids: IdAllocator,
    /// Sắp theo `id`; thứ tự này là thứ tự cập nhật trong mọi system.
    pub players: Vec<Player>,
    pub field: Field,
    /// Sắp theo `id` (cũng là thứ tự đặt).
    pub bombs: Vec<Bomb>,
    /// Sắp theo ô; mỗi ô tối đa một flame.
    pub flames: Vec<Flame>,
    /// Sắp theo `id`; mỗi ô tối đa một item.
    pub items: Vec<FloorItem>,
}

impl SimulationState {
    pub fn player(&self, id: u8) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    /// State hash `fnv1a64-v0` trên encoding chuẩn (ADR 0004). Thứ tự khối cố định.
    pub fn hash(&self) -> u64 {
        let mut h = Fnv1a64::default();
        h.u32(self.tick);
        match self.phase {
            Phase::Playing => h.u8(0),
            Phase::Ended(Outcome::Draw) => h.u8(1),
            Phase::Ended(Outcome::Winner(id)) => h.u8(2).u8(id),
        };
        self.rng.hash_into(&mut h);
        h.u32(self.ids.next);
        hash_list(&mut h, &self.players, Player::hash_into);
        self.field.hash_into(&mut h);
        hash_list(&mut h, &self.bombs, Bomb::hash_into);
        hash_list(&mut h, &self.flames, Flame::hash_into);
        hash_list(&mut h, &self.items, FloorItem::hash_into);
        h.finish()
    }

    /// Kiểm bất biến của contract; harness scenario gọi sau mỗi tick. Trả lỗi đầu tiên tìm thấy.
    pub fn check_invariants(&self) -> Result<(), String> {
        if !self.players.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("players phải sắp theo id, không trùng".into());
        }
        for p in &self.players {
            p.check()?;
        }
        if !self.bombs.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("bombs phải sắp theo id".into());
        }
        let mut cells: Vec<_> = self.bombs.iter().map(|b| b.cell).collect();
        cells.sort();
        if cells.windows(2).any(|w| w[0] == w[1]) {
            return Err("hai bom cùng một ô".into());
        }
        if !self.flames.windows(2).all(|w| w[0].cell < w[1].cell) {
            return Err("flames phải sắp theo ô, mỗi ô một flame".into());
        }
        if !self.items.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("items phải sắp theo id".into());
        }
        let mut cells: Vec<_> = self.items.iter().map(|i| i.cell).collect();
        cells.sort();
        if cells.windows(2).any(|w| w[0] == w[1]) {
            return Err("hai item cùng một ô".into());
        }
        let max_id = self.ids.next;
        let ids = self
            .bombs
            .iter()
            .map(|b| b.id)
            .chain(self.items.iter().map(|i| i.id));
        if let Some(id) = ids.into_iter().find(|id| id.0 == 0 || id.0 >= max_id) {
            return Err(format!("entity {id:?} không do IdAllocator cấp"));
        }
        self.field.check()
    }
}

fn hash_list<T>(h: &mut Fnv1a64, items: &[T], f: fn(&T, &mut Fnv1a64)) {
    h.u32(u32::try_from(items.len()).expect("danh sách quá dài"));
    for item in items {
        f(item, h);
    }
}
