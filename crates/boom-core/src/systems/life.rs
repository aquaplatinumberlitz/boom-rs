//! Bị nhốt, chạm địch, hết thời gian nhốt thì chết — V1-020. Chưa triển khai: system đang là chỗ trống trong pipeline FFA.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/life/` (đang `pending`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use crate::event::EventSink;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;
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
        _world: &World,
        _state: &mut SimulationState,
        _inputs: &[PlayerInput],
        _events: &mut EventSink<'_>,
    ) {
        // TODO(V1-020): triển khai theo task được giao.
    }
}
