use boom_content::hash::Fnv1a64;
use boom_content::map::{CellKind, Terrain};

use crate::num::Cell;
use crate::state::item::ItemKind;

/// Lưới đổi được trong lượt: loại ô (khối mềm vỡ thành nền) và item giấu trong khối mềm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    width: i32,
    height: i32,
    kinds: Vec<CellKind>,
    hidden: Vec<Option<ItemKind>>,
}

impl Field {
    pub fn from_terrain(t: &Terrain) -> Self {
        Self {
            width: i32::from(t.width),
            height: i32::from(t.height),
            kinds: t.kinds.clone(),
            hidden: vec![None; t.kinds.len()],
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    fn index(&self, c: Cell) -> Option<usize> {
        if (0..self.width).contains(&c.x) && (0..self.height).contains(&c.y) {
            usize::try_from(c.y * self.width + c.x).ok()
        } else {
            None
        }
    }

    /// `None` khi ngoài lưới.
    pub fn kind(&self, c: Cell) -> Option<CellKind> {
        self.index(c).and_then(|i| self.kinds.get(i).copied())
    }

    pub fn set_kind(&mut self, c: Cell, kind: CellKind) {
        let i = self.index(c).expect("ô ngoài lưới");
        self.kinds[i] = kind;
    }

    pub fn hidden(&self, c: Cell) -> Option<ItemKind> {
        self.index(c)
            .and_then(|i| self.hidden.get(i).copied().flatten())
    }

    pub fn set_hidden(&mut self, c: Cell, item: Option<ItemKind>) {
        let i = self.index(c).expect("ô ngoài lưới");
        self.hidden[i] = item;
    }

    /// Mọi ô theo thứ tự hàng rồi cột.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| Cell::new(x, y)))
    }

    pub(crate) fn hash_into(&self, h: &mut Fnv1a64) {
        h.i32(self.width).i32(self.height);
        for (kind, hidden) in self.kinds.iter().zip(&self.hidden) {
            h.u8(match kind {
                CellKind::Floor => 0,
                CellKind::Solid => 1,
                CellKind::Breakable => 2,
                CellKind::Unknown => 3,
            });
            match hidden {
                Some(k) => h.u8(1).u32(u32::from(k.0)),
                None => h.u8(0),
            };
        }
    }

    /// Số ô dữ liệu khớp kích thước khai báo. Phải gọi trước mọi truy cập ô khi kiểm state lạ.
    pub(crate) fn check_shape(&self) -> Result<(), String> {
        let expected = usize::try_from(self.width)
            .ok()
            .zip(usize::try_from(self.height).ok())
            .and_then(|(w, h)| w.checked_mul(h))
            .filter(|&n| n > 0)
            .ok_or_else(|| {
                format!(
                    "kích thước lưới {}x{} không hợp lệ",
                    self.width, self.height
                )
            })?;
        if self.kinds.len() != expected || self.hidden.len() != expected {
            return Err(format!(
                "lưới {}x{} cần {expected} ô, có {} loại ô và {} ô item giấu",
                self.width,
                self.height,
                self.kinds.len(),
                self.hidden.len()
            ));
        }
        Ok(())
    }

    /// `item_kinds` = số loại item trong `Rules::items`.
    pub(crate) fn check(&self, item_kinds: usize) -> Result<(), String> {
        for (i, (kind, hidden)) in self.kinds.iter().zip(&self.hidden).enumerate() {
            if *kind == CellKind::Unknown {
                return Err(format!("ô {i} Unknown trong state"));
            }
            if let Some(k) = hidden {
                if *kind != CellKind::Breakable {
                    return Err(format!("ô {i} giấu item nhưng không phải khối mềm"));
                }
                if usize::from(k.0) >= item_kinds {
                    return Err(format!("ô {i} giấu ItemKind({}) không có trong rules", k.0));
                }
            }
        }
        Ok(())
    }
}
