//! Codec: envelope viết tay + payload postcard (ADR 0009 §5).
//!
//! Quy tắc encode/decode:
//!
//! - Encode: envelope trước, payload sau; postcard sinh byte canonical.
//! - Decode: kiểm magic/schema/kind/flags/reserved/len trước; giới hạn frame/snapshot/batch;
//!   từ chối trailing byte; kiểm invariant của DTO sau decode.
//! - Mọi lỗi là giá trị trả về: không panic, không áp dụng một phần.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::envelope::{Envelope, read_envelope};
use crate::error::ProtoError;
use crate::kind::{Channel, MessageKind, Role};
use crate::limits::{ENVELOPE_BYTES, ProtocolLimits};

/// Một message của giao thức V1.
pub trait Message: Serialize + DeserializeOwned {
    /// Kind của message trong envelope.
    const KIND: MessageKind;

    /// Kiểm miền giá trị sau khi decode.
    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError>;
}

/// Encode payload của một message (chưa có envelope).
pub fn encode_payload<M: Message>(message: &M) -> Result<Vec<u8>, ProtoError> {
    postcard::to_allocvec(message).map_err(ProtoError::from)
}

/// Decode payload của một message; từ chối byte thừa sau payload.
pub fn decode_payload<M: Message>(payload: &[u8]) -> Result<M, ProtoError> {
    let (message, rest) = postcard::take_from_bytes::<M>(payload)?;
    if !rest.is_empty() {
        return Err(ProtoError::TrailingBytes(rest.len()));
    }
    Ok(message)
}

/// Encode trọn một frame: envelope 16 byte + payload postcard.
pub fn encode_frame<M: Message>(
    message: &M,
    limits: &ProtocolLimits,
) -> Result<Vec<u8>, ProtoError> {
    let payload = encode_payload(message)?;
    let max = limits.max_payload_bytes(M::KIND);
    if payload.len() > max {
        return Err(ProtoError::PayloadTooLarge {
            len: payload.len(),
            max,
        });
    }
    let total = ENVELOPE_BYTES + payload.len();
    if total > limits.max_frame_bytes {
        return Err(ProtoError::FrameTooLarge {
            len: total,
            max: limits.max_frame_bytes,
        });
    }
    let mut out = vec![0u8; ENVELOPE_BYTES];
    Envelope::of(M::KIND, payload.len()).write_to(&mut out);
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Decode trọn một frame: đọc envelope, kiểm kind và giới hạn, decode payload rồi kiểm DTO.
pub fn decode_frame<M: Message>(bytes: &[u8], limits: &ProtocolLimits) -> Result<M, ProtoError> {
    decode_frame_inner::<M>(bytes, limits, None)
}

/// Decode trọn một frame **trên một kênh**: kind không hợp lệ trên kênh bị từ chối trước khi
/// decode payload.
///
/// `role` là vai trò của bên đang xử lý kết nối: server nhận message chiều client→server, client
/// nhận message chiều server→client. Kênh game không nhận kind health và admin listener chỉ nhận
/// `HealthRequest` (ADR 0009 Addendum A §A.1). Sai kênh xử lý **như kind lạ**: trả
/// `ProtoError::UnknownKind` để caller đóng kết nối và log, không đoán.
pub fn decode_frame_on<M: Message>(
    bytes: &[u8],
    limits: &ProtocolLimits,
    channel: Channel,
    role: Role,
) -> Result<M, ProtoError> {
    decode_frame_inner::<M>(bytes, limits, Some((channel, role)))
}

fn decode_frame_inner<M: Message>(
    bytes: &[u8],
    limits: &ProtocolLimits,
    channel: Option<(Channel, Role)>,
) -> Result<M, ProtoError> {
    if bytes.len() > limits.max_frame_bytes {
        return Err(ProtoError::FrameTooLarge {
            len: bytes.len(),
            max: limits.max_frame_bytes,
        });
    }
    let (envelope, payload) = read_envelope(bytes)?;
    if let Some((channel, role)) = channel
        && !envelope.kind.is_receivable_on(channel, role)
    {
        return Err(ProtoError::UnknownKind(envelope.kind.code()));
    }
    if envelope.kind != M::KIND {
        return Err(ProtoError::KindMismatch {
            expected: M::KIND,
            got: envelope.kind,
        });
    }
    let max = limits.max_payload_bytes(M::KIND);
    if payload.len() > max {
        return Err(ProtoError::PayloadTooLarge {
            len: payload.len(),
            max,
        });
    }
    let message = decode_payload::<M>(payload)?;
    message.check(limits)?;
    Ok(message)
}
