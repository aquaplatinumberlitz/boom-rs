//! Nhặt item Bomb/Fire/Roller — V1-017…V1-019. Chưa triển khai: system đang là chỗ trống trong pipeline FFA.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/pickup/` (đang `pending`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use crate::event::EventSink;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;
use crate::system::System;

#[derive(Debug)]
pub struct Pickup;

pub static PICKUP: Pickup = Pickup;

impl System for Pickup {
    fn name(&self) -> &'static str {
        "pickup"
    }

    fn run(
        &self,
        _world: &World,
        _state: &mut SimulationState,
        _inputs: &[PlayerInput],
        _events: &mut EventSink<'_>,
    ) {
        // TODO(V1-017…V1-019): triển khai theo task được giao.
    }
}
