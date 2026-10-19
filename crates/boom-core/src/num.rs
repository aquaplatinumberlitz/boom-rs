/// Bước mô phỏng; tần số lấy từ `sim.ticks_per_second`.
pub type Tick = u32;

/// Tọa độ/độ dài theo 1/1000 pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mpx(pub i32);

impl Mpx {
    pub const PER_PX: i32 = 1000;

    pub const fn from_px(px: i32) -> Self {
        Self(px * Self::PER_PX)
    }
}

/// Ô lưới `(x = cột, y = hàng)`. Thứ tự sắp xếp: theo hàng rồi cột, dùng khi cần duyệt ổn định.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Ô kề theo hướng `(dx, dy)`.
    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.y, self.x).cmp(&(other.y, other.x))
    }
}

/// Đổi ms sang tick, làm tròn lên (ADR 0004): một thời lượng dương không bao giờ thành 0 tick.
pub fn ms_to_ticks(ms: u32, ticks_per_second: u32) -> Tick {
    let ticks = (u64::from(ms) * u64::from(ticks_per_second)).div_ceil(1000);
    Tick::try_from(ticks).expect("thời lượng vượt miền Tick")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_round_up() {
        assert_eq!(ms_to_ticks(0, 60), 0);
        assert_eq!(ms_to_ticks(1, 60), 1);
        assert_eq!(ms_to_ticks(2500, 60), 150);
        assert_eq!(ms_to_ticks(6300, 60), 378);
        assert_eq!(ms_to_ticks(17, 60), 2); // 1,02 tick → 2
    }

    #[test]
    fn cells_sort_row_major() {
        let mut v = vec![Cell::new(2, 0), Cell::new(0, 1), Cell::new(1, 0)];
        v.sort();
        assert_eq!(v, vec![Cell::new(1, 0), Cell::new(2, 0), Cell::new(0, 1)]);
    }
}
