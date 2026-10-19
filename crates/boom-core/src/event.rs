//! Event mô phỏng — danh sách V1 chốt trong contracts-r1.
//!
//! Mỗi system chỉ phát loại event thuộc phần mình (ARCHITECTURE §5). Thêm loại mới là thay đổi
//! contract: integrator thêm variant, `name()` và `fields()`; digest và harness scenario tự dùng hai
//! hàm đó nên không cần sửa thêm chỗ nào.

use boom_content::hash::Fnv1a64;

use crate::mode::Outcome;
use crate::num::{Cell, Tick};
use crate::state::{EntityId, ItemKind};

/// ID ổn định của event: `(tick, thứ tự trong tick)`. Cùng input → cùng ID, nên consumer
/// (âm thanh, thống kê, phần thưởng) dedup được khi replay/reconcile (ARCHITECTURE §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId {
    pub tick: Tick,
    pub seq: u16,
}

/// Loại event. `non_exhaustive`: consumer ngoài core phải bỏ qua loại nó không biết.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventKind {
    /// movement: người chơi đổi hướng nhìn.
    PlayerTurned { player: u8 },
    /// bomb_place: bom mới được đặt.
    BombPlaced {
        bomb: EntityId,
        player: u8,
        cell: Cell,
        range: u8,
    },
    /// explosion: bom nổ; `chained` khi bị lửa của bom khác kích nổ.
    BombExploded {
        bomb: EntityId,
        owner: u8,
        cell: Cell,
        chained: bool,
    },
    /// blocks: khối mềm vỡ.
    BlockDestroyed { cell: Cell },
    /// blocks: item lộ ra từ khối vỡ.
    ItemRevealed {
        item: EntityId,
        kind: ItemKind,
        cell: Cell,
    },
    /// blocks: item trên nền bị lửa phá (chỉ khi luật quyết định vậy).
    ItemDestroyed { item: EntityId, cell: Cell },
    /// pickup: người chơi nhặt item.
    ItemPicked {
        item: EntityId,
        kind: ItemKind,
        player: u8,
    },
    /// life: bị nhốt trong bong bóng.
    PlayerTrapped { player: u8, by: u8 },
    /// life: chết; `by` là người chạm làm vỡ bong bóng, `None` khi hết thời gian nhốt.
    PlayerDied { player: u8, by: Option<u8> },
    /// mode: lượt kết thúc.
    RoundEnded { outcome: Outcome },
}

impl EventKind {
    /// Tên ổn định (snake_case), dùng trong scenario, log và digest.
    pub fn name(&self) -> &'static str {
        match self {
            Self::PlayerTurned { .. } => "player_turned",
            Self::BombPlaced { .. } => "bomb_placed",
            Self::BombExploded { .. } => "bomb_exploded",
            Self::BlockDestroyed { .. } => "block_destroyed",
            Self::ItemRevealed { .. } => "item_revealed",
            Self::ItemDestroyed { .. } => "item_destroyed",
            Self::ItemPicked { .. } => "item_picked",
            Self::PlayerTrapped { .. } => "player_trapped",
            Self::PlayerDied { .. } => "player_died",
            Self::RoundEnded { .. } => "round_ended",
        }
    }

    /// Các trường dạng số nguyên, thứ tự cố định. Ô viết thành `x`, `y`; `None` thành -1;
    /// kết quả lượt thành `winner` (-1 là hòa).
    pub fn fields(&self) -> Vec<(&'static str, i64)> {
        let cell = |c: &Cell| [("x", i64::from(c.x)), ("y", i64::from(c.y))];
        let id = |e: &EntityId| i64::from(e.0);
        match self {
            Self::PlayerTurned { player } => vec![("player", i64::from(*player))],
            Self::BombPlaced {
                bomb,
                player,
                cell: c,
                range,
            } => [("bomb", id(bomb)), ("player", i64::from(*player))]
                .into_iter()
                .chain(cell(c))
                .chain([("range", i64::from(*range))])
                .collect(),
            Self::BombExploded {
                bomb,
                owner,
                cell: c,
                chained,
            } => [("bomb", id(bomb)), ("owner", i64::from(*owner))]
                .into_iter()
                .chain(cell(c))
                .chain([("chained", i64::from(*chained))])
                .collect(),
            Self::BlockDestroyed { cell: c } => cell(c).to_vec(),
            Self::ItemRevealed {
                item,
                kind,
                cell: c,
            } => [("item", id(item)), ("kind", i64::from(kind.0))]
                .into_iter()
                .chain(cell(c))
                .collect(),
            Self::ItemDestroyed { item, cell: c } => {
                [("item", id(item))].into_iter().chain(cell(c)).collect()
            }
            Self::ItemPicked { item, kind, player } => vec![
                ("item", id(item)),
                ("kind", i64::from(kind.0)),
                ("player", i64::from(*player)),
            ],
            Self::PlayerTrapped { player, by } => {
                vec![("player", i64::from(*player)), ("by", i64::from(*by))]
            }
            Self::PlayerDied { player, by } => vec![
                ("player", i64::from(*player)),
                ("by", by.map_or(-1, i64::from)),
            ],
            Self::RoundEnded { outcome } => vec![(
                "winner",
                match outcome {
                    Outcome::Draw => -1,
                    Outcome::Winner(p) => i64::from(*p),
                },
            )],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationEvent {
    pub id: EventId,
    pub kind: EventKind,
}

/// Nơi system ghi event trong một tick; cấp ID theo thứ tự ghi.
#[derive(Debug)]
pub struct EventSink<'a> {
    tick: Tick,
    next: u16,
    out: &'a mut Vec<SimulationEvent>,
}

impl<'a> EventSink<'a> {
    pub(crate) fn new(tick: Tick, out: &'a mut Vec<SimulationEvent>) -> Self {
        Self { tick, next: 0, out }
    }

    pub fn emit(&mut self, kind: EventKind) {
        let id = EventId {
            tick: self.tick,
            seq: self.next,
        };
        self.next = self
            .next
            .checked_add(1)
            .expect("quá nhiều event trong một tick");
        self.out.push(SimulationEvent { id, kind });
    }
}

/// Digest của chuỗi event trên encoding chuẩn, để golden replay bắt cả thay đổi event
/// (vd. âm thanh/hiệu ứng sẽ phát) chứ không chỉ state.
pub fn event_digest(events: &[SimulationEvent]) -> u64 {
    let mut h = Fnv1a64::default();
    h.u32(u32::try_from(events.len()).expect("quá nhiều event"));
    for e in events {
        h.u32(e.id.tick).u32(u32::from(e.id.seq)).str(e.kind.name());
        for (k, v) in e.kind.fields() {
            h.str(k).i64(v);
        }
    }
    h.finish()
}
