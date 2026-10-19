//! Cấu hình authority. Không chứa luật chơi; các timeout vận hành là policy SCOPE_V1 §3
//! và các ngưỡng input là tham số giao thức ADR 0009 (design), có tên và nguồn.

use boom_core::{CharacterStats, Rules, Tick, ms_to_ticks};
use boom_proto::ProtocolLimits;
use boom_room::RoomPolicy;

/// Policy vòng đời phòng/trận của V1. Giá trị mặc định lấy từ SCOPE_V1 §3; caller có thể
/// thay bằng cấu hình khác khi test hoặc khi scope revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyConfig {
    /// Loading barrier tối đa trước khi hủy start (SCOPE_V1 §3).
    pub loading_timeout_ms: u32,
    /// Thời gian mất kết nối trước khi loại một lần (SCOPE_V1 §3).
    pub disconnect_timeout_ms: u32,
    /// Thời gian chờ ack kết quả rồi về phòng (SCOPE_V1 §3).
    pub results_timeout_ms: u32,
    /// Số slot và ngưỡng start; policy phòng do server nạp, không phải hằng số của boom-room.
    pub room_policy: RoomPolicy,
}

impl PolicyConfig {
    /// Policy V1 theo SCOPE_V1 §3. Đây là policy dự án (DESIGN), không phải luật client gốc.
    pub fn scope_v1() -> Self {
        Self {
            loading_timeout_ms: 15_000,
            disconnect_timeout_ms: 10_000,
            results_timeout_ms: 5_000,
            room_policy: RoomPolicy::new(4, 2).expect("policy SCOPE_V1 hợp lệ"),
        }
    }
}

/// Giới hạn scheduler input theo ADR 0009 §3.5. Đây là tham số giao thức, không phải luật chơi;
/// đổi giá trị phải kèm revision ADR/ProtocolLimits, không sửa lén trong code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputLimits {
    /// Số lệnh tối đa trong một batch (ADR 0009).
    pub max_batch_commands: u8,
    /// Số sequence tối đa được buffer vượt `applied_seq` (ADR 0009).
    pub max_gap: u32,
    /// Số batch tối đa mỗi giây cho một người (ADR 0009).
    pub max_batches_per_second: u32,
    /// Lệnh cũ hơn `server_tick - late` bị retire, không áp dụng (ADR 0009).
    pub late_ms: u32,
    /// Lệnh xa hơn `server_tick + lead` bị từ chối (ADR 0009).
    pub lead_ms: u32,
    /// Gap timeout chỉ retire sequence thiếu (ADR 0009).
    pub gap_timeout_ms: u32,
    /// Không có input mới trong khoảng này thì hướng về neutral (ADR 0009).
    pub stale_ms: u32,
}

impl InputLimits {
    /// Giá trị V1 trong ADR 0009 §3.5 (design, chưa có trace server gốc).
    pub fn adr_0009() -> Self {
        Self {
            max_batch_commands: 8,
            max_gap: 16,
            max_batches_per_second: 120,
            late_ms: 100,
            lead_ms: 200,
            gap_timeout_ms: 100,
            stale_ms: 150,
        }
    }
}

/// Cấu hình đầy đủ của authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub policy: PolicyConfig,
    pub input: InputLimits,
    /// Miền DTO phiên V1 (ADR 0009 §4.2), dùng khi decode/check `InputBatch`/`Snapshot`.
    pub protocol_limits: ProtocolLimits,
}

impl ServerConfig {
    pub fn v1() -> Self {
        Self {
            policy: PolicyConfig::scope_v1(),
            input: InputLimits::adr_0009(),
            protocol_limits: ProtocolLimits::v1(),
        }
    }
}

/// Ngưỡng input đã đổi sang tick theo `Rules::ticks_per_second` (ADR 0009 §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputTicks {
    pub(crate) max_gap: u32,
    pub(crate) max_batch_commands: u8,
    pub(crate) max_batches_per_second: u32,
    pub(crate) late: Tick,
    pub(crate) lead: Tick,
    pub(crate) gap_timeout: Tick,
    pub(crate) stale: Tick,
}

impl InputTicks {
    pub(crate) fn from_limits(limits: InputLimits, rules: &Rules) -> Self {
        Self {
            max_gap: limits.max_gap,
            max_batch_commands: limits.max_batch_commands,
            max_batches_per_second: limits.max_batches_per_second,
            late: ms_to_ticks(limits.late_ms, rules.ticks_per_second),
            lead: ms_to_ticks(limits.lead_ms, rules.ticks_per_second),
            gap_timeout: ms_to_ticks(limits.gap_timeout_ms, rules.ticks_per_second),
            stale: ms_to_ticks(limits.stale_ms, rules.ticks_per_second),
        }
    }
}

/// Chỉ số khởi đầu của hai template C00/C01, do caller nạp từ content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterStatsTable {
    pub c00: CharacterStats,
    pub c01: CharacterStats,
}

/// Định danh server dùng để chấp nhận `Hello` (ADR 0009 §1, V1-002).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandshakeConfig {
    /// `rules_profile_id` server đã nạp.
    pub rules_profile_id: String,
    /// `rules_hash` (`fnv1a64-v0`) của profile server đã nạp.
    pub rules_hash: u64,
    /// SHA-256 manifest content server đã nạp.
    pub content_sha256: [u8; 32],
    /// Epoch content của phiên server.
    pub content_epoch: u32,
    /// Chu kỳ gửi snapshot, ms (ADR 0009 design).
    pub snapshot_interval_ms: u32,
}

/// Cấu hình vận hành T1 (ADR 0011 §8.2/§10), tách khỏi [`ServerConfig`] của authority.
///
/// Đây là cấu hình tiến trình: địa chỉ admin loopback, thư mục artifact, chu kỳ heartbeat và
/// timeout CLI health. Không chứa luật chơi; các giá trị mặc định là DESIGN của V1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationalConfig {
    /// Địa chỉ admin listener; bắt buộc loopback (ADR 0011 §8.2.1).
    pub admin_addr: std::net::SocketAddr,
    /// Thư mục chứa `health.json` và (sau này) DB.
    pub data_dir: std::path::PathBuf,
    /// Các địa chỉ game server bind; dùng trong `HealthResponse.listen_addrs`.
    pub listen_addrs: Vec<std::net::SocketAddr>,
    /// Build id hiển thị trong log/health fingerprint.
    pub server_version: String,
    /// Chu kỳ heartbeat + ghi `health.json`, ms (ADR 0011 §8.1/§8.2.3).
    pub heartbeat_interval_ms: u64,
    /// Timeout CLI health runtime, ms (ADR 0011 §8.2.1).
    pub health_timeout_ms: u64,
    /// TTL phiên tài khoản, ms (ADR 0011 §6; T4).
    pub session_ttl_ms: u64,
    /// Chu kỳ Ping cho connection đã auth, ms (ADR 0011 §6.1; T4).
    pub session_ping_interval_ms: u64,
    /// Timeout không Pong trước khi coi connection chết, ms (ADR 0011 §6.1; T4).
    pub session_liveness_timeout_ms: u64,
    /// Cho phép đăng ký tài khoản (ADR 0011 §9; T3).
    pub registration_open: bool,
    /// Ngưỡng thử auth theo tài khoản mỗi phút (ADR 0011 §5; T3).
    pub auth_attempts_per_minute: u32,
    /// Ngưỡng thử auth theo IP mỗi phút (ADR 0011 §5; T3).
    pub auth_attempts_per_ip_per_minute: u32,
    /// `wire_schema` của V1 (ADR 0009).
    pub wire_schema: u16,
    /// `state_schema` của DTO snapshot V1.
    pub state_schema: u16,
    /// Revision của `ProtocolLimits`.
    pub protocol_limits_version: u32,
    /// Profile rules đang chạy.
    pub rules_profile_id: String,
    /// Hash định danh profile rules.
    pub rules_hash: u64,
    /// Hash manifest content đã nạp.
    pub content_sha256: [u8; 32],
    /// Content epoch của phiên.
    pub content_epoch: u32,
}

/// Địa chỉ admin mặc định (ADR 0011 §8.2.1).
pub const DEFAULT_ADMIN_ADDR: &str = "127.0.0.1:17890";
/// Địa chỉ game mặc định dùng trong fingerprint/health.
pub const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:17889";
/// Chu kỳ heartbeat mặc định: 60 giây.
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 60_000;
/// Timeout CLI health mặc định: 3 giây.
pub const DEFAULT_HEALTH_TIMEOUT_MS: u64 = 3_000;

impl Default for OperationalConfig {
    fn default() -> Self {
        Self {
            admin_addr: DEFAULT_ADMIN_ADDR.parse().expect("địa chỉ admin mặc định"),
            data_dir: std::path::PathBuf::from("data"),
            listen_addrs: vec![DEFAULT_LISTEN_ADDR.parse().expect("địa chỉ game mặc định")],
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            health_timeout_ms: DEFAULT_HEALTH_TIMEOUT_MS,
            session_ttl_ms: 86_400_000,
            session_ping_interval_ms: 15_000,
            session_liveness_timeout_ms: 30_000,
            registration_open: true,
            auth_attempts_per_minute: 10,
            auth_attempts_per_ip_per_minute: 20,
            wire_schema: 1,
            state_schema: 1,
            protocol_limits_version: ProtocolLimits::v1().revision,
            rules_profile_id: "unknown".to_owned(),
            rules_hash: 0,
            content_sha256: [0; 32],
            content_epoch: 0,
        }
    }
}

/// Lỗi đọc cấu hình vận hành từ biến môi trường.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Giá trị địa chỉ không parse được.
    Addr { name: &'static str, value: String },
    /// Giá trị số không parse được.
    Number { name: &'static str, value: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Addr { name, value } => write!(f, "{name} không phải địa chỉ: {value}"),
            Self::Number { name, value } => write!(f, "{name} không phải số: {value}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl OperationalConfig {
    /// Cấu hình từ biến môi trường, dùng cho binary và test CLI.
    ///
    /// Các biến: `BOOM_ADMIN_ADDR`, `BOOM_DATA_DIR`, `BOOM_LISTEN_ADDR`,
    /// `BOOM_SERVER_VERSION`, `BOOM_HEARTBEAT_INTERVAL_MS`, `BOOM_HEALTH_TIMEOUT_MS`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        if let Ok(value) = std::env::var("BOOM_ADMIN_ADDR") {
            config.admin_addr = value.parse().map_err(|_| ConfigError::Addr {
                name: "BOOM_ADMIN_ADDR",
                value,
            })?;
        }
        if let Ok(value) = std::env::var("BOOM_DATA_DIR") {
            config.data_dir = value.into();
        }
        if let Ok(value) = std::env::var("BOOM_LISTEN_ADDR") {
            config.listen_addrs = value
                .split(',')
                .map(|part| {
                    part.trim().parse().map_err(|_| ConfigError::Addr {
                        name: "BOOM_LISTEN_ADDR",
                        value: value.clone(),
                    })
                })
                .collect::<Result<_, _>>()?;
        }
        if let Ok(value) = std::env::var("BOOM_SERVER_VERSION") {
            config.server_version = value;
        }
        if let Ok(value) = std::env::var("BOOM_HEARTBEAT_INTERVAL_MS") {
            config.heartbeat_interval_ms = value.parse().map_err(|_| ConfigError::Number {
                name: "BOOM_HEARTBEAT_INTERVAL_MS",
                value,
            })?;
        }
        if let Ok(value) = std::env::var("BOOM_HEALTH_TIMEOUT_MS") {
            config.health_timeout_ms = value.parse().map_err(|_| ConfigError::Number {
                name: "BOOM_HEALTH_TIMEOUT_MS",
                value,
            })?;
        }
        Ok(config)
    }

    /// Ngón tay cấu hình không nhạy cảm (ADR 0011 §8.3).
    pub fn fingerprint(&self) -> ConfigFingerprint {
        ConfigFingerprint {
            server_version: self.server_version.clone(),
            wire_schema: self.wire_schema,
            state_schema: self.state_schema,
            rules_profile_id: self.rules_profile_id.clone(),
            rules_hash: self.rules_hash,
            content_sha256: self.content_sha256,
            protocol_limits_version: self.protocol_limits_version,
        }
    }

    /// Kiểm admin listener chỉ bind loopback.
    pub fn admin_is_loopback(&self) -> bool {
        self.admin_addr.ip().is_loopback()
    }
}

/// Ngón tay cấu hình không nhạy cảm, xuất hiện trong `health` và `server.start` (ADR 0011 §8.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigFingerprint {
    pub server_version: String,
    pub wire_schema: u16,
    pub state_schema: u16,
    pub rules_profile_id: String,
    pub rules_hash: u64,
    pub content_sha256: [u8; 32],
    pub protocol_limits_version: u32,
}

impl ConfigFingerprint {
    pub fn content_sha256_hex(&self) -> String {
        self.content_sha256
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
