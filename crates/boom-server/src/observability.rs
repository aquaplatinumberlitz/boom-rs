//! Log JSON Lines + heartbeat/health artifact (ADR 0011 §8.1, §8.2.3).
//!
//! Mọi dòng log là một object JSON trên stdout. Trường bắt buộc của ADR 0011 §8.1 được thêm ở
//! đây, không phụ thuộc caller nhớ. Không có API nhận token/mật khẩu/PHC string; `Secret<T>` của
//! identity (khi merge) chỉ được log qua định danh không nhạy cảm.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use crate::config::ConfigFingerprint;

/// Tên sự kiện log ổn định (ADR 0011 §8.1).
///
/// ADR 0011 §8.1 liệt kê một **tập tối thiểu** tên sự kiện; các hằng ở đây giữ **đúng tên**
/// của ADR (kể cả khi issue ghi khác, ví dụ `auth.login_fail` — không phải
/// `auth.login_failed`). Sự kiện ngoài tập tối thiểu được đặt tên riêng và **không** thay
/// thế tên nào đã có trong ADR.
///
/// **Chưa có nguồn phát** (sẽ đi kèm T5 phần 2 — message auth trên wire): `auth.logout`
/// (client đăng xuất) và `session.expired` (job quét TTL của tiến trình server).
pub mod events {
    /// Đăng ký tài khoản thành công.
    pub const AUTH_REGISTER: &str = "auth.register";
    /// Đăng ký bị từ chối (tên đã có, đăng ký đóng, input sai).
    pub const AUTH_REGISTER_FAILED: &str = "auth.register_failed";
    /// Đăng nhập thành công.
    pub const AUTH_LOGIN_OK: &str = "auth.login_ok";
    /// Đăng nhập bị từ chối (sai thông tin, sai định dạng, tài khoản bị vô hiệu hoá).
    pub const AUTH_LOGIN_FAIL: &str = "auth.login_fail";
    /// Vượt ngưỡng tần suất theo tài khoản hoặc theo nguồn.
    pub const AUTH_RATE_LIMITED: &str = "auth.rate_limited";
    /// Đăng nhập bị từ chối vì tài khoản đã có phiên còn sống.
    pub const AUTH_LOGIN_REJECTED_DUPLICATE_SESSION: &str = "auth.login_rejected_duplicate_session";
    /// Lỗi entropy của hệ điều hành: không phát token, không fallback RNG yếu.
    pub const AUTH_RNG_UNAVAILABLE: &str = "auth.rng_unavailable";
    /// Phiên bị thu hồi (logout, disconnect, timeout, lệnh admin).
    pub const SESSION_REVOKED: &str = "session.revoked";
    /// Batch input bị **bỏ** vì client khai slot khác slot server bind từ phiên
    /// (ADR 0009 Addendum B §B.2). Không phải lỗi sequence: client chỉ chưa biết
    /// slot của mình.
    pub const INPUT_SLOT_MISMATCH: &str = "input.slot_mismatch";
}

/// Mức log cố định.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// Các trường tùy ngữ cảnh; caller không bao giờ truyền bí mật.
pub type LogFields = BTreeMap<&'static str, Value>;

/// Logger JSON Lines, thread-safe.
pub struct Logger {
    fingerprint: ConfigFingerprint,
    instance_id: String,
    content_epoch: u32,
    sink: Mutex<Box<dyn Write + Send>>,
}

impl Logger {
    /// Logger ghi ra writer cho trước; test dùng buffer, production dùng stdout.
    pub fn new(
        fingerprint: ConfigFingerprint,
        instance_id: String,
        content_epoch: u32,
        sink: Box<dyn Write + Send>,
    ) -> Self {
        Self {
            fingerprint,
            instance_id,
            content_epoch,
            sink: Mutex::new(sink),
        }
    }

    /// Logger production ghi stdout.
    pub fn stdout(fingerprint: ConfigFingerprint, instance_id: String, content_epoch: u32) -> Self {
        Self::new(
            fingerprint,
            instance_id,
            content_epoch,
            Box::new(io::stdout()),
        )
    }

    pub fn fingerprint(&self) -> &ConfigFingerprint {
        &self.fingerprint
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Ghi một dòng. Các trường bắt buộc luôn có; `content_sha256` chỉ ở sự kiện được ADR cho phép.
    pub fn log(&self, level: LogLevel, event: &str, fields: LogFields) {
        let mut object = Map::new();
        object.insert("ts".into(), json!(rfc3339_utc_ms(now_ms())));
        object.insert("level".into(), json!(level.as_str()));
        object.insert("event".into(), json!(event));
        object.insert(
            "server_version".into(),
            json!(self.fingerprint.server_version),
        );
        object.insert("wire_schema".into(), json!(self.fingerprint.wire_schema));
        object.insert("state_schema".into(), json!(self.fingerprint.state_schema));
        object.insert(
            "protocol_limits_version".into(),
            json!(self.fingerprint.protocol_limits_version),
        );
        object.insert(
            "rules_profile_id".into(),
            json!(self.fingerprint.rules_profile_id),
        );
        object.insert("rules_hash".into(), json!(self.fingerprint.rules_hash));
        object.insert("instance_id".into(), json!(self.instance_id));
        object.insert("content_epoch".into(), json!(self.content_epoch));
        if matches!(event, "server.start" | "content.load_ok") {
            object.insert(
                "content_sha256".into(),
                json!(hex(&self.fingerprint.content_sha256)),
            );
        }
        // Các trường ngữ cảnh bắt buộc theo bảng ADR; null khi không áp dụng.
        for key in [
            "account_id",
            "session_id",
            "player_id",
            "room_id",
            "round_id",
            "revoke_reason",
            "reason",
            "error_code",
            "error_message",
        ] {
            object.insert(key.into(), Value::Null);
        }
        for (key, value) in fields {
            if is_sensitive_key(key) {
                continue;
            }
            object.insert(key.into(), value);
        }
        if let Ok(mut sink) = self.sink.lock() {
            let line = Value::Object(object);
            let _ = writeln!(sink, "{line}");
            let _ = sink.flush();
        }
    }

    pub fn info(&self, event: &str, fields: LogFields) {
        self.log(LogLevel::Info, event, fields);
    }

    pub fn warn(&self, event: &str, fields: LogFields) {
        self.log(LogLevel::Warn, event, fields);
    }

    pub fn error(&self, event: &str, fields: LogFields) {
        self.log(LogLevel::Error, event, fields);
    }

    /// Log khởi động đầy đủ ngón tay cấu hình + địa chỉ thật.
    pub fn server_start(&self, listen_addrs: &[String], admin_addr: &str) {
        let mut fields = LogFields::new();
        fields.insert("listen_addrs", json!(listen_addrs));
        fields.insert("admin_addr", json!(admin_addr));
        self.info("server.start", fields);
    }

    pub fn server_stop(&self, reason: &str) {
        let mut fields = LogFields::new();
        fields.insert("reason", json!(reason));
        self.info("server.stop", fields);
    }

    pub fn heartbeat(&self, uptime_ms: u64) {
        let mut fields = LogFields::new();
        fields.insert("uptime_ms", json!(uptime_ms));
        self.info("server.heartbeat", fields);
    }
}

/// Khóa không được xuất hiện trong log, kể cả khi caller truyền nhầm.
fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("token")
        || key.contains("password")
        || key.contains("passwd")
        || key.contains("authorization")
        || key.contains("secret")
}

/// Buffer dùng chung để test parse JSON Lines.
#[derive(Clone, Default)]
pub struct SharedLogBuffer {
    inner: Arc<Mutex<Vec<u8>>>,
}

impl SharedLogBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.inner.lock().unwrap().clone()
    }

    pub fn lines(&self) -> Vec<Value> {
        String::from_utf8_lossy(&self.bytes())
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("log phải là JSON Lines"))
            .collect()
    }
}

impl Write for SharedLogBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Sinh boot id đơn giản, không phải bí mật; `instance_id` chỉ để đối chiếu log/artifact.
pub fn new_instance_id() -> String {
    format!("{:016x}", now_ms() ^ u64::from(std::process::id()))
}

/// Mốc hiện tại, Unix epoch ms.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// RFC 3339 UTC với mili giây (ADR 0011 §8.1).
pub fn rfc3339_utc_ms(ms: u64) -> String {
    let secs = ms / 1_000;
    let millis = ms % 1_000;
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX);
    let day_secs = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = day_secs / 3_600;
    let minute = (day_secs % 3_600) / 60;
    let second = day_secs % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Howard Hinnant's `civil_from_days`, đủ cho timestamp log, không cần dependency lịch.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + i64::from(m <= 2);
    (
        year,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

/// Hex chữ thường của 32 byte hash.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OperationalConfig;

    #[test]
    fn log_has_required_fields_and_hides_secret_fields() {
        let config = OperationalConfig::default();
        let buffer = SharedLogBuffer::new();
        let logger = Logger::new(
            config.fingerprint(),
            "boot".into(),
            config.content_epoch,
            Box::new(buffer.clone()),
        );
        logger.server_start(&["127.0.0.1:17889".into()], "127.0.0.1:17890");
        let mut fields = LogFields::new();
        fields.insert("token", json!("must-not-leak"));
        fields.insert("password", json!("must-not-leak"));
        logger.info("room.create", fields);
        let lines = buffer.lines();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            for key in [
                "ts",
                "level",
                "event",
                "server_version",
                "wire_schema",
                "state_schema",
                "protocol_limits_version",
                "rules_profile_id",
                "rules_hash",
                "instance_id",
            ] {
                assert!(line.get(key).is_some(), "thiếu {key}: {line}");
            }
            assert!(line.get("token").is_none());
            assert!(line.get("password").is_none());
            assert!(line.get("authorization").is_none());
        }
        assert!(lines[0].get("content_sha256").is_some());
        assert!(lines[1].get("content_sha256").is_none());
    }

    #[test]
    fn timestamp_is_rfc3339_utc() {
        assert_eq!(rfc3339_utc_ms(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            rfc3339_utc_ms(1_700_000_000_123),
            "2023-11-14T22:13:20.123Z"
        );
    }
}
