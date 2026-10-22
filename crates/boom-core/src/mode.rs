use std::fmt;

use crate::num::Tick;
use crate::sim::World;
use crate::state::SimulationState;
use crate::system::System;
use crate::systems::{BLOCKS, BOMB_PLACE, EXPLOSION, LIFE, MOVEMENT, PICKUP};

/// Kết quả lượt do authority quyết một lần (SCOPE_V1 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Winner(u8),
    Draw,
}

/// Chế độ chơi: danh sách system theo thứ tự và điều kiện kết thúc.
///
/// Tương ứng `CGameStrategy` của client gốc, nơi loại phòng/map chọn strategy
/// (`CNormalGameStrategy`, `CConfusedFightGameStrategy`, `CRPGGameStrategy`…; `boom-re/docs/oc_e-game.md`).
/// Mode mới = một type implement trait này + đăng ký trong [`modes`]; không sửa mode có sẵn.
pub trait Mode: Sync {
    /// ID ổn định, ghi trong replay và thông điệp mạng.
    fn id(&self) -> &'static str;

    fn systems(&self) -> &'static [&'static dyn System];

    /// Gọi sau khi mọi system của tick đã chạy; `None` là chơi tiếp.
    fn outcome(&self, world: &World, state: &SimulationState) -> Option<Outcome>;
}

impl fmt::Debug for dyn Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mode({})", self.id())
    }
}

/// Battle cá nhân V1 (V1-021): người cuối cùng chưa chết thắng; không còn ai hoặc hết giờ là hòa.
/// Người đang bị nhốt vẫn tính là chưa chết.
#[derive(Debug)]
pub struct FfaBattle;

/// Thứ tự trong một tick (contracts-r1, ARCHITECTURE §5). Thứ tự là luật: đổi phải qua integrator,
/// có boundary scenario và bless golden.
static FFA_SYSTEMS: [&dyn System; 6] =
    [&MOVEMENT, &BOMB_PLACE, &EXPLOSION, &BLOCKS, &PICKUP, &LIFE];

impl Mode for FfaBattle {
    fn id(&self) -> &'static str {
        "battle_ffa"
    }

    fn systems(&self) -> &'static [&'static dyn System] {
        &FFA_SYSTEMS
    }

    fn outcome(&self, world: &World, state: &SimulationState) -> Option<Outcome> {
        let mut alive = state.players.iter().filter(|p| !p.is_dead());
        let first = alive.next();
        let more = alive.next().is_some();
        match (first, more) {
            // Luật "người cuối cùng" chỉ áp dụng khi lượt bắt đầu với từ hai người.
            (Some(p), false) if state.players.len() >= 2 => Some(Outcome::Winner(p.id)),
            (None, _) => Some(Outcome::Draw),
            // Hết giờ: đồng hồ lượt chạy **sau** warmup, nên warmup không tính vào thời hạn
            // (R-100, BnB `server/game.js:641–645`). Kết quả gameplay được xét trước timeout.
            _ if round_tick(world, state) >= world.rules().round_ticks => Some(Outcome::Draw),
            _ => None,
        }
    }
}

/// Đồng hồ lượt: số tick đã chạy kể từ khi hết warmup (R-100).
fn round_tick(world: &World, state: &SimulationState) -> Tick {
    state.tick.saturating_sub(world.rules().warmup_ticks)
}

static MODES: [&dyn Mode; 1] = [&FfaBattle];

/// Mọi mode đã đăng ký.
pub fn modes() -> &'static [&'static dyn Mode] {
    &MODES
}

pub fn mode_by_id(id: &str) -> Option<&'static dyn Mode> {
    modes().iter().copied().find(|m| m.id() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique_and_resolvable() {
        let mut ids: Vec<_> = modes().iter().map(|m| m.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), modes().len());
        assert!(mode_by_id("battle_ffa").is_some());
        assert!(mode_by_id("nope").is_none());
    }
}
