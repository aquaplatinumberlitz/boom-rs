//! Định dạng dựng trận dùng chung và harness boundary scenario (TEAM_WORKFLOW §5).
//!
//! - [`setup`]: dựng trận từ TOML (map ASCII, người chơi, input, entity dựng sẵn), chạy từng tick và
//!   kiểm bất biến contract sau mỗi tick. `boom-replay` dùng chung phần này.
//! - [`scenario`]: kịch bản nghiệm thu có kỳ vọng; mọi file trong `fixtures/scenarios/` chạy trong
//!   `cargo test -p boom-scenario` (lọc bằng biến môi trường `SCENARIO=<chuỗi>`).

pub mod scenario;
pub mod setup;

pub type Error = Box<dyn std::error::Error>;
