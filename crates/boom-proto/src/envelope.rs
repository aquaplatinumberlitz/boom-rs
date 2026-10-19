//! Envelope 16 byte viết tay, tách khỏi payload (ADR 0009 §2).
//!
//! ```text
//! offset 0  4  magic        42 4F 4F 4D ("BOOM")
//! offset 4  2  wire_schema  u16 LE = 1
//! offset 6  2  kind         u16 LE
//! offset 8  2  flags        u16 LE = 0 (bit lạ bị từ chối)
//! offset 10 4  payload_len  u32 LE
//! offset 14 2  reserved     u16 LE = 0
//! ```
//!
//! Envelope không mang `round_id`: lệnh phòng và handshake không có round.

use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::{ENVELOPE_BYTES, MAX_FRAME_BYTES, WIRE_SCHEMA};

/// Magic 4 byte đầu frame: `BOOM`.
pub const MAGIC: [u8; 4] = *b"BOOM";

/// Phần đầu envelope đã đọc; payload nằm ngay sau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// `wire_schema` của frame; V1 chỉ nhận `1`.
    pub wire_schema: u16,
    /// Loại message.
    pub kind: MessageKind,
    /// Bit cờ; V1 chỉ nhận `0`.
    pub flags: u16,
    /// Độ dài payload tính bằng byte.
    pub payload_len: u32,
    /// Trường dành riêng; V1 chỉ nhận `0`.
    pub reserved: u16,
}

impl Envelope {
    /// Ghi envelope ra 16 byte đầu của `out`.
    pub fn write_to(&self, out: &mut [u8]) {
        out[0..4].copy_from_slice(&MAGIC);
        out[4..6].copy_from_slice(&self.wire_schema.to_le_bytes());
        out[6..8].copy_from_slice(&self.kind.code().to_le_bytes());
        out[8..10].copy_from_slice(&self.flags.to_le_bytes());
        out[10..14].copy_from_slice(&self.payload_len.to_le_bytes());
        out[14..16].copy_from_slice(&self.reserved.to_le_bytes());
    }

    /// Envelope của một message, `flags`/`reserved` bằng 0.
    pub fn of(kind: MessageKind, payload_len: usize) -> Self {
        Self {
            wire_schema: WIRE_SCHEMA,
            kind,
            flags: 0,
            payload_len: u32::try_from(payload_len).expect("payload len vượt u32"),
            reserved: 0,
        }
    }
}

/// Đọc envelope ở đầu `bytes`; trả về envelope và phần payload còn lại.
///
/// Kiểm magic, schema, kind, flags, reserved và độ dài trước khi trả payload: decoder luôn
/// kiểm phần đầu trước, không bao giờ decode payload của frame hỏng (ADR 0009 §5.2).
pub fn read_envelope(bytes: &[u8]) -> Result<(Envelope, &[u8]), ProtoError> {
    if bytes.len() < ENVELOPE_BYTES {
        return Err(ProtoError::ShortFrame {
            need: ENVELOPE_BYTES,
            got: bytes.len(),
        });
    }
    if bytes[0..4] != MAGIC {
        let mut got = [0u8; 4];
        got.copy_from_slice(&bytes[0..4]);
        return Err(ProtoError::BadMagic { got });
    }
    let wire_schema = u16::from_le_bytes([bytes[4], bytes[5]]);
    if wire_schema != WIRE_SCHEMA {
        return Err(ProtoError::UnsupportedWireSchema(wire_schema));
    }
    let kind_code = u16::from_le_bytes([bytes[6], bytes[7]]);
    let kind = MessageKind::from_u16(kind_code).ok_or(ProtoError::UnknownKind(kind_code))?;
    if !kind.is_defined() {
        return Err(ProtoError::KindNotDefined(kind));
    }
    let flags = u16::from_le_bytes([bytes[8], bytes[9]]);
    if flags != 0 {
        return Err(ProtoError::NonZeroFlags(flags));
    }
    let payload_len = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);
    let reserved = u16::from_le_bytes([bytes[14], bytes[15]]);
    if reserved != 0 {
        return Err(ProtoError::NonZeroReserved(reserved));
    }
    let payload_len = usize::try_from(payload_len).map_err(|_| ProtoError::PayloadTooLarge {
        len: usize::MAX,
        max: MAX_FRAME_BYTES - ENVELOPE_BYTES,
    })?;
    if payload_len > MAX_FRAME_BYTES - ENVELOPE_BYTES {
        return Err(ProtoError::PayloadTooLarge {
            len: payload_len,
            max: MAX_FRAME_BYTES - ENVELOPE_BYTES,
        });
    }
    let total = ENVELOPE_BYTES + payload_len;
    if bytes.len() != total {
        return Err(ProtoError::LengthMismatch {
            declared: u32::try_from(payload_len).expect("đã kiểm trần u32"),
            actual: bytes.len() - ENVELOPE_BYTES,
        });
    }
    Ok((
        Envelope {
            wire_schema,
            kind,
            flags,
            payload_len: u32::try_from(payload_len).expect("đã kiểm trần u32"),
            reserved,
        },
        &bytes[ENVELOPE_BYTES..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let payload = [1u8, 2, 3];
        let mut out = vec![0u8; ENVELOPE_BYTES + payload.len()];
        Envelope::of(MessageKind::Ping, payload.len()).write_to(&mut out[..ENVELOPE_BYTES]);
        out[ENVELOPE_BYTES..].copy_from_slice(&payload);
        out
    }

    #[test]
    fn round_trip() {
        let bytes = sample();
        let (env, payload) = read_envelope(&bytes).unwrap();
        assert_eq!(env.kind, MessageKind::Ping);
        assert_eq!(env.payload_len, 3);
        assert_eq!(payload, &[1, 2, 3]);
    }

    #[test]
    fn magic_is_boom() {
        assert_eq!(&sample()[0..4], b"BOOM");
    }

    #[test]
    fn short_frame_is_rejected() {
        let err = read_envelope(&[0u8; 8]).unwrap_err();
        assert!(matches!(err, ProtoError::ShortFrame { need: 16, got: 8 }));
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut bytes = sample();
        bytes[0] = 0;
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::BadMagic { .. })
        ));
    }

    #[test]
    fn other_wire_schema_is_rejected() {
        let mut bytes = sample();
        bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::UnsupportedWireSchema(2))
        ));
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let mut bytes = sample();
        bytes[6..8].copy_from_slice(&0x00FFu16.to_le_bytes());
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::UnknownKind(0x00FF))
        ));
    }

    #[test]
    fn room_kind_is_accepted_after_addendum_b() {
        let mut bytes = sample();
        bytes[6..8].copy_from_slice(&0x0040u16.to_le_bytes());
        let envelope = read_envelope(&bytes).expect("kind room đã định nghĩa");
        assert_eq!(envelope.0.kind, MessageKind::RoomCommand);
    }

    #[test]
    fn non_zero_flags_and_reserved_are_rejected() {
        let mut bytes = sample();
        bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::NonZeroFlags(1))
        ));

        let mut bytes = sample();
        bytes[14..16].copy_from_slice(&1u16.to_le_bytes());
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::NonZeroReserved(1))
        ));
    }

    #[test]
    fn length_mismatch_is_rejected() {
        let mut bytes = sample();
        bytes[10..14].copy_from_slice(&9u32.to_le_bytes());
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::LengthMismatch {
                declared: 9,
                actual: 3
            })
        ));
    }

    #[test]
    fn payload_over_frame_limit_is_rejected() {
        let mut bytes = sample();
        bytes[10..14].copy_from_slice(&(MAX_FRAME_BYTES as u32).to_le_bytes());
        assert!(matches!(
            read_envelope(&bytes),
            Err(ProtoError::PayloadTooLarge { .. })
        ));
    }
}
