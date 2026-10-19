use boom_content::hash::Fnv1a64;

use crate::num::{Cell, Tick};
use crate::state::EntityId;

/// Một ô đang có lửa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flame {
    pub cell: Cell,
    /// Tick cuối cùng ô còn lửa (bao gồm).
    pub until: Tick,
    /// Bom gây ra lửa này (bom gần nhất nếu nhiều bom chồng lửa).
    pub source: EntityId,
    pub owner: u8,
}

impl Flame {
    pub(crate) fn hash_into(&self, h: &mut Fnv1a64) {
        h.i32(self.cell.x)
            .i32(self.cell.y)
            .u32(self.until)
            .u32(self.source.0)
            .u8(self.owner);
    }
}
