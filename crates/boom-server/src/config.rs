//! Cấu hình authority. Không chứa luật chơi; các timeout vận hành là policy SCOPE_V1 §3
//! và các ngưỡng input là tham số giao thức ADR 0009 (design), có tên và nguồn.
//!
//! # Đường đọc cấu hình
//!
//! Thứ tự áp dụng (sau cùng thắng trước):
//!
//! 1. **Mặc định** — [`OperationalConfig::default`] lấy từ [`AuthConfig::default`] và các
//!    hằng vận hành của ADR 0011.
//! 2. **File cấu hình** — file TOML mà `BOOM_CONFIG_FILE` trỏ tới
//!    ([`OperationalConfig::from_file`]); trường không có trong file thì giữ mặc định.
//! 3. **Biến môi trường** — `BOOM_*` ghi đè giá trị của file ([`OperationalConfig::from_env`]).
//!
//! Cấu hình là **tham số vận hành** (ADR 0011 §5, §14), không phải luật chơi: không có giá
//! trị nào ở đây nằm trong `data/rules` và không sinh claim `R-xxx`.
//!
//! # Chỉ số nhân vật: ở rules profile, không ở đây
//!
//! Chỉ số C00/C01 là **giá trị luật** (RULES_V1 R-040/R-041) nên nằm trong rules profile
//! (`data/rules/*.toml`, khóa `characters.c00_*` / `characters.c01_*`) chứ không phải trong
//! file cấu hình: mọi người chạy server phải dùng cùng một nguồn, và `rules_hash` phải
//! phản ánh chỉ số để handshake bắt được lệch (AGENTS §3). `boom-server serve` đọc bảng
//! chỉ số từ profile qua [`crate::runtime::load_content`].

use std::path::{Path, PathBuf};

use boom_core::{Rules, Tick, ms_to_ticks};
use boom_proto::ProtocolLimits;
use boom_room::RoomPolicy;
use serde::{Deserialize, Serialize};

use crate::auth::{Argon2Settings, AuthConfig};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub policy: PolicyConfig,
    pub input: InputLimits,
    /// Miền DTO phiên V1 (ADR 0009 §4.2), dùng khi decode/check `InputBatch`/`Snapshot`.
    pub protocol_limits: ProtocolLimits,
    /// Thư mục ghi replay round; `None` tắt ghi (V1-026).
    pub replay_dir: Option<std::path::PathBuf>,
}

impl ServerConfig {
    pub fn v1() -> Self {
        Self {
            policy: PolicyConfig::scope_v1(),
            input: InputLimits::adr_0009(),
            protocol_limits: ProtocolLimits::v1(),
            replay_dir: None,
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
    /// Thư mục chứa `health.json` và DB.
    pub data_dir: std::path::PathBuf,
    /// Đường dẫn file SQLite identity; `data_dir/boom-server.sqlite3` mặc định.
    pub db_path: std::path::PathBuf,
    /// Schema version T1/identity mong đợi (ADR 0011 §4).
    pub expected_db_schema_version: u32,
    /// Hạn hoàn tất handshake+auth cho kết nối chưa `AuthOk` (ADR 0011 §6.1).
    pub auth_deadline_ms: u64,
    /// Các địa chỉ game server bind; dùng trong `HealthResponse.listen_addrs`.
    pub listen_addrs: Vec<std::net::SocketAddr>,
    /// Thư mục bundle content (map V1) mà `serve` nạp.
    pub bundle_dir: std::path::PathBuf,
    /// File rules profile mà `serve` nạp.
    pub rules_path: std::path::PathBuf,
    /// File bảng phân loại ô theo build mà `serve` nạp (`data/maps/*.toml`).
    pub map_profile_path: std::path::PathBuf,
    /// Thư mục ghi replay round; `None` tắt ghi (V1-026).
    pub replay_dir: Option<std::path::PathBuf>,
    /// Các tick cần ghi checkpoint hash trong replay (V1-026).
    pub replay_checkpoints: Vec<u32>,
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
    /// Ngưỡng lần đăng nhập sai tối đa trong một cửa sổ (ADR 0011 §5).
    pub rate_limit_max_failures: u32,
    /// Ngưỡng **lượt đăng ký** tối đa trong một cửa sổ (ADR 0011 §9 dòng 12).
    pub register_rate_limit_max_attempts: u32,
    /// Cửa sổ tính rate limit (ms) — chung cho bộ đếm đăng nhập và quota đăng ký.
    pub rate_limit_window_ms: u64,
    /// Trần số bucket rate limit giữ trong RAM (ghi chú review PR #85).
    pub rate_limit_max_buckets: usize,
    /// Tham số Argon2id (ADR 0011 §5).
    pub argon2: Argon2Settings,
    /// Số ký tự tối thiểu của mật khẩu (ADR 0011 §4).
    pub password_min_chars: usize,
    /// Số byte UTF-8 tối đa của mật khẩu (ADR 0011 §4).
    pub password_max_bytes: usize,
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
/// Hạn handshake+auth mặc định: 10 giây.
pub const DEFAULT_AUTH_DEADLINE_MS: u64 = 10_000;
/// Thư mục bundle mặc định (bundle do `boom-assets bundle` dựng).
pub const DEFAULT_BUNDLE_DIR: &str = "data/bundle/village00";
/// File rules profile mặc định.
pub const DEFAULT_RULES_PATH: &str = "data/rules/v1-dev.toml";
/// File bảng phân loại ô mặc định (map V1).
pub const DEFAULT_MAP_PROFILE_PATH: &str = "data/maps/village00-kr.toml";

impl Default for OperationalConfig {
    fn default() -> Self {
        // Mặc định của phần auth lấy **đúng** từ `AuthConfig::default()` (ADR 0011 §5) để
        // không có hai nguồn giá trị cho cùng một tham số.
        let auth = AuthConfig::default();
        Self {
            admin_addr: DEFAULT_ADMIN_ADDR.parse().expect("địa chỉ admin mặc định"),
            data_dir: PathBuf::from("data"),
            db_path: PathBuf::from("data/boom-server.sqlite3"),
            expected_db_schema_version: 1,
            auth_deadline_ms: DEFAULT_AUTH_DEADLINE_MS,
            listen_addrs: vec![DEFAULT_LISTEN_ADDR.parse().expect("địa chỉ game mặc định")],
            bundle_dir: PathBuf::from(DEFAULT_BUNDLE_DIR),
            rules_path: PathBuf::from(DEFAULT_RULES_PATH),
            map_profile_path: PathBuf::from(DEFAULT_MAP_PROFILE_PATH),
            replay_dir: None,
            replay_checkpoints: Vec::new(),
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            health_timeout_ms: DEFAULT_HEALTH_TIMEOUT_MS,
            session_ttl_ms: u64::try_from(auth.session_ttl_ms).unwrap_or(u64::MAX),
            session_ping_interval_ms: 15_000,
            session_liveness_timeout_ms: u64::try_from(auth.session_liveness_timeout_ms)
                .unwrap_or(u64::MAX),
            registration_open: auth.registration_open,
            auth_attempts_per_minute: 10,
            auth_attempts_per_ip_per_minute: 20,
            rate_limit_max_failures: auth.rate_limit_max_failures,
            register_rate_limit_max_attempts: auth.register_rate_limit_max_attempts,
            rate_limit_window_ms: u64::try_from(auth.rate_limit_window_ms).unwrap_or(u64::MAX),
            rate_limit_max_buckets: auth.rate_limit_max_buckets,
            argon2: auth.argon2,
            password_min_chars: auth.password_min_chars,
            password_max_bytes: auth.password_max_bytes,
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

/// Lỗi đọc cấu hình vận hành từ biến môi trường hoặc file cấu hình.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Giá trị địa chỉ không parse được.
    Addr { name: &'static str, value: String },
    /// Giá trị số không parse được.
    Number { name: &'static str, value: String },
    /// Giá trị boolean không parse được.
    Bool { name: &'static str, value: String },
    /// Lỗi đọc/ghi file cấu hình.
    File { path: String, message: String },
    /// File cấu hình không parse được (TOML sai hoặc có trường lạ).
    Toml { path: String, message: String },
    /// File cấu hình còn mục đã bỏ (`[characters]`).
    Characters { message: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Addr { name, value } => write!(f, "{name} không phải địa chỉ: {value}"),
            Self::Number { name, value } => write!(f, "{name} không phải số: {value}"),
            Self::Bool { name, value } => write!(f, "{name} không phải true/false: {value}"),
            Self::File { path, message } => write!(f, "lỗi file cấu hình {path}: {message}"),
            Self::Toml { path, message } => {
                write!(f, "file cấu hình {path} sai định dạng: {message}")
            }
            Self::Characters { message } => {
                write!(f, "mục `[characters]` không còn được chấp nhận: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl OperationalConfig {
    /// Cấu hình hiệu lực: mặc định → file cấu hình → biến môi trường.
    ///
    /// Biến môi trường: `BOOM_ADMIN_ADDR`, `BOOM_DATA_DIR`, `BOOM_DB_PATH`,
    /// `BOOM_EXPECTED_DB_SCHEMA_VERSION`, `BOOM_AUTH_DEADLINE_MS`, `BOOM_LISTEN_ADDR`,
    /// `BOOM_BUNDLE_DIR`, `BOOM_RULES_PATH`, `BOOM_MAP_PROFILE_PATH`, `BOOM_REPLAY_DIR`,
    /// `BOOM_SERVER_VERSION`, `BOOM_HEARTBEAT_INTERVAL_MS`, `BOOM_HEALTH_TIMEOUT_MS`,
    /// `BOOM_SESSION_TTL_MS`, `BOOM_SESSION_PING_INTERVAL_MS`,
    /// `BOOM_SESSION_LIVENESS_TIMEOUT_MS`, `BOOM_REGISTRATION_OPEN`,
    /// `BOOM_RATE_LIMIT_MAX_FAILURES`, `BOOM_REGISTER_RATE_LIMIT_MAX_ATTEMPTS`,
    /// `BOOM_RATE_LIMIT_WINDOW_MS`, `BOOM_RATE_LIMIT_MAX_BUCKETS`,
    /// `BOOM_PASSWORD_MIN_CHARS`, `BOOM_PASSWORD_MAX_BYTES`, `BOOM_ARGON2_M_COST`,
    /// `BOOM_ARGON2_T_COST`, `BOOM_ARGON2_P_COST`, `BOOM_ARGON2_OUTPUT_LEN`,
    /// `BOOM_ARGON2_SALT_LEN`.
    ///
    /// `BOOM_CONFIG_FILE` trỏ tới file TOML: giá trị trong file bị biến môi trường ghi đè.
    pub fn from_env() -> Result<Self, ConfigError> {
        match env_value("BOOM_CONFIG_FILE") {
            Some(path) => Self::from_file_with_env(Path::new(&path)),
            None => {
                let mut config = Self::default();
                config.apply_env()?;
                Ok(config)
            }
        }
    }

    /// Đọc file cấu hình rồi áp biến môi trường (biến môi trường thắng file).
    ///
    /// Dùng cho `boom-server serve --config <file>`: thứ tự mặc định → file → biến môi
    /// trường giữ nguyên như [`Self::from_env`].
    pub fn from_file_with_env(path: &Path) -> Result<Self, ConfigError> {
        let mut config = Self::from_file(path)?;
        config.apply_env()?;
        Ok(config)
    }

    /// Áp biến môi trường `BOOM_*` lên cấu hình đang có.
    fn apply_env(&mut self) -> Result<(), ConfigError> {
        let config = self;
        if let Some(value) = env_value("BOOM_ADMIN_ADDR") {
            config.admin_addr = value.parse().map_err(|_| ConfigError::Addr {
                name: "BOOM_ADMIN_ADDR",
                value,
            })?;
        }
        if let Some(value) = env_value("BOOM_DATA_DIR") {
            config.data_dir = value.clone().into();
            config.db_path = Path::new(&value).join("boom-server.sqlite3");
        }
        if let Some(value) = env_value("BOOM_DB_PATH") {
            config.db_path = value.into();
        }
        if let Some(value) = env_number_u32("BOOM_EXPECTED_DB_SCHEMA_VERSION")? {
            config.expected_db_schema_version = value;
        }
        if let Some(value) = env_number("BOOM_AUTH_DEADLINE_MS")? {
            config.auth_deadline_ms = value;
        }
        if let Some(value) = env_value("BOOM_LISTEN_ADDR") {
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
        if let Some(value) = env_value("BOOM_BUNDLE_DIR") {
            config.bundle_dir = value.into();
        }
        if let Some(value) = env_value("BOOM_RULES_PATH") {
            config.rules_path = value.into();
        }
        if let Some(value) = env_value("BOOM_MAP_PROFILE_PATH") {
            config.map_profile_path = value.into();
        }
        if let Some(value) = env_value("BOOM_REPLAY_DIR") {
            config.replay_dir = Some(value.into());
        }
        if let Some(value) = env_value("BOOM_SERVER_VERSION") {
            config.server_version = value;
        }
        if let Some(value) = env_number("BOOM_HEARTBEAT_INTERVAL_MS")? {
            config.heartbeat_interval_ms = value;
        }
        if let Some(value) = env_number("BOOM_HEALTH_TIMEOUT_MS")? {
            config.health_timeout_ms = value;
        }
        if let Some(value) = env_number("BOOM_SESSION_TTL_MS")? {
            config.session_ttl_ms = value;
        }
        if let Some(value) = env_number("BOOM_SESSION_PING_INTERVAL_MS")? {
            config.session_ping_interval_ms = value;
        }
        if let Some(value) = env_number("BOOM_SESSION_LIVENESS_TIMEOUT_MS")? {
            config.session_liveness_timeout_ms = value;
        }
        if let Some(value) = env_bool("BOOM_REGISTRATION_OPEN")? {
            config.registration_open = value;
        }
        if let Some(value) = env_number_u32("BOOM_RATE_LIMIT_MAX_FAILURES")? {
            config.rate_limit_max_failures = value;
        }
        if let Some(value) = env_number_u32("BOOM_REGISTER_RATE_LIMIT_MAX_ATTEMPTS")? {
            config.register_rate_limit_max_attempts = value;
        }
        if let Some(value) = env_number("BOOM_RATE_LIMIT_WINDOW_MS")? {
            config.rate_limit_window_ms = value;
        }
        if let Some(value) = env_number_usize("BOOM_RATE_LIMIT_MAX_BUCKETS")? {
            config.rate_limit_max_buckets = value;
        }
        if let Some(value) = env_number_usize("BOOM_PASSWORD_MIN_CHARS")? {
            config.password_min_chars = value;
        }
        if let Some(value) = env_number_usize("BOOM_PASSWORD_MAX_BYTES")? {
            config.password_max_bytes = value;
        }
        if let Some(value) = env_number_u32("BOOM_ARGON2_M_COST")? {
            config.argon2.m_cost = value;
        }
        if let Some(value) = env_number_u32("BOOM_ARGON2_T_COST")? {
            config.argon2.t_cost = value;
        }
        if let Some(value) = env_number_u32("BOOM_ARGON2_P_COST")? {
            config.argon2.p_cost = value;
        }
        if let Some(value) = env_number_usize("BOOM_ARGON2_OUTPUT_LEN")? {
            config.argon2.output_len = value;
        }
        if let Some(value) = env_number_usize("BOOM_ARGON2_SALT_LEN")? {
            config.argon2.salt_len = value;
        }
        Ok(())
    }

    /// Đọc cấu hình từ file TOML.
    ///
    /// Trường không có trong file thì giữ **mặc định**; trường lạ thì báo lỗi (để không
    /// âm thầm bỏ qua một chính tả trong cấu hình).
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|err| ConfigError::File {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        let file: ConfigFile = toml::from_str(&text).map_err(|err| ConfigError::Toml {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        Self::from_file_sections(file)
    }

    /// Cấu hình hiệu lực dưới dạng TOML (dùng để xuất hoặc ghi file cấu hình).
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        let file = ConfigFile::from_config(self);
        toml::to_string(&file).map_err(|err| ConfigError::Toml {
            path: "<hiệu lực>".to_owned(),
            message: err.to_string(),
        })
    }

    /// Ghi cấu hình hiệu lực ra file TOML (ghi đè file đã có).
    pub fn write_to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let text = self.to_toml_string()?;
        std::fs::write(path, text).map_err(|err| ConfigError::File {
            path: path.display().to_string(),
            message: err.to_string(),
        })
    }

    /// Cấu hình auth tương ứng (ADR 0011 §5, §6).
    ///
    /// Mọi trường lấy từ [`OperationalConfig`]; phần còn lại giữ mặc định của
    /// [`AuthConfig::default`].
    pub fn auth_config(&self) -> AuthConfig {
        let defaults = AuthConfig::default();
        AuthConfig {
            session_ttl_ms: i64::try_from(self.session_ttl_ms).unwrap_or(defaults.session_ttl_ms),
            session_liveness_timeout_ms: i64::try_from(self.session_liveness_timeout_ms)
                .unwrap_or(defaults.session_liveness_timeout_ms),
            rate_limit_max_failures: self.rate_limit_max_failures,
            register_rate_limit_max_attempts: self.register_rate_limit_max_attempts,
            rate_limit_window_ms: i64::try_from(self.rate_limit_window_ms)
                .unwrap_or(defaults.rate_limit_window_ms),
            rate_limit_max_buckets: self.rate_limit_max_buckets,
            argon2: self.argon2,
            registration_open: self.registration_open,
            password_min_chars: self.password_min_chars,
            password_max_bytes: self.password_max_bytes,
        }
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

    /// Áp các trường của file cấu hình lên trên mặc định.
    ///
    /// Địa chỉ sai trong file ⇒ lỗi rõ ràng (không âm thầm bỏ qua giá trị).
    fn from_file_sections(file: ConfigFile) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let server = file.server;
        if let Some(value) = server.admin_addr {
            config.admin_addr = value.parse().map_err(|_| ConfigError::Addr {
                name: "server.admin_addr",
                value,
            })?;
        }
        if let Some(value) = server.data_dir {
            config.data_dir = value.clone().into();
            config.db_path = Path::new(&value).join("boom-server.sqlite3");
        }
        if let Some(value) = server.listen_addrs {
            config.listen_addrs = value
                .iter()
                .map(|part| {
                    part.trim().parse().map_err(|_| ConfigError::Addr {
                        name: "server.listen_addrs",
                        value: part.clone(),
                    })
                })
                .collect::<Result<_, _>>()?;
        }
        if let Some(value) = server.server_version {
            config.server_version = value;
        }
        if let Some(value) = server.heartbeat_interval_ms {
            config.heartbeat_interval_ms = value;
        }
        if let Some(value) = server.health_timeout_ms {
            config.health_timeout_ms = value;
        }
        if let Some(value) = server.bundle_dir {
            config.bundle_dir = value.into();
        }
        if let Some(value) = server.rules_path {
            config.rules_path = value.into();
        }
        if let Some(value) = server.map_profile_path {
            config.map_profile_path = value.into();
        }
        if let Some(value) = server.replay_dir {
            config.replay_dir = Some(value.into());
        }
        if let Some(value) = server.replay_checkpoints {
            config.replay_checkpoints = value;
        }

        let identity = file.identity;
        if let Some(value) = identity.db_path {
            config.db_path = value.into();
        }
        if let Some(value) = identity.expected_db_schema_version {
            config.expected_db_schema_version = value;
        }
        if let Some(value) = identity.auth_deadline_ms {
            config.auth_deadline_ms = value;
        }
        if let Some(value) = identity.session_ttl_ms {
            config.session_ttl_ms = value;
        }
        if let Some(value) = identity.session_ping_interval_ms {
            config.session_ping_interval_ms = value;
        }
        if let Some(value) = identity.session_liveness_timeout_ms {
            config.session_liveness_timeout_ms = value;
        }

        let auth = file.auth;
        if let Some(value) = auth.registration_open {
            config.registration_open = value;
        }
        if let Some(value) = auth.rate_limit_max_failures {
            config.rate_limit_max_failures = value;
        }
        if let Some(value) = auth.register_rate_limit_max_attempts {
            config.register_rate_limit_max_attempts = value;
        }
        if let Some(value) = auth.rate_limit_window_ms {
            config.rate_limit_window_ms = value;
        }
        if let Some(value) = auth.rate_limit_max_buckets {
            config.rate_limit_max_buckets = value;
        }
        if let Some(value) = auth.password_min_chars {
            config.password_min_chars = value;
        }
        if let Some(value) = auth.password_max_bytes {
            config.password_max_bytes = value;
        }
        if let Some(value) = auth.argon2_m_cost {
            config.argon2.m_cost = value;
        }
        if let Some(value) = auth.argon2_t_cost {
            config.argon2.t_cost = value;
        }
        if let Some(value) = auth.argon2_p_cost {
            config.argon2.p_cost = value;
        }
        if let Some(value) = auth.argon2_output_len {
            config.argon2.output_len = value;
        }
        if let Some(value) = auth.argon2_salt_len {
            config.argon2.salt_len = value;
        }

        // Mục `[characters]` đã bỏ: chỉ số nhân vật là giá trị luật nên nằm ở rules profile.
        if file.characters.is_some() {
            return Err(ConfigError::Characters {
                message: "chỉ số C00/C01 đọc từ rules profile (khóa `characters.c00_*` / \
                          `characters.c01_*` trong file rules); bỏ mục này khỏi file cấu hình"
                    .to_owned(),
            });
        }

        Ok(config)
    }
}

/// Giá trị biến môi trường đã cắt khoảng trắng; `None` nếu không có hoặc rỗng.
fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Giá trị biến môi trường dạng số (`u64`).
fn env_number(name: &'static str) -> Result<Option<u64>, ConfigError> {
    match env_value(name) {
        Some(value) => value
            .parse()
            .map(Some)
            .map_err(|_| ConfigError::Number { name, value }),
        None => Ok(None),
    }
}

/// Giá trị biến môi trường dạng số (`u32`).
fn env_number_u32(name: &'static str) -> Result<Option<u32>, ConfigError> {
    match env_value(name) {
        Some(value) => value
            .parse()
            .map(Some)
            .map_err(|_| ConfigError::Number { name, value }),
        None => Ok(None),
    }
}

/// Giá trị biến môi trường dạng số (`usize`).
fn env_number_usize(name: &'static str) -> Result<Option<usize>, ConfigError> {
    match env_value(name) {
        Some(value) => value
            .parse()
            .map(Some)
            .map_err(|_| ConfigError::Number { name, value }),
        None => Ok(None),
    }
}

/// Giá trị biến môi trường dạng boolean.
fn env_bool(name: &'static str) -> Result<Option<bool>, ConfigError> {
    match env_value(name) {
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(Some(true)),
            "0" | "false" | "no" | "off" => Ok(Some(false)),
            _ => Err(ConfigError::Bool { name, value }),
        },
        None => Ok(None),
    }
}

/// File cấu hình TOML: mỗi trường tuỳ chọn, trường lạ thì báo lỗi.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    server: ServerSection,
    #[serde(default)]
    identity: IdentitySection,
    #[serde(default)]
    auth: AuthSection,
    /// Mục `[characters]` **đã bỏ**: giữ tên để báo lỗi rõ (kèm cách sửa) thay vì lỗi
    /// "unknown field" của serde.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    characters: Option<toml::Value>,
}

/// Phần `[server]` của file cấu hình.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ServerSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    admin_addr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    listen_addrs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    server_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    heartbeat_interval_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    health_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bundle_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rules_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map_profile_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay_checkpoints: Option<Vec<u32>>,
}

/// Phần `[identity]` của file cấu hình.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IdentitySection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    db_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_db_schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth_deadline_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_ttl_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_ping_interval_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_liveness_timeout_ms: Option<u64>,
}

/// Phần `[auth]` của file cấu hình.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    registration_open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rate_limit_max_failures: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    register_rate_limit_max_attempts: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rate_limit_window_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rate_limit_max_buckets: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password_min_chars: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password_max_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    argon2_m_cost: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    argon2_t_cost: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    argon2_p_cost: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    argon2_output_len: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    argon2_salt_len: Option<usize>,
}

impl ConfigFile {
    /// Dựng file cấu hình từ cấu hình hiệu lực (để ghi ra file).
    fn from_config(config: &OperationalConfig) -> Self {
        Self {
            server: ServerSection {
                admin_addr: Some(config.admin_addr.to_string()),
                data_dir: Some(config.data_dir.display().to_string()),
                listen_addrs: Some(
                    config
                        .listen_addrs
                        .iter()
                        .map(|addr| addr.to_string())
                        .collect(),
                ),
                server_version: Some(config.server_version.clone()),
                heartbeat_interval_ms: Some(config.heartbeat_interval_ms),
                health_timeout_ms: Some(config.health_timeout_ms),
                bundle_dir: Some(config.bundle_dir.display().to_string()),
                rules_path: Some(config.rules_path.display().to_string()),
                map_profile_path: Some(config.map_profile_path.display().to_string()),
                replay_dir: config
                    .replay_dir
                    .as_ref()
                    .map(|dir| dir.display().to_string()),
                replay_checkpoints: Some(config.replay_checkpoints.clone()),
            },
            identity: IdentitySection {
                db_path: Some(config.db_path.display().to_string()),
                expected_db_schema_version: Some(config.expected_db_schema_version),
                auth_deadline_ms: Some(config.auth_deadline_ms),
                session_ttl_ms: Some(config.session_ttl_ms),
                session_ping_interval_ms: Some(config.session_ping_interval_ms),
                session_liveness_timeout_ms: Some(config.session_liveness_timeout_ms),
            },
            auth: AuthSection {
                registration_open: Some(config.registration_open),
                rate_limit_max_failures: Some(config.rate_limit_max_failures),
                register_rate_limit_max_attempts: Some(config.register_rate_limit_max_attempts),
                rate_limit_window_ms: Some(config.rate_limit_window_ms),
                rate_limit_max_buckets: Some(config.rate_limit_max_buckets),
                password_min_chars: Some(config.password_min_chars),
                password_max_bytes: Some(config.password_max_bytes),
                argon2_m_cost: Some(config.argon2.m_cost),
                argon2_t_cost: Some(config.argon2.t_cost),
                argon2_p_cost: Some(config.argon2.p_cost),
                argon2_output_len: Some(config.argon2.output_len),
                argon2_salt_len: Some(config.argon2.salt_len),
            },
            characters: None,
        }
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

/// Metadata bất biến cần để ghi replay round thật (V1-026).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaySetup {
    /// Đường dẫn rules profile dùng để `boom-replay` nạp lại.
    pub rules_path: std::path::PathBuf,
    /// `RulesProfile.profile_id`; golden replay so khớp id này.
    pub profile_id: String,
    /// Các tick cần ghi checkpoint hash.
    pub checkpoints: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::{ConfigError, OperationalConfig};
    use crate::auth::AuthConfig;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn auth_config_defaults_match_auth_config_default() {
        // Mặc định của server và mặc định của tầng auth là **một** nguồn (ADR 0011 §5).
        assert_eq!(
            OperationalConfig::default().auth_config(),
            AuthConfig::default()
        );
    }

    #[test]
    fn auth_config_uses_operational_values() {
        let config = OperationalConfig {
            session_ttl_ms: 1_000,
            registration_open: false,
            rate_limit_max_failures: 9,
            register_rate_limit_max_attempts: 3,
            rate_limit_window_ms: 2_000,
            rate_limit_max_buckets: 7,
            password_min_chars: 12,
            password_max_bytes: 64,
            ..OperationalConfig::default()
        };
        let auth = config.auth_config();
        assert_eq!(auth.session_ttl_ms, 1_000);
        assert!(!auth.registration_open);
        assert_eq!(auth.rate_limit_max_failures, 9);
        assert_eq!(auth.register_rate_limit_max_attempts, 3);
        assert_eq!(auth.rate_limit_window_ms, 2_000);
        assert_eq!(auth.rate_limit_max_buckets, 7);
        assert_eq!(auth.password_min_chars, 12);
        assert_eq!(auth.password_max_bytes, 64);
    }

    #[test]
    fn config_file_round_trip_keeps_effective_values() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        let config = OperationalConfig {
            rate_limit_max_buckets: 128,
            registration_open: false,
            db_path: dir.path().join("db.sqlite3"),
            ..OperationalConfig::default()
        };
        config.write_to_file(&path).expect("ghi file cấu hình");

        let read_back = OperationalConfig::from_file(&path).expect("đọc file cấu hình");
        assert_eq!(read_back, config);
        assert_eq!(read_back.auth_config().rate_limit_max_buckets, 128);
        assert!(!read_back.auth_config().registration_open);
    }

    #[test]
    fn config_file_missing_fields_fall_back_to_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        std::fs::write(&path, "[server]\nserver_version = \"test-1\"\n").expect("ghi file");
        let config = OperationalConfig::from_file(&path).expect("đọc file cấu hình");
        assert_eq!(config.server_version, "test-1");
        // Trường không có trong file ⇒ giữ mặc định (kể cả phần auth).
        assert_eq!(config.auth_config(), AuthConfig::default());
    }

    #[test]
    fn config_file_reads_auth_section() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        std::fs::write(
            &path,
            "[auth]\nregistration_open = false\nrate_limit_max_failures = 2\n\
             rate_limit_max_buckets = 16\nargon2_t_cost = 3\n",
        )
        .expect("ghi file");
        let config = OperationalConfig::from_file(&path).expect("đọc file cấu hình");
        let auth = config.auth_config();
        assert!(!auth.registration_open);
        assert_eq!(auth.rate_limit_max_failures, 2);
        assert_eq!(auth.rate_limit_max_buckets, 16);
        assert_eq!(auth.argon2.t_cost, 3);
        // Phần không nêu trong file giữ mặc định.
        assert_eq!(auth.register_rate_limit_max_attempts, 5);
    }

    #[test]
    fn config_file_rejects_unknown_field() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        std::fs::write(&path, "[auth]\nkhong_ton_tai = 1\n").expect("ghi file");
        let error = OperationalConfig::from_file(&path).unwrap_err();
        assert!(matches!(error, ConfigError::Toml { .. }), "{error:?}");
    }

    #[test]
    fn missing_config_file_is_reported() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("khong-co.toml");
        let error = OperationalConfig::from_file(&path).unwrap_err();
        assert!(matches!(error, ConfigError::File { .. }), "{error:?}");
    }

    #[test]
    fn config_file_reads_content_paths() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        std::fs::write(
            &path,
            "[server]\n\
             bundle_dir = \"data/bundle/village00\"\n\
             rules_path = \"data/rules/v1-dev.toml\"\n\
             map_profile_path = \"data/maps/village00-kr.toml\"\n\
             replay_dir = \"data/replay\"\n\
             replay_checkpoints = [1, 16, 32]\n\
             \n\
             [identity]\n\
             auth_deadline_ms = 12345\n",
        )
        .expect("ghi file");
        let config = OperationalConfig::from_file(&path).expect("đọc file cấu hình");
        assert_eq!(config.bundle_dir, PathBuf::from("data/bundle/village00"));
        assert_eq!(config.rules_path, PathBuf::from("data/rules/v1-dev.toml"));
        assert_eq!(
            config.map_profile_path,
            PathBuf::from("data/maps/village00-kr.toml")
        );
        assert_eq!(config.replay_dir, Some(PathBuf::from("data/replay")));
        assert_eq!(config.replay_checkpoints, vec![1, 16, 32]);
        assert_eq!(config.auth_deadline_ms, 12_345);
    }

    #[test]
    fn config_file_rejects_characters_section() {
        // Chỉ số nhân vật là **giá trị luật**: nằm ở rules profile, không phải ở file cấu hình.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        std::fs::write(
            &path,
            "[characters.c00]\n\
             speed_value = 5\nspeed_max = 7\n\
             bombs_value = 1\nbombs_max = 10\n\
             range_value = 1\nrange_max = 7\n",
        )
        .expect("ghi file");
        let error = OperationalConfig::from_file(&path).unwrap_err();
        assert!(matches!(error, ConfigError::Characters { .. }), "{error:?}");
        // Thông điệp phải chỉ ra cách sửa (đọc từ rules profile).
        assert!(error.to_string().contains("rules profile"), "{error}");
    }

    #[test]
    fn config_file_with_env_keeps_env_priority() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("server.toml");
        std::fs::write(&path, "[server]\nserver_version = \"tu-file\"\n").expect("ghi file");
        // Biến môi trường ghi đè giá trị trong file (không cần test khác đọc env).
        let config = OperationalConfig::from_file(&path).expect("đọc file cấu hình");
        assert_eq!(config.server_version, "tu-file");
    }
}
