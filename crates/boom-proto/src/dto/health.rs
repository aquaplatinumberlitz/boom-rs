//! Health runtime qua admin listener loopback (ADR 0011 §8.2.1; ADR 0009 Addendum A §A.1).
//!
//! Admin listener nói **cùng** envelope/kind của `boom-proto` và chỉ nhận **một** kind:
//! `HealthRequest` (kind `0x0050`, C→S) → trả `HealthResponse` (kind `0x0051`, S→C). Mọi kind
//! khác → đóng kết nối, log `server.health_rejected`. Không mở HTTP.
//!
//! Admin listener chỉ bind loopback và chỉ trả **số liệu vận hành**: không có bí mật, không có
//! danh sách tài khoản, không có token (ADR 0011 §8.2.1).

use serde::{Deserialize, Serialize};

use crate::dto::de_listen_addrs;
use crate::dto::de_problems;
use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::ProtocolLimits;
use crate::message::Message;

/// `HealthRequest` (C→S, kind `0x0050`, admin loopback): cây hỏi sức khoẻ runtime.
///
/// Không có payload: `HealthResponse` mang đúng danh sách trường của ADR 0011 §8.2.1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthRequest {}

/// Trạng thái sức khoẻ (ADR 0011 §8.2.1).
///
/// Thứ tự khai báo là mã trên dây (postcard encode variant theo thứ tự khai báo): đổi thứ tự là
/// đổi nghĩa message.
///
/// `Unreachable`/`Timeout` **không** xuất hiện trong `HealthResponse`: chúng là kết quả của CLI
/// khi không liên lạc được server, không phải câu trả lời của server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Server trả lời và mọi kiểm đều xanh.
    Healthy,
    /// Server trả lời nhưng có kiểm fail (`db_ok = false`, integrity lỗi, schema lệch, ...).
    Unhealthy,
    /// Không liên lạc được server (chỉ có ở output của CLI).
    Unreachable,
    /// Server không trả lời trong thời hạn (chỉ có ở output của CLI).
    Timeout,
}

impl HealthStatus {
    /// Mã trạng thái trên dây.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Tên trạng thái như trong JSON của CLI (ADR 0011 §8.2.1).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Unhealthy => "unhealthy",
            Self::Unreachable => "unreachable",
            Self::Timeout => "timeout",
        }
    }
}

/// `HealthResponse` (S→C, kind `0x0051`, admin loopback): số liệu vận hành của tiến trình server.
///
/// Trường đúng theo danh sách ADR 0011 §8.2.1. `uptime_ms`/`listen_addrs`/`last_error_ts` lấy
/// **từ tiến trình server**, không phải từ CLI — đó là phần thiết kế cũ thiếu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Trạng thái sức khoẻ.
    pub status: HealthStatus,
    /// Thời gian server đã chạy, ms.
    pub uptime_ms: u64,
    /// Mốc khởi động, Unix epoch milliseconds.
    pub started_at_ms: u64,
    /// Boot id của tiến trình server (ADR 0011 §6.1).
    pub instance_id: String,
    /// Địa chỉ server đang bind.
    #[serde(deserialize_with = "de_listen_addrs")]
    pub listen_addrs: Vec<String>,
    /// Địa chỉ admin listener (loopback).
    pub admin_addr: String,
    /// Mốc lỗi gần nhất, `None` khi không có lỗi.
    pub last_error_ts: Option<u64>,
    /// DB còn dùng được không.
    pub db_ok: bool,
    /// `PRAGMA user_version` hiện tại của file DB.
    pub schema_version: u32,
    /// `PRAGMA user_version` mà binary đang chạy mong đợi.
    pub expected_schema_version: u32,
    /// Kết quả `PRAGMA integrity_check` (`"ok"` khi lành).
    pub integrity_check: String,
    /// Profile rules đang chạy.
    pub rules_profile_id: String,
    /// Hash định danh profile rules.
    pub rules_hash: u64,
    /// Hash manifest content đã chuẩn hóa.
    pub content_sha256: [u8; 32],
    /// Số tài khoản trong DB.
    pub accounts: u64,
    /// Số phiên đang hiệu lực.
    pub active_sessions: u64,
    /// Danh sách vấn đề; **rỗng** khi `status = Healthy` (ADR 0011 §8.2.1).
    #[serde(deserialize_with = "de_problems")]
    pub problems: Vec<String>,
}

impl HealthRequest {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl HealthResponse {
    /// Kiểm DTO sau khi decode: miền giá trị của `HealthResponse` trên wire.
    ///
    /// Ba kiểm, theo đúng nghĩa ADR 0011 §8.2.1:
    ///
    /// 1. `status` trên wire chỉ có `Healthy`/`Unhealthy`. `Unreachable`/`Timeout` là **kết quả
    ///    của CLI** khi không liên lạc được server, không phải câu trả lời của server.
    /// 2. Số mục `problems` trong trần của phiên.
    /// 3. `Healthy` ⇒ `problems` **rỗng**: exit code `0` của `boom-server health` đòi
    ///    `status = "healthy"` và `problems` rỗng (ADR 0011 §8.2.1). `Unhealthy` thì `problems`
    ///    phải nêu vấn đề, nhưng `db_ok`/`integrity_check`/`schema_version` lệch cũng đủ để
    ///    `unhealthy`, nên `problems` rỗng vẫn hợp lệ khi `Unhealthy`.
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if matches!(
            self.status,
            HealthStatus::Unreachable | HealthStatus::Timeout
        ) {
            return Err(ProtoError::Invalid {
                field: "status",
                detail: format!(
                    "{} là kết quả của CLI, không phải trạng thái HealthResponse trên wire",
                    self.status.name()
                ),
            });
        }
        if self.problems.len() > limits.max_problems {
            return Err(ProtoError::CountLimit {
                field: "problems",
                len: self.problems.len(),
                max: limits.max_problems,
            });
        }
        if self.status == HealthStatus::Healthy && !self.problems.is_empty() {
            return Err(ProtoError::Invalid {
                field: "problems",
                detail: format!(
                    "status = healthy thì problems phải rỗng, có {} mục",
                    self.problems.len()
                ),
            });
        }
        Ok(())
    }
}

impl Message for HealthRequest {
    const KIND: MessageKind = MessageKind::HealthRequest;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        HealthRequest::check(self, limits)
    }
}

impl Message for HealthResponse {
    const KIND: MessageKind = MessageKind::HealthResponse;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        HealthResponse::check(self, limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> HealthResponse {
        HealthResponse {
            status: HealthStatus::Healthy,
            uptime_ms: 1_234,
            started_at_ms: 1_700_000_000_000,
            instance_id: "0123456789abcdef".into(),
            listen_addrs: vec!["0.0.0.0:17889".into()],
            admin_addr: "127.0.0.1:17890".into(),
            last_error_ts: None,
            db_ok: true,
            schema_version: 1,
            expected_schema_version: 1,
            integrity_check: "ok".into(),
            rules_profile_id: "kr00967-v1-dev".into(),
            rules_hash: 0x1234_5678_9abc_def0,
            content_sha256: [7u8; 32],
            accounts: 3,
            active_sessions: 1,
            problems: vec![],
        }
    }

    #[test]
    fn status_codes_are_stable() {
        assert_eq!(HealthStatus::Healthy.code(), 0);
        assert_eq!(HealthStatus::Unhealthy.code(), 1);
        assert_eq!(HealthStatus::Unreachable.code(), 2);
        assert_eq!(HealthStatus::Timeout.code(), 3);
    }

    #[test]
    fn status_names_match_adr_0011() {
        assert_eq!(HealthStatus::Healthy.name(), "healthy");
        assert_eq!(HealthStatus::Unhealthy.name(), "unhealthy");
        assert_eq!(HealthStatus::Unreachable.name(), "unreachable");
        assert_eq!(HealthStatus::Timeout.name(), "timeout");
    }

    #[test]
    fn healthy_response_has_no_problems() {
        let limits = ProtocolLimits::v1();
        assert!(response().check(&limits).is_ok());
    }

    /// `Unreachable`/`Timeout` là kết quả của CLI, không phải trạng thái trên wire.
    #[test]
    fn cli_only_status_is_rejected() {
        let limits = ProtocolLimits::v1();
        for status in [HealthStatus::Unreachable, HealthStatus::Timeout] {
            let mut resp = response();
            resp.status = status;
            let err = resp.check(&limits).unwrap_err();
            assert!(
                matches!(
                    err,
                    ProtoError::Invalid {
                        field: "status",
                        ..
                    }
                ),
                "thực tế: {err:?}"
            );
        }
    }

    /// `healthy` mà `problems` không rỗng thì không hợp lệ (ADR 0011 §8.2.1, exit code 0).
    #[test]
    fn healthy_with_problems_is_rejected() {
        let limits = ProtocolLimits::v1();
        let mut resp = response();
        resp.problems = vec!["db hỏng".into()];
        let err = resp.check(&limits).unwrap_err();
        assert!(
            matches!(
                err,
                ProtoError::Invalid {
                    field: "problems",
                    ..
                }
            ),
            "thực tế: {err:?}"
        );
    }

    /// `unhealthy` kèm `problems` là hợp lệ.
    #[test]
    fn unhealthy_with_problems_is_accepted() {
        let limits = ProtocolLimits::v1();
        let mut resp = response();
        resp.status = HealthStatus::Unhealthy;
        resp.problems = vec!["db hỏng".into()];
        assert!(resp.check(&limits).is_ok());
    }

    /// `unhealthy` mà `problems` **rỗng** vẫn hợp lệ: `db_ok`/`integrity_check`/
    /// `schema_version` lệch cũng đủ để `unhealthy` (ADR 0011 §8.2.1, exit `1`).
    #[test]
    fn unhealthy_without_problems_is_accepted() {
        let limits = ProtocolLimits::v1();
        let mut resp = response();
        resp.status = HealthStatus::Unhealthy;
        resp.db_ok = false;
        resp.integrity_check = "page 3 corrupt".into();
        assert!(resp.check(&limits).is_ok());
    }

    #[test]
    fn problems_over_limit_is_an_error() {
        let limits = ProtocolLimits::v1();
        let mut resp = response();
        resp.status = HealthStatus::Unhealthy;
        resp.problems = (0..limits.max_problems + 1)
            .map(|i| format!("problem {i}"))
            .collect();
        let err = resp.check(&limits).unwrap_err();
        assert!(
            matches!(
                err,
                ProtoError::CountLimit {
                    field: "problems",
                    max: 32,
                    ..
                }
            ),
            "thực tế: {err:?}"
        );
    }
}
