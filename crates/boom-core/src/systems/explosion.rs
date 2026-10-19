//! Fuse, nổ, lan, chặn và nổ dây chuyền — V1-014, V1-015. Chưa triển khai: system đang là chỗ trống trong pipeline FFA.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/explosion/` (đang `pending`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use crate::event::EventSink;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;
use crate::system::System;

#[derive(Debug)]
pub struct Explosion;

pub static EXPLOSION: Explosion = Explosion;

impl System for Explosion {
    fn name(&self) -> &'static str {
        "explosion"
    }

    fn run(
        &self,
        _world: &World,
        _state: &mut SimulationState,
        _inputs: &[PlayerInput],
        _events: &mut EventSink<'_>,
    ) {
        // TODO(V1-014, V1-015): triển khai theo task được giao.
    }
}
