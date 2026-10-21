//! Bất biến của contract (contracts-r3). `new_round` kiểm sau khi dựng lượt; harness scenario và replay
//! kiểm sau mỗi tick. Mọi tham chiếu (ô, người chơi, loại item, ID entity) được đối chiếu với `World`.

use boom_content::map::CellKind;

use crate::sim::World;
use crate::state::{EntityId, SimulationState};

impl SimulationState {
    /// Trả lỗi đầu tiên tìm thấy. Không panic với state sai, kể cả tham chiếu ngoài miền.
    pub fn check_invariants(&self, world: &World) -> Result<(), String> {
        let field = &self.field;
        // Độ dài dữ liệu trước mọi truy cập ô (review #2, r3).
        field.check_shape()?;
        let terrain = world.terrain();
        if (field.width(), field.height()) != (i32::from(terrain.width), i32::from(terrain.height))
        {
            return Err("kích thước lưới khác địa hình của World".into());
        }
        let item_kinds = world.rules().items.len();

        if !self.players.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("players phải sắp theo id, không trùng".into());
        }
        for p in &self.players {
            p.check()?;
            let c = world.cell_at(p.x, p.y);
            if field.kind(c).is_none() {
                return Err(format!(
                    "người chơi {} ở ngoài lưới ({},{})",
                    p.id, c.x, c.y
                ));
            }
        }
        let player_exists = |id: u8| self.players.iter().any(|p| p.id == id);

        if !self.bombs.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("bombs phải sắp theo id".into());
        }
        for b in &self.bombs {
            if field.kind(b.cell) != Some(CellKind::Floor) {
                return Err(format!("bom {:?} ở ô không phải nền {:?}", b.id, b.cell));
            }
            if !player_exists(b.owner) {
                return Err(format!("bom {:?} có owner {} không tồn tại", b.id, b.owner));
            }
            if !b.grace.windows(2).all(|w| w[0] < w[1]) {
                return Err(format!("bom {:?}: grace phải sắp tăng, không trùng", b.id));
            }
            if let Some(g) = b.grace.iter().find(|&&g| !player_exists(g)) {
                return Err(format!(
                    "bom {:?}: grace có người chơi {g} không tồn tại",
                    b.id
                ));
            }
        }
        no_duplicate_cells(self.bombs.iter().map(|b| b.cell), "hai bom cùng một ô")?;

        if !self.flames.windows(2).all(|w| w[0].cell < w[1].cell) {
            return Err("flames phải sắp theo ô, mỗi ô một flame".into());
        }
        for f in &self.flames {
            if !matches!(
                field.kind(f.cell),
                Some(CellKind::Floor | CellKind::Breakable)
            ) {
                return Err(format!("lửa ở ô không phải nền/khối mềm {:?}", f.cell));
            }
            if !player_exists(f.owner) {
                return Err(format!(
                    "lửa ở {:?} có owner {} không tồn tại",
                    f.cell, f.owner
                ));
            }
            if f.source.0 >= self.ids.next {
                return Err(format!(
                    "lửa ở {:?} trỏ bom {:?} chưa được cấp",
                    f.cell, f.source
                ));
            }
        }

        if !self.blasts.windows(2).all(|w| w[0] < w[1]) {
            return Err("blasts phải sắp theo ô, mỗi ô một lần".into());
        }
        for c in &self.blasts {
            if field.kind(*c).is_none() {
                return Err(format!("blast ở ô ngoài lưới {c:?}"));
            }
        }

        if !self.items.windows(2).all(|w| w[0].id < w[1].id) {
            return Err("items phải sắp theo id".into());
        }
        for i in &self.items {
            if field.kind(i.cell) != Some(CellKind::Floor) {
                return Err(format!("item {:?} ở ô không phải nền {:?}", i.id, i.cell));
            }
            if usize::from(i.kind.0) >= item_kinds {
                return Err(format!(
                    "item {:?} có ItemKind({}) không có trong rules",
                    i.id, i.kind.0
                ));
            }
        }
        no_duplicate_cells(self.items.iter().map(|i| i.cell), "hai item cùng một ô")?;

        // ID entity duy nhất trên mọi loại, và do allocator cấp.
        let mut ids: Vec<EntityId> = self
            .bombs
            .iter()
            .map(|b| b.id)
            .chain(self.items.iter().map(|i| i.id))
            .collect();
        ids.sort();
        if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
            return Err(format!("ID entity {:?} bị dùng cho hai entity", w[0]));
        }
        if let Some(id) = ids.iter().find(|id| id.0 == 0 || id.0 >= self.ids.next) {
            return Err(format!("entity {id:?} không do IdAllocator cấp"));
        }

        field.check(item_kinds)
    }
}

fn no_duplicate_cells(
    cells: impl Iterator<Item = crate::num::Cell>,
    msg: &str,
) -> Result<(), String> {
    let mut cells: Vec<_> = cells.collect();
    cells.sort();
    if cells.windows(2).any(|w| w[0] == w[1]) {
        return Err(msg.into());
    }
    Ok(())
}
