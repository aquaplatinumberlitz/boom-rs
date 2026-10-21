//! Hồ sơ QA: tham số mạng (theo SCOPE_V1 §7.6) + tham số harness, nạp từ TOML.
//!
//! Hồ sơ là **tham số fixture kiểm thử**, không phải luật chơi: không sinh claim `R-xxx` trong
//! RULES_V1 và không nằm trong `data/rules/`. Nguồn giá trị mạng là SCOPE_V1 §7.6 (RTT 100 ms,
//! jitter 0–30 ms, loss 1 %); tỉ lệ trùng/đảo thứ tự chưa được scope chốt nên hồ sơ tự đặt và
//! ghi rõ trong [QA_NETWORK](../../../docs/specification/QA_NETWORK.md).
//!
//! Giá trị luật trận (tick, fuse, speed…) **không** nằm ở đây: chúng đọc từ profile rules
//! (`crates/boom-qa/rules/*.toml`) qua `boom-content`, đúng quy tắc "mọi giá trị luật lấy từ
//! rules profile" của AGENTS §3.

use std::fmt;

use boom_net::{MemoryConfig, MemoryFaults, Timing};
use serde::Deserialize;

/// Schema hiện hành của file hồ sơ QA.
pub const PROFILE_SCHEMA: u32 = 1;

/// Lỗi nạp hồ sơ QA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// Không có hồ sơ tên này.
    Unknown(String),
    /// File hồ sơ không đọc được.
    Parse(String),
    /// File hồ sơ sai schema hoặc sai tên.
    Invalid(String),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(f, "không có hồ sơ QA tên `{name}`"),
            Self::Parse(detail) => write!(f, "không đọc được hồ sơ QA: {detail}"),
            Self::Invalid(detail) => write!(f, "hồ sơ QA không hợp lệ: {detail}"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Cách đo trễ của transport trong bộ nhớ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMode {
    /// Đồng hồ ảo: không ngủ, thứ tự giao vẫn tất định.
    Virtual,
    /// Trễ thật: transport ngủ đúng số ms ảo để đo độ trễ bằng đồng hồ thật.
    #[default]
    Real,
}

/// Thông số mạng của một hồ sơ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkProfile {
    /// RTT mục tiêu (ms) — SCOPE_V1 §7.6.
    pub rtt_ms: u32,
    /// Jitter cộng thêm vào trễ một chiều (ms) — SCOPE_V1 §7.6.
    pub jitter_ms: u32,
    /// Tỉ lệ mất frame (phần nghìn) — SCOPE_V1 §7.6 (1 %).
    pub loss_permille: u16,
    /// Tỉ lệ gửi trùng frame (phần nghìn) — scope chưa chốt, hồ sơ tự đặt.
    pub duplicate_permille: u16,
    /// Tỉ lệ đảo thứ tự (phần nghìn) — scope chưa chốt, hồ sơ tự đặt.
    pub reorder_permille: u16,
    /// Cửa sổ đảo thứ tự (ms) — trễ thêm ngẫu nhiên trong `[1, reorder_window_ms]`.
    pub reorder_window_ms: u32,
    /// Cách đo trễ.
    #[serde(default)]
    pub timing: TimingMode,
}

impl NetworkProfile {
    /// Trễ một chiều (ms) mà `boom-net` mô phỏng: RTT chia đôi.
    pub fn one_way_delay_ms(&self) -> u32 {
        self.rtt_ms / 2
    }

    /// Cấu hình transport trong bộ nhớ cho hồ sơ này.
    pub fn memory_config(&self, seed: u64) -> MemoryConfig {
        MemoryConfig {
            seed,
            faults: MemoryFaults {
                loss_permille: self.loss_permille,
                duplicate_permille: self.duplicate_permille,
                reorder_permille: self.reorder_permille,
                delay_ms: self.one_way_delay_ms(),
                jitter_ms: self.jitter_ms,
                reorder_window_ms: self.reorder_window_ms,
            },
            timing: match self.timing {
                TimingMode::Real => Timing::Real,
                TimingMode::Virtual => Timing::Virtual,
            },
        }
    }
}

/// Thông số chạy của harness cho một hồ sơ.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessProfile {
    /// Tên profile rules trong `crates/boom-qa/rules/`.
    pub rules_profile: String,
    /// Tên bản đồ trong `crates/boom-qa/maps/`.
    pub map: String,
    /// Số trận mỗi lần chạy.
    pub rounds: u32,
    /// Chu kỳ gửi input của bot (ms).
    pub input_interval_ms: u64,
    /// Số tick bot nhắm trước tick server cuối cùng bot biết.
    pub input_lead_ticks: u32,
    /// Chu kỳ gửi Ping của bot để đo RTT (ms).
    pub ping_interval_ms: u64,
    /// Chu kỳ server gửi snapshot cho client (ms) — tham số giao thức, không phải luật chơi.
    pub snapshot_interval_ms: u32,
    /// Ngân sách thời gian cho cả lần chạy (ms).
    pub timeout_ms: u64,
    /// Chu kỳ vòng lặp harness (ms).
    pub step_ms: u64,
    /// Chu kỳ xin lại danh sách sảnh khi bot chưa vào được phòng (ms).
    pub lobby_retry_ms: u64,
    /// Chu kỳ gửi lại lệnh idempotent khi chưa thấy phản hồi (ms).
    pub retry_ms: u64,
    /// Chu kỳ thử lại `Auth` khi chưa thấy `AuthOk` (ms).
    pub auth_retry_ms: u64,
    /// Chu kỳ xin lại snapshot kết thúc round khi chưa nhận được (ms).
    pub snapshot_retry_ms: u64,
    /// Hạn chờ snapshot kết thúc round trước khi bot từ bỏ và ack kết quả (ms).
    pub snapshot_deadline_ms: u64,
    /// Hạn chờ `AuthOk` trước khi harness mở lại phiên cho bot (ms).
    pub auth_deadline_ms: u64,
    /// Hạn chờ `RoomState` sau khi server đã xác nhận bot đang ở trong phòng (ms).
    ///
    /// Server chỉ đẩy `RoomState` khi phòng đổi; quá hạn mà chưa có thì bot rời phòng đó
    /// rồi vào lại để lấy lại `RoomState`.
    pub room_state_deadline_ms: u64,
    /// Chu kỳ `Ping` server → bot để giữ phiên (ms).
    ///
    /// Server chỉ làm mới hạn liveness khi **nhận được `Pong`** của client, nên một `Pong`
    /// mất đi sẽ không được bù lại nếu không có `Ping` tiếp theo.
    pub server_ping_interval_ms: u64,
    /// Hạn chờ `Pong` của server (ms): quá hạn thì server đóng phiên.
    ///
    /// Phải **lớn hơn nhiều** chu kỳ `Ping`: hồ sơ mạng cố ý mất 1 % frame, nên một `Pong`
    /// mất đi không được giết phiên.
    pub server_liveness_timeout_ms: u64,
    /// Số lần mở lại phiên tối đa cho một bot.
    pub max_reconnects: u32,
}

/// Một hồ sơ QA đầy đủ.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QaProfile {
    /// Schema của file hồ sơ.
    pub schema: u32,
    /// Tên hồ sơ.
    pub name: String,
    /// Thông số mạng.
    pub network: NetworkProfile,
    /// Thông số harness.
    pub harness: HarnessProfile,
}

impl QaProfile {
    /// Nạp hồ sơ tích hợp sẵn theo tên.
    pub fn builtin(name: &str) -> Result<Self, ProfileError> {
        let text = match name {
            "scope-v1" => include_str!("../profiles/scope-v1.toml"),
            "fast" => include_str!("../profiles/fast.toml"),
            other => return Err(ProfileError::Unknown(other.to_owned())),
        };
        let profile: Self =
            toml::from_str(text).map_err(|err| ProfileError::Parse(err.to_string()))?;
        if profile.schema != PROFILE_SCHEMA {
            return Err(ProfileError::Invalid(format!(
                "schema {} không được hỗ trợ (cần {PROFILE_SCHEMA})",
                profile.schema
            )));
        }
        if profile.name != name {
            return Err(ProfileError::Invalid(format!(
                "tên trong file là `{}`, không khớp `{name}`",
                profile.name
            )));
        }
        Ok(profile)
    }

    /// Tên các hồ sơ tích hợp sẵn.
    pub fn names() -> &'static [&'static str] {
        &["scope-v1", "fast"]
    }

    /// Cấu hình transport cho seed cho trước.
    pub fn memory_config(&self, seed: u64) -> MemoryConfig {
        self.network.memory_config(seed)
    }
}

/// Nội dung profile rules của hồ sơ QA (tích hợp sẵn).
pub fn builtin_rules(name: &str) -> Result<&'static str, ProfileError> {
    match name {
        "qa-net" => Ok(include_str!("../rules/qa-net.toml")),
        "qa-fast" => Ok(include_str!("../rules/qa-fast.toml")),
        other => Err(ProfileError::Unknown(other.to_owned())),
    }
}

/// Nội dung bản đồ ASCII của hồ sơ QA (tích hợp sẵn).
pub fn builtin_map(name: &str) -> Result<&'static str, ProfileError> {
    match name {
        "qa-village" => Ok(include_str!("../maps/qa-village.txt")),
        other => Err(ProfileError::Unknown(other.to_owned())),
    }
}

/// Các dòng bản đồ dùng được (bỏ dòng trống và dòng chú thích `;`).
pub fn map_rows(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_v1_profile_matches_scope_requirements() {
        let profile = QaProfile::builtin("scope-v1").unwrap();
        // SCOPE_V1 §7.6: RTT 100 ms, jitter 0–30 ms, loss 1 %.
        assert_eq!(profile.network.rtt_ms, 100);
        assert_eq!(profile.network.jitter_ms, 30);
        assert_eq!(profile.network.loss_permille, 10);
        assert_eq!(profile.network.one_way_delay_ms(), 50);
        assert_eq!(profile.harness.rounds, 2);
    }

    #[test]
    fn unknown_profile_is_rejected() {
        assert!(matches!(
            QaProfile::builtin("khong-co"),
            Err(ProfileError::Unknown(_))
        ));
    }

    #[test]
    fn map_rows_skip_comments() {
        let text = builtin_map("qa-village").unwrap();
        let rows = map_rows(text);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0], "#########");
        assert_eq!(rows[1], "#0.1.2.3#");
    }

    #[test]
    fn builtin_rules_parse() {
        for name in ["qa-net", "qa-fast"] {
            let text = builtin_rules(name).unwrap();
            let profile = boom_content::rules::RulesProfile::from_toml(text).unwrap();
            assert!(!profile.release);
        }
    }
}
