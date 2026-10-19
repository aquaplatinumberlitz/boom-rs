//! Server authoritative V1 (ADR 0005): vòng đời phòng → loading barrier → trận → kết quả → phòng.
//!
//! Lõi authority thuần, không wall clock trực tiếp; caller truyền `now_ms` và gọi
//! [`Authority::advance_to`]. [`Server`] bọc lõi bằng vòng IO `boom-net` (một luồng đọc chặn mỗi
//! kết nối), handshake `Hello`/`Welcome`/`Reject` và snapshot/event DTO. Input ở biên dùng
//! `boom_proto::InputBatch`; chuyển state↔DTO nằm ở `boom-sync`, không có bản chuyển đổi riêng
//! trong server.
//!
//! T1 vận hành (ADR 0011 §8): [`Logger`] ghi JSON Lines, [`HealthState`] + [`AdminServer`] mở
//! admin listener loopback, [`Heartbeat`] ghi `health.json` atomic, [`run_health_cli`] cho lệnh
//! `health` runtime/offline.

mod admin;
pub mod auth;
mod authority;
mod config;
pub mod identity;
mod input;
mod observability;
mod replay_log;
mod server;
#[cfg(test)]
mod tests;

pub use admin::{
    AdminServer, DbHealth, HealthCliOutcome, HealthState, Heartbeat, handle_admin_connection,
    health_artifact_path, peer_is_loopback, probe_database, run_health_cli, write_health_json,
};
pub use authority::{
    Authority, InternalRoundResult, InternalServerEvent, MatchOutcome, MatchSetup, RoomUpdate,
    ServerError, ServerPhase,
};
pub use config::{
    CharacterStatsTable, ConfigError, ConfigFingerprint, HandshakeConfig, InputLimits,
    OperationalConfig, PolicyConfig, ReplaySetup, ServerConfig,
};
pub use input::{InputAck, InputError};
pub use observability::{
    LogFields, LogLevel, Logger, SharedLogBuffer, events, new_instance_id, now_ms, rfc3339_utc_ms,
};
pub use server::Server;
