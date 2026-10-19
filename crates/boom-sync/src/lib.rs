//! Adapter state ↔ wire (ADR 0009, ARCHITECTURE §1): dựng `Snapshot` DTO từ `SimulationState` và khôi phục
//! ngược lại bằng getter contracts-r4; chuyển `InputCommand` đã chấp nhận thành `PlayerInput`.
//! Dùng chung cho server (authority) và client (prediction) để không có hai bản chuyển đổi.
//!
//! Chưa triển khai — task đợt 2 (#44).
