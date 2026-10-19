//! Giấu item đầu lượt, phá khối mềm, lộ item — V1-016. Chưa triển khai: system đang là chỗ trống trong pipeline FFA.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/blocks/` (đang `pending`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use crate::event::EventSink;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;
use crate::system::System;

#[derive(Debug)]
pub struct Blocks;

pub static BLOCKS: Blocks = Blocks;

impl System for Blocks {
    fn name(&self) -> &'static str {
        "blocks"
    }

    fn run(
        &self,
        _world: &World,
        _state: &mut SimulationState,
        _inputs: &[PlayerInput],
        _events: &mut EventSink<'_>,
    ) {
        // TODO(V1-016): triển khai theo task được giao.
    }
}
