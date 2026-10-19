//! Đặt bom và hạn mức — V1-013. Chưa triển khai: system đang là chỗ trống trong pipeline FFA.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/bomb_place/` (đang `pending`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use crate::event::EventSink;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;
use crate::system::System;

#[derive(Debug)]
pub struct BombPlace;

pub static BOMB_PLACE: BombPlace = BombPlace;

impl System for BombPlace {
    fn name(&self) -> &'static str {
        "bomb_place"
    }

    fn run(
        &self,
        _world: &World,
        _state: &mut SimulationState,
        _inputs: &[PlayerInput],
        _events: &mut EventSink<'_>,
    ) {
        // TODO(V1-013): triển khai theo task được giao.
    }
}
