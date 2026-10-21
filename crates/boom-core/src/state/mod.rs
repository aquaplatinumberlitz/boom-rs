//! `SimulationState` theo contracts-r1 (ARCHITECTURE §5).
//!
//! Mỗi loại dữ liệu nằm trong file riêng và tự đóng góp vào state hash qua `hash_into`.
//! Field công khai là contract: thêm/đổi field công khai phải qua integrator; field riêng tư mà một
//! system cần thì thêm trong file của loại đó và nhớ đưa vào `hash_into` của nó.
//! Ngoại lệ đã biết: `blasts` là kênh **trong tick** (xem doc của field) nên không hash.
//! Ma trận ai được ghi field nào: ARCHITECTURE §5.

mod bomb;
mod field;
mod flame;
mod invariants;
mod item;
mod player;

pub use bomb::Bomb;
pub use field::Field;
pub use flame::Flame;
pub use item::{FloorItem, ItemKind};
pub use player::{CharacterStats, Life, Player, Stat};

use boom_content::hash::Fnv1a64;

use crate::mode::Outcome;
use crate::num::{Cell, Tick};
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
    /// ID sẽ cấp ở lần `next_id()` kế tiếp (contracts-r4, ADR 0009 §4.4).
    pub fn peek_next(&self) -> u32 {
        self.next
    }

    /// Dựng lại allocator từ `peek_next()` đã lưu. `None` khi `next == 0` (ID 0 không bao giờ được cấp).
    pub fn from_next(next: u32) -> Option<Self> {
        (next >= 1).then_some(Self { next })
    }

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
    /// Ô bị vụ nổ phủ — hàng chờ trong tick cho `blocks` phá khối mềm.
    ///
    /// BnB (`server/game.js:401–431`) phá gạch **ngay lúc nổ**, trước bước giảm TTL
    /// (`server/game.js:448–451`), nên gạch vỡ không phụ thuộc tuổi lửa: `flame_ticks = 1`
    /// (lửa không còn tick nào) vẫn phá gạch. V1 tách hai ý này: `flames` là **lửa còn sống**
    /// (sát thương + hình), `blasts` là **ô vừa bị nổ phủ** (phá gạch).
    ///
    /// `explosion` ghi (đặt lại danh sách ô của các bom vừa nổ), `blocks` đọc rồi xóa —
    /// cùng kiểu `movement` xóa `grace` (`systems/mod.rs`). Hàng chờ nên **rỗng ở mọi mốc
    /// tick**: state khôi phục từ snapshot giống hệt state gốc.
    ///
    /// Sắp theo ô, mỗi ô một lần.
    ///
    /// **Không nằm trong state hash** (`fnv1a64-v0` không đổi, ADR 0004 §5): kênh này rỗng ở
    /// mọi mốc tick nên thêm nó vào hash cũng không thêm thông tin nào, chỉ đổi encoding.
    pub blasts: Vec<Cell>,
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
}

fn hash_list<T>(h: &mut Fnv1a64, items: &[T], f: fn(&T, &mut Fnv1a64)) {
    h.u32(u32::try_from(items.len()).expect("danh sách quá dài"));
    for item in items {
        f(item, h);
    }
}
