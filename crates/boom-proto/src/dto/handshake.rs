//! Handshake (ADR 0009 §1): `Hello`, `Welcome`, `Reject`.
//!
//! Bốn định danh khác nhau, không gộp: `wire_schema`, `state_schema`, `rules_profile_id` +
//! `rules_hash`, `content_sha256`. V1 chỉ chấp nhận `wire_schema = 1` khớp chính xác; server so
//! với profile/bundle **của mình**, không dùng hash client khai để chọn nội dung.

use serde::{Deserialize, Serialize};

use crate::dto::{check_schema, de_capabilities};
use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::ProtocolLimits;
use crate::message::Message;

/// `Hello` (C→S): client khai định danh của mình và capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Cách đọc envelope + codec + nghĩa message; V1 là `1`.
    pub wire_schema: u16,
    /// Bố cục DTO `Snapshot`; V1 là `1`.
    pub state_schema: u16,
    /// ID profile rules để server đọc.
    pub rules_profile_id: String,
    /// Hash định danh profile rules (`fnv1a64-v0`).
    pub rules_hash: u64,
    /// Hash manifest content đã chuẩn hóa; chỉ trao đổi đầy đủ ở handshake.
    pub content_sha256: [u8; 32],
    /// Build của client, để log và thông báo lỗi.
    pub client_build: String,
    /// Danh sách capability client chọn.
    #[serde(deserialize_with = "de_capabilities")]
    pub capabilities: Vec<String>,
}

/// `Welcome` (S→C): server chấp nhận định danh nào và cấp phiên.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    /// ID **kết nối** do server cấp trước auth (ADR 0011 §6.1 gọi là `connection_id`).
    /// Không phải phiên tài khoản và không phải token.
    pub session_id: String,
    /// ID người chơi trong phiên.
    pub player_id: u8,
    /// `wire_schema` server chấp nhận.
    pub wire_schema: u16,
    /// `state_schema` server chấp nhận.
    pub state_schema: u16,
    /// Build id của server (fingerprint ADR 0011 §8.3).
    pub server_version: String,
    /// Profile rules server dùng.
    pub rules_profile_id: String,
    /// Hash định danh profile rules server dùng.
    pub rules_hash: u64,
    /// SHA-256 manifest content server đã nạp (fingerprint ADR 0011 §8.3).
    pub content_sha256: [u8; 32],
    /// Số content epoch của lần nạp content; chỉ có nghĩa trong phiên.
    pub content_epoch: u32,
    /// Chu kỳ gửi snapshot, ms.
    pub snapshot_interval_ms: u32,
    /// Revision của bộ giới hạn giao thức server dùng.
    pub protocol_limits_version: u32,
}

/// Lý do server từ chối handshake hoặc phiên (ADR 0009 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    /// Client nói `wire_schema` server không hỗ trợ.
    UnsupportedWireSchema,
    /// Client nói `state_schema` server không hỗ trợ.
    UnsupportedStateSchema,
    /// `rules_profile_id`/`rules_hash` không khớp profile của server.
    RulesMismatch,
    /// `content_sha256` không khớp bundle của server.
    ContentMismatch,
    /// Phiên bị từ chối vì lý do khác (chi tiết trong `Reject::detail`).
    SessionRejected,
}

impl RejectReason {
    /// Mã lý do trên dây (thứ tự khai báo là mã, ADR 0009 §2: message có mã lý do).
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// `Reject` (S→C): lỗi handshake/session kèm mã lý do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reject {
    /// Mã lý do.
    pub reason: RejectReason,
    /// Chi tiết để hiển thị/log; không phải dữ liệu gameplay.
    pub detail: String,
}

impl Hello {
    /// Kiểm DTO sau khi decode.
    ///
    /// `wire_schema` trong payload **không** bị từ chối ở đây: server phải đọc được envelope
    /// schema 1 thì mới gửi được `Reject` (ADR 0009 §1).
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Message for Hello {
    const KIND: MessageKind = MessageKind::Hello;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Hello::check(self, limits)
    }
}

impl Message for Welcome {
    const KIND: MessageKind = MessageKind::Welcome;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        // Server chỉ được chấp nhận đúng schema của V1 (ADR 0009 §1).
        check_schema("wire_schema", self.wire_schema)?;
        check_schema("state_schema", self.state_schema)?;
        if self.server_version.len() > limits.max_name_bytes {
            return Err(ProtoError::Invalid {
                field: "server_version",
                detail: format!(
                    "{} byte vượt trần {} byte",
                    self.server_version.len(),
                    limits.max_name_bytes
                ),
            });
        }
        Ok(())
    }
}

impl Message for Reject {
    const KIND: MessageKind = MessageKind::Reject;

    fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::ProtocolLimits;

    #[test]
    fn reject_reason_codes_are_stable() {
        assert_eq!(RejectReason::UnsupportedWireSchema.code(), 0);
        assert_eq!(RejectReason::UnsupportedStateSchema.code(), 1);
        assert_eq!(RejectReason::RulesMismatch.code(), 2);
        assert_eq!(RejectReason::ContentMismatch.code(), 3);
        assert_eq!(RejectReason::SessionRejected.code(), 4);
    }

    #[test]
    fn hello_with_other_schema_is_not_rejected_by_decoder() {
        // Server phải gửi được `Reject`, nên decoder không từ chối payload lệch schema.
        let hello = Hello {
            wire_schema: 2,
            state_schema: 1,
            rules_profile_id: "p".into(),
            rules_hash: 7,
            content_sha256: [0; 32],
            client_build: "b".into(),
            capabilities: vec!["snapshot".into()],
        };
        assert!(hello.check(&ProtocolLimits::v1()).is_ok());
    }
}
