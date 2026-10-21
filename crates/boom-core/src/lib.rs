//! Mô phỏng trận thuần và xác định (ADR 0004), thiết kế để mở rộng (ADR 0008).
//!
//! Core không đọc file, socket hay wall clock; thời gian là [`Tick`], tọa độ là [`Mpx`] (1/1000 px),
//! mọi giá trị luật lấy từ rules profile qua [`Rules`]. Quy tắc determinism được clippy thực thi
//! (`clippy.toml` của crate này và `float_arithmetic` bên dưới).
//!
//! Điểm mở rộng:
//! - [`Mode`]: luật thắng/thua và danh sách system của một chế độ; chọn qua [`mode_by_id`].
//! - [`System`]: một bước xử lý trong tick (di chuyển, bom, item…), chạy theo thứ tự mode khai báo.
//! - [`SimulationEvent`]: mọi thay đổi đáng kể phát ra event có ID ổn định; trình diễn, thống kê,
//!   nhiệm vụ, event vận hành tiêu thụ event ở ngoài core thay vì chèn code vào core.
//!
//! Contract hiện hành: **contracts-r4** (ARCHITECTURE §5): kiểu state, event V1, pipeline FFA và
//! ma trận quyền ghi trong `systems/mod.rs`. Đã có di chuyển cơ bản và hết giờ; bom, lửa, item,
//! nhốt là system rỗng chờ task được giao.
#![deny(clippy::float_arithmetic)]

mod event;
mod mode;
mod num;
mod rng;
mod rules;
mod sim;
mod state;
mod system;
mod systems;

pub use event::{EventId, EventKind, EventSink, SimulationEvent, event_digest};
pub use mode::{FfaBattle, Mode, Outcome, mode_by_id, modes};
pub use num::{Cell, Mpx, Tick, ms_to_ticks};
pub use rng::Rng;
pub use rules::{CharacterStatsTable, ItemDef, ItemEffect, Rules};
pub use sim::{CoreError, Dir, PlayerInput, PlayerSetup, World};
pub use state::{
    Bomb, CharacterStats, EntityId, Field, Flame, FloorItem, IdAllocator, ItemKind, Life, Phase,
    Player, SimulationState, Stat,
};
pub use system::System;
