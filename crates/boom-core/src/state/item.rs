use boom_content::hash::Fnv1a64;

use crate::num::Cell;
use crate::state::EntityId;

/// Chỉ số trong `Rules::items` (V1: 0 Bomb, 1 Fire, 2 Roller).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemKind(pub u16);

/// Item đang nằm trên nền, nhặt được.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloorItem {
    pub id: EntityId,
    pub kind: ItemKind,
    pub cell: Cell,
}

impl FloorItem {
    pub(crate) fn hash_into(&self, h: &mut Fnv1a64) {
        h.u32(self.id.0)
            .u32(u32::from(self.kind.0))
            .i32(self.cell.x)
            .i32(self.cell.y);
    }
}
