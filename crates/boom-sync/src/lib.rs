//! Adapter giữa state của `boom-core` và DTO của `boom-proto` (ADR 0009, ARCHITECTURE §1).
//!
//! Crate này là **chỗ duy nhất** biết cách đổi giữa hai bên, dùng chung cho server (authority) và
//! client (prediction) để không có hai bản chuyển đổi lệch nhau:
//!
//! - [`snapshot`]: dựng `Snapshot` DTO từ `SimulationState` và khôi phục ngược lại; state khôi
//!   phục phải có **cùng state hash** (ADR 0009 §4).
//! - [`input`]: đổi `InputCommand` trên dây thành `PlayerInput` của core.
//! - [`events`]: đổi event của core thành DTO `RoundEvents`.
//!
//! Không có luật chơi nào ở đây: mọi giá trị luật đến từ `Rules` của core.

pub mod error;
pub mod events;
pub mod input;
pub mod snapshot;

pub use error::SyncError;
pub use events::{input_ack, to_round_events};
pub use input::{command_input, merge_commands};
pub use snapshot::{SnapshotContext, from_snapshot, to_snapshot};
