//! Adapter giữa state của `boom-core` và DTO của `boom-proto` (ADR 0009, ARCHITECTURE §1).
//!
//! Crate này là **chỗ duy nhất** biết cách đổi giữa hai bên, dùng chung cho server (authority) và
//! client (prediction) để không có hai bản chuyển đổi lệch nhau:
//!
//! - [`snapshot`]: dựng `Snapshot` DTO từ `SimulationState` và khôi phục ngược lại; state khôi
//!   phục phải có **cùng state hash** (ADR 0009 §4).
//! - [`input`]: đổi `InputCommand` trên dây thành `PlayerInput` của core.
//! - [`events`]: đổi event của core thành DTO `RoundEvents`.
//! - [`room`]: đổi `boom_room` ↔ DTO phòng và cổng trạng thái sau `AuthOk` (Addendum B).
//!
//! Không có luật chơi nào ở đây: mọi giá trị luật đến từ `Rules` của core.

pub mod error;
pub mod events;
pub mod input;
pub mod room;
pub mod snapshot;

pub use error::SyncError;
pub use events::{input_ack, to_round_events};
pub use input::{command_input, merge_commands};
pub use room::{
    RoomGate, RoomGateError, RoomSyncError, from_room_command, from_room_state, reject_reason,
    to_lobby_list, to_room_command, to_room_state,
};
pub use snapshot::{SnapshotContext, from_snapshot, to_snapshot};
