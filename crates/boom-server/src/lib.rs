//! Server authoritative V1 (ADR 0005): vòng đời phòng → loading barrier → trận → kết quả → phòng.
//!
//! Lõi authority thuần, không wall clock trực tiếp; caller truyền `now_ms` và gọi
//! [`Authority::advance_to`]. [`Server`] bọc lõi bằng vòng IO `boom-net` (một luồng đọc chặn mỗi
//! kết nối), handshake `Hello`/`Welcome`/`Reject` và snapshot/event DTO. Input ở biên dùng
//! `boom_proto::InputBatch`; chuyển state↔DTO nằm ở `boom-sync`, không có bản chuyển đổi riêng
//! trong server.

pub mod auth;
mod authority;
mod config;
pub mod identity;
mod input;
mod server;
#[cfg(test)]
mod tests;

pub use authority::{
    Authority, InternalRoundResult, InternalServerEvent, MatchOutcome, MatchSetup, RoomUpdate,
    ServerError, ServerPhase,
};
pub use config::{CharacterStatsTable, HandshakeConfig, InputLimits, PolicyConfig, ServerConfig};
pub use input::{InputAck, InputError};
pub use server::Server;
