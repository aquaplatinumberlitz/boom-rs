use boom_content::hash::Fnv1a64;

use crate::num::{Cell, Tick};
use crate::state::EntityId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bomb {
    pub id: EntityId,
    pub owner: u8,
    pub cell: Cell,
    /// Tầm nổ chụp lúc đặt (RULES_V1 R-032).
    pub range: u8,
    pub explodes_at: Tick,
    /// Người chơi đang đứng chồng lên bom và được phép đi ra (grace, SCOPE_V1 §4); sắp tăng dần.
    /// Movement xóa id khi người đó rời ô; không ai thêm lại.
    pub grace: Vec<u8>,
}

impl Bomb {
    pub(crate) fn hash_into(&self, h: &mut Fnv1a64) {
        h.u32(self.id.0)
            .u8(self.owner)
            .i32(self.cell.x)
            .i32(self.cell.y)
            .u8(self.range)
            .u32(self.explodes_at);
        h.u32(u32::try_from(self.grace.len()).expect("ít người chơi"));
        for &p in &self.grace {
            h.u8(p);
        }
    }
}
