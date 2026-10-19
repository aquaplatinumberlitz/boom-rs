use boom_content::rules::{ProfileError, RulesProfile};

use crate::num::{Mpx, Tick, ms_to_ticks};

/// Giá trị luật đã đổi sang đơn vị mô phỏng. Chỉ chứa rule mà core đang dùng.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub ticks_per_second: u32,
    pub tile: Mpx,
    /// Tốc độ một cấp, mpx/tick, làm tròn xuống khi đổi từ px/s.
    pub speed_per_level: Mpx,
    pub round_ticks: Tick,
    pub profile_hash: u64,
}

impl Rules {
    pub fn from_profile(p: &RulesProfile) -> Result<Self, ProfileError> {
        let hz = i32_of(p.int_in("sim.ticks_per_second", "Hz", 1, 1000)?);
        let tile_px = i32_of(p.int_in("grid.tile_px", "px", 1, 1000)?);
        let speed_px_s = i32_of(p.int_in("player.speed_px_per_s_per_level", "px/s", 1, 10_000)?);
        let round_ms = u32::try_from(p.int_in("round.duration_ms", "ms", 1, 3_600_000)?)
            .expect("đã kiểm miền");
        let ticks_per_second = u32::try_from(hz).expect("đã kiểm miền");
        Ok(Self {
            ticks_per_second,
            tile: Mpx::from_px(tile_px),
            speed_per_level: Mpx(speed_px_s * Mpx::PER_PX / hz),
            round_ticks: ms_to_ticks(round_ms, ticks_per_second),
            profile_hash: p.identity_hash(),
        })
    }
}

fn i32_of(v: i64) -> i32 {
    i32::try_from(v).expect("miền đã kiểm nằm trong i32")
}
