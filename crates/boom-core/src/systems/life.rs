//! Bị nhốt, chạm địch, hết thời gian nhốt thì chết — V1-020.
//!
//! Thứ tự trong một tick (design, RULES_V1 R-034/R-035):
//! 1. Người `Alive` đứng trong ô có lửa → `Trapped { since: tick }`, phát `player_trapped`
//!    với `by` là chủ của lửa.
//! 2. Chỉ người đã `Trapped` từ đầu bước life mới được xét chết. Người `Alive` còn lại đứng cùng
//!    ô làm vỡ bong bóng (R-028); nếu vừa hết `trap_ticks` vừa bị chạm thì `by` là người chạm
//!    (nhiều người `Alive` cùng ô thì chọn id nhỏ nhất, R-035).
//! 3. Hết thời gian nhốt mà không bị chạm → chết, `by = None` (R-026).
//!
//! Người `Trapped` không làm vỡ bong bóng của nhau; `Dead` không đổi trạng thái lần nữa.
//! Mọi giá trị thời gian lấy từ `Rules::trap_ticks`; không có hằng số luật trong file này.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/life/`. Luật và nguồn: RULES_V1.

use crate::event::{EventKind, EventSink};
use crate::num::Cell;
use crate::sim::{PlayerInput, World};
use crate::state::{Life as PlayerLife, SimulationState};
use crate::system::System;

#[derive(Debug)]
pub struct Life;

pub static LIFE: Life = Life;

impl System for Life {
    fn name(&self) -> &'static str {
        "life"
    }

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        _inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
    ) {
        let tick = state.tick;
        // Trạng thái đầu bước life: người vừa bị lửa nhốt trong tick này chưa thể bị chạm làm chết.
        let lives: Vec<PlayerLife> = state.players.iter().map(|p| p.life).collect();
        let cells: Vec<Cell> = state
            .players
            .iter()
            .map(|p| world.cell_at(p.x, p.y))
            .collect();

        // 1. Lửa nhốt người còn sống. Mỗi ô tối đa một flame (invariant), nên chủ lửa là duy nhất.
        for i in 0..state.players.len() {
            if lives[i] != PlayerLife::Alive {
                continue;
            }
            let cell = cells[i];
            let Some(owner) = state
                .flames
                .iter()
                .find(|f| f.cell == cell)
                .map(|f| f.owner)
            else {
                continue;
            };
            let player = state.players[i].id;
            state.players[i].life = PlayerLife::Trapped { since: tick };
            events.emit(EventKind::PlayerTrapped { player, by: owner });
        }

        // 2. Chạm địch và 3. hết thời gian nhốt. Chỉ xét người đã Trapped từ đầu bước life.
        let trap_ticks = world.rules().trap_ticks;
        for i in 0..state.players.len() {
            let PlayerLife::Trapped { since } = lives[i] else {
                continue;
            };
            let cell = cells[i];
            let popper = (0..state.players.len()).find(|&j| {
                j != i && state.players[j].life == PlayerLife::Alive && cells[j] == cell
            });
            let player = state.players[i].id;
            // Chạm được ưu tiên hơn hết hạn khi cả hai xảy ra trong cùng tick (R-035).
            let by = if let Some(j) = popper {
                Some(state.players[j].id)
            } else if tick.saturating_sub(since) >= trap_ticks {
                None
            } else {
                continue;
            };
            state.players[i].life = PlayerLife::Dead;
            events.emit(EventKind::PlayerDied { player, by });
        }
    }
}
