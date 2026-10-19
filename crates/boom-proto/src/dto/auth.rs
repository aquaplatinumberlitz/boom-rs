//! Auth trên dây (ADR 0009 Addendum A §A.1–§A.2).
//!
//! Thứ tự trên kênh game: `Hello` → `Welcome` → `Auth` → `AuthOk`/`AuthFail`
//! (ADR 0009 Addendum A §A.1). `Auth` chỉ hợp lệ **một lần thành công** mỗi kết nối; `Auth` thứ
//! hai sau `AuthOk` là lỗi giao thức. `Logout` thu hồi đúng phiên của kết nối rồi server đóng
//! kết nối, không trả message nào.
//!
//! Ranh giới giữa crate này và tầng account:
//!
//! - **Giao thức** (crate này): biên độ dài `username`/`password`, mã lý do, token là bearer
//!   credential.
//! - **Tài khoản** (ADR 0011 §4, tầng `boom-server`): tên 3–24 ký tự, mật khẩu ≥ 8 ký tự,
//!   Argon2id, một phiên mỗi tài khoản. Không có luật nào ở đây.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::dto::{de_password, de_username};
use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::ProtocolLimits;
use crate::message::Message;

/// Thao tác auth (ADR 0009 Addendum A §A.2).
///
/// Thứ tự khai báo là mã trên dây (postcard encode variant theo thứ tự khai báo): đổi thứ tự là
/// đổi nghĩa message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthOp {
    /// Đăng nhập tài khoản đã có.
    Login,
    /// Đăng ký tài khoản mới.
    Register,
}

impl AuthOp {
    /// Mã thao tác trên dây.
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// `Auth` (C→S, kind `0x0004`): đăng ký hoặc đăng nhập sau `Welcome`.
///
/// Mật khẩu đi **thô** trên dây: V1 chưa có TLS (ADR 0006), đây là giới hạn đã biết của ADR 0011
/// §6/§14 — V1 chỉ dùng cho LAN/máy local.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Auth {
    /// Thao tác: đăng nhập hay đăng ký.
    pub op: AuthOp,
    /// Tên đăng nhập; giữ nguyên dấu gõ, so khớp theo bản chuẩn hóa của server (ADR 0011 §4).
    #[serde(deserialize_with = "de_username")]
    pub username: String,
    /// Mật khẩu thô; server hash bằng Argon2id và **không bao giờ** ghi vào log (ADR 0011 §5).
    #[serde(deserialize_with = "de_password")]
    pub password: String,
}

/// `AuthOk` (S→C, kind `0x0005`): auth thành công, phát token phiên.
///
/// `Debug` **không** in token: token là bearer credential, rò ra là lộ phiên (ADR 0011 §6).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthOk {
    /// ID tài khoản do server cấp; client không tự khai (ADR 0005).
    pub account_id: u64,
    /// Tên hiển thị của tài khoản.
    pub display_name: String,
    /// Token phiên: base64url không padding, 43 ký tự (ADR 0011 §6).
    pub token: String,
    /// Mốc hết hạn của token, Unix epoch milliseconds (ADR 0011 §6).
    pub expires_at_ms: u64,
}

/// Lý do auth thất bại (ADR 0009 Addendum A §A.2).
///
/// Thứ tự khai báo là mã trên dây (postcard encode variant theo thứ tự khai báo): đổi thứ tự là
/// đổi nghĩa message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthFailReason {
    /// Sai tên hoặc sai mật khẩu — **dùng chung** để không lộ enumeration (ADR 0011 §5).
    InvalidCredentials,
    /// Tên đã có; chỉ trả cho `Register` (đăng ký tự bản chất lộ tên đã có).
    UsernameUnavailable,
    /// Input không hợp lệ: vượt biên độ dài, hoặc sai định dạng tên/mật khẩu.
    InvalidInput,
    /// Tài khoản đã có phiên đang sống (ADR 0011 §6).
    AccountHasActiveSession,
    /// Vượt giới hạn tần suất theo tài khoản hoặc theo nguồn (ADR 0011 §5).
    RateLimited,
    /// Đăng ký đã đóng theo cấu hình server.
    RegistrationClosed,
    /// Lỗi phía server (OS RNG, DB); chi tiết chỉ vào log, không trả cho client.
    Unavailable,
}

impl AuthFailReason {
    /// Mã lý do trên dây.
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// `AuthFail` (S→C, kind `0x0006`): auth thất bại, có mã lý do.
///
/// Sau `AuthFail` client được gửi lại `Auth` trên cùng kết nối, có rate limit
/// (ADR 0009 Addendum A §A.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthFail {
    /// Mã lý do.
    pub reason: AuthFailReason,
}

/// `Logout` (C→S, kind `0x0007`): thu hồi phiên rồi server đóng kết nối.
///
/// Server thu hồi **đúng** phiên của kết nối đó (reason `logout`, ADR 0011 §6.1), không trả
/// message nào.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logout {}

impl fmt::Debug for Auth {
    /// Không in mật khẩu: mật khẩu **không bao giờ** vào log (ADR 0011 §5, §8.1).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Auth")
            .field("op", &self.op)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for AuthOk {
    /// Không in token: token là bearer credential, rò token là lộ phiên (ADR 0011 §6, §9).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthOk")
            .field("account_id", &self.account_id)
            .field("display_name", &self.display_name)
            .field("token", &"<redacted>")
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

impl Auth {
    /// Kiểm DTO sau khi decode: biên độ dài `username`/`password` (ADR 0009 Addendum A §A.2).
    ///
    /// Vượt biên → lỗi, **không** hash: server bắt lỗi này rồi trả `AuthFail { InvalidInput }`.
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.username.len() > limits.max_username_bytes {
            return Err(ProtoError::Invalid {
                field: "username",
                detail: format!(
                    "{} byte vượt trần {} byte",
                    self.username.len(),
                    limits.max_username_bytes
                ),
            });
        }
        if self.password.len() > limits.max_password_bytes {
            return Err(ProtoError::Invalid {
                field: "password",
                detail: format!(
                    "{} byte vượt trần {} byte",
                    self.password.len(),
                    limits.max_password_bytes
                ),
            });
        }
        Ok(())
    }

    /// Lý do vượt biên độ dài, nếu có.
    ///
    /// Server dùng để trả `AuthFail { InvalidInput }` mà không phải đoán lý do từ lỗi decode
    /// (ADR 0009 Addendum A §A.2).
    pub fn limit_violation(&self, limits: &ProtocolLimits) -> Option<AuthFailReason> {
        if self.username.len() > limits.max_username_bytes
            || self.password.len() > limits.max_password_bytes
        {
            Some(AuthFailReason::InvalidInput)
        } else {
            None
        }
    }
}

impl AuthOk {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl AuthFail {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Logout {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Message for Auth {
    const KIND: MessageKind = MessageKind::Auth;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Auth::check(self, limits)
    }
}

impl Message for AuthOk {
    const KIND: MessageKind = MessageKind::AuthOk;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        AuthOk::check(self, limits)
    }
}

impl Message for AuthFail {
    const KIND: MessageKind = MessageKind::AuthFail;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        AuthFail::check(self, limits)
    }
}

impl Message for Logout {
    const KIND: MessageKind = MessageKind::Logout;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Logout::check(self, limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_op_codes_are_stable() {
        assert_eq!(AuthOp::Login.code(), 0);
        assert_eq!(AuthOp::Register.code(), 1);
    }

    #[test]
    fn auth_fail_reason_codes_are_stable() {
        assert_eq!(AuthFailReason::InvalidCredentials.code(), 0);
        assert_eq!(AuthFailReason::UsernameUnavailable.code(), 1);
        assert_eq!(AuthFailReason::InvalidInput.code(), 2);
        assert_eq!(AuthFailReason::AccountHasActiveSession.code(), 3);
        assert_eq!(AuthFailReason::RateLimited.code(), 4);
        assert_eq!(AuthFailReason::RegistrationClosed.code(), 5);
        assert_eq!(AuthFailReason::Unavailable.code(), 6);
    }

    /// `Debug` của `AuthOk` không được chứa token (ADR 0009 Addendum A §A.2).
    #[test]
    fn auth_ok_debug_hides_token() {
        let ok = AuthOk {
            account_id: 7,
            display_name: "player".into(),
            token: "s3cret-token-value".into(),
            expires_at_ms: 1_700_000_000_000,
        };
        let text = format!("{ok:?}");
        assert!(!text.contains("s3cret-token-value"), "thực tế: {text}");
        assert!(text.contains("<redacted>"), "thực tế: {text}");
        // Phần không nhạy cảm vẫn phải hiện để log còn dùng được.
        assert!(text.contains("player"), "thực tế: {text}");
        assert!(text.contains("7"), "thực tế: {text}");
    }

    /// `Debug` của `Auth` không được chứa mật khẩu (ADR 0011 §5).
    #[test]
    fn auth_debug_hides_password() {
        let auth = Auth {
            op: AuthOp::Login,
            username: "player".into(),
            password: "hunter2".into(),
        };
        let text = format!("{auth:?}");
        assert!(!text.contains("hunter2"), "thực tế: {text}");
        assert!(text.contains("player"), "thực tế: {text}");
    }

    #[test]
    fn username_over_limit_is_an_error() {
        let limits = ProtocolLimits::v1();
        let auth = Auth {
            op: AuthOp::Login,
            username: "u".repeat(limits.max_username_bytes + 1),
            password: "p".into(),
        };
        let err = auth.check(&limits).unwrap_err();
        assert!(
            matches!(
                err,
                ProtoError::Invalid {
                    field: "username",
                    ..
                }
            ),
            "thực tế: {err:?}"
        );
        assert_eq!(
            auth.limit_violation(&limits),
            Some(AuthFailReason::InvalidInput)
        );
    }

    #[test]
    fn password_over_limit_is_an_error() {
        let limits = ProtocolLimits::v1();
        let auth = Auth {
            op: AuthOp::Register,
            username: "u".into(),
            password: "p".repeat(limits.max_password_bytes + 1),
        };
        let err = auth.check(&limits).unwrap_err();
        assert!(
            matches!(
                err,
                ProtoError::Invalid {
                    field: "password",
                    ..
                }
            ),
            "thực tế: {err:?}"
        );
        assert_eq!(
            auth.limit_violation(&limits),
            Some(AuthFailReason::InvalidInput)
        );
    }

    #[test]
    fn limits_are_inclusive() {
        let limits = ProtocolLimits::v1();
        let auth = Auth {
            op: AuthOp::Login,
            username: "u".repeat(limits.max_username_bytes),
            password: "p".repeat(limits.max_password_bytes),
        };
        assert!(auth.check(&limits).is_ok());
        assert_eq!(auth.limit_violation(&limits), None);
    }

    /// Biên tính theo **byte** UTF-8, không theo ký tự.
    #[test]
    fn limits_count_bytes_not_chars() {
        let limits = ProtocolLimits::v1();
        // Mỗi ký tự `é` là 2 byte: 33 ký tự = 66 byte > 64.
        let auth = Auth {
            op: AuthOp::Login,
            username: "é".repeat(33),
            password: "p".into(),
        };
        assert!(auth.check(&limits).is_err());
    }
}
