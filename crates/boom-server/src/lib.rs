//! Server authoritative V1 (ADR 0005): vòng đời phòng → loading barrier → trận → kết quả → phòng.
//!
//! Lõi authority thuần, không socket/wall clock trực tiếp; caller truyền `now_ms` và gọi
//! [`Authority::advance_to`]. Test gọi hàm trực tiếp. Phần 2 sẽ nối DTO `boom-proto`/`boom-sync`
//! và IO `boom-net`; các kiểu `Internal*` ở đây được đặt tên rõ là nội bộ để thay thế.

mod authority;
mod config;
mod input;
#[cfg(test)]
mod tests;

pub use authority::{
    Authority, MatchOutcome, MatchSetup, RoomUpdate, RoundResult, ServerError, ServerEvent,
    ServerPhase,
};
pub use config::{CharacterStatsTable, InputLimits, PolicyConfig, ServerConfig};
pub use input::{InputAck, InputError, InternalInputBatch, InternalInputCommand};
