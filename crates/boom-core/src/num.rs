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
}
