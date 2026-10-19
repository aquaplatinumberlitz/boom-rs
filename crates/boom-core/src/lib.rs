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
//! Đây là walking skeleton: mới có di chuyển, va chạm ô và hết giờ; chưa có bom/item/nhốt.
#![deny(clippy::float_arithmetic)]

mod event;
mod mode;
mod num;
mod rng;
mod rules;
mod sim;
mod system;
mod systems;

pub use event::{EventId, EventKind, EventSink, SimulationEvent, event_digest};
pub use mode::{FfaBattle, Mode, Outcome, mode_by_id, modes};
pub use num::{Mpx, Tick, ms_to_ticks};
pub use rng::Rng;
pub use rules::Rules;
pub use sim::{CoreError, Dir, Phase, Player, PlayerInput, PlayerSetup, SimulationState, World};
pub use system::System;
