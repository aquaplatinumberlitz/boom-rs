//! Harness QA mạng V1-027: **1 server + N bot headless** qua `boom-net::MemoryTransport`.
//!
//! # Mục tiêu
//!
//! - Chạy đúng kịch bản V1: `handshake → Register/Login → sảnh → phòng → ready/start → chơi
//!   bằng input ngẫu nhiên có seed → kết quả → trận 2` (SCOPE_V1 §7.1).
//! - Mô phỏng hồ sơ mạng SCOPE_V1 §7.6: RTT 100 ms, jitter 0–30 ms, loss 1 %, trùng/đảo thứ tự
//!   có kiểm soát (thông số trong [`profile`]).
//! - Đo độ trễ input (gửi → server áp dụng) và RTT (ping/pong), có p50/p95.
//! - Kiểm **không diverge vĩnh viễn**: hash state bot dựng lại sau resync phải bằng hash state
//!   server ở cùng tick cuối round.
//!
//! # Ranh giới
//!
//! - Harness **không** kết luận gì về độ trùng khớp với client/server gốc: nó chỉ đo bản Rust
//!   hiện tại (SCOPE_V1 §7.8, REWRITE_PLAN "đo nguồn").
//! - Input và round fingerprint dùng source/API client chung ([`prediction`], [`room`]);
//!   auth/phòng vẫn dùng adapter DTO của harness, chưa chạy facade NetClient đầy đủ.
//! - Correction đo sai số vị trí cùng tick; bot chỉ ack sau khi có bảng RoundResult và hash cuối.
//! - Mọi giá trị luật trận đọc từ profile rules (`crates/boom-qa/rules/*.toml`), không có hằng số
//!   luật trong code (AGENTS §3).

pub mod bot;
pub mod harness;
pub mod metrics;
pub mod profile;
pub mod report;

pub use bot::{
    BotClient, BotConfig, BotError, BotFactory, BotPhase, BotStats, DirectDtoBot,
    DirectDtoBotFactory,
};
pub use harness::{QaError, RecoveryScenario, RunConfig, run, run_scenario, run_with};
pub use metrics::Samples;
pub use profile::{ProfileError, QaProfile};
pub use report::{REPORT_SCHEMA, RunReport};

// Dùng chính implementation client, không fork/copy prediction. boom-client hiện là binary;
// chưa mở phạm vi Cargo.toml/lib client trong #166 nên include source chuẩn qua path.
#[allow(dead_code)]
#[path = "../../boom-client/src/net/prediction.rs"]
pub(crate) mod prediction;

/// API phòng thuần dùng cùng client: fingerprint, roster và round identity.
#[allow(dead_code)]
#[path = "../../boom-client/src/net/room.rs"]
pub(crate) mod room;
