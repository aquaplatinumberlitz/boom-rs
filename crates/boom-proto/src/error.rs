//! Lỗi giao thức.
//!
//! Mọi lỗi decode/encode là giá trị trả về, không panic, không áp dụng một phần (ADR 0009 §5.2).

use std::fmt;

use crate::kind::MessageKind;

/// Lỗi envelope, codec hoặc miền giá trị của DTO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtoError {
    /// Frame ngắn hơn envelope.
    ShortFrame {
        /// Số byte tối thiểu cần.
        need: usize,
        /// Số byte thực có.
        got: usize,
    },
    /// Magic sai: frame không thuộc protocol này.
    BadMagic {
        /// 4 byte đầu đọc được.
        got: [u8; 4],
    },
    /// `wire_schema` khác `1`; V1 chỉ nhận khớp chính xác (ADR 0009 §1).
    UnsupportedWireSchema(u16),
    /// Mã kind không có trong bảng kind.
    UnknownKind(u16),
    /// Kind có trong bảng nhưng chưa định nghĩa payload ở V1.
    KindNotDefined(MessageKind),
    /// `flags` khác 0: bit lạ bị từ chối (ADR 0009 §2).
    NonZeroFlags(u16),
    /// `reserved` khác 0.
    NonZeroReserved(u16),
    /// `payload_len` không khớp độ dài payload thật: cắt bớt/đệm đều là lỗi giao thức.
    LengthMismatch {
        /// Độ dài khai trong envelope.
        declared: u32,
        /// Độ dài payload thật.
        actual: usize,
    },
    /// Payload vượt trần của message.
    PayloadTooLarge {
        /// Độ dài payload.
        len: usize,
        /// Trần áp dụng.
        max: usize,
    },
    /// Frame vượt trần 64 KiB.
    FrameTooLarge {
        /// Độ dài frame.
        len: usize,
        /// Trần áp dụng.
        max: usize,
    },
    /// Còn byte thừa sau payload: decoder từ chối trailing byte (ADR 0009 §5.2).
    TrailingBytes(usize),
    /// Envelope khai kind khác với loại message được yêu cầu.
    KindMismatch {
        /// Kind của message được yêu cầu.
        expected: MessageKind,
        /// Kind đọc được từ envelope.
        got: MessageKind,
    },
    /// Số phần tử vượt miền của phiên (ADR 0009 §4.2).
    CountLimit {
        /// Tên trường.
        field: &'static str,
        /// Số phần tử thực.
        len: usize,
        /// Trần áp dụng.
        max: usize,
    },
    /// Giá trị trong DTO nằm ngoài miền.
    Invalid {
        /// Tên trường.
        field: &'static str,
        /// Mô tả cụ thể.
        detail: String,
    },
    /// Postcard không decode được payload.
    Decode(postcard::Error),
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShortFrame { need, got } => {
                write!(f, "frame cần ít nhất {need} byte, có {got}")
            }
            Self::BadMagic { got } => {
                write!(f, "magic sai: {got:02x?}, cần 42 4f 4f 4d")
            }
            Self::UnsupportedWireSchema(s) => {
                write!(f, "wire_schema {s} không được hỗ trợ (V1 chỉ nhận 1)")
            }
            Self::UnknownKind(code) => write!(f, "kind 0x{code:04x} không có trong bảng"),
            Self::KindNotDefined(kind) => {
                write!(f, "kind {:?} chưa định nghĩa payload ở V1", kind)
            }
            Self::NonZeroFlags(flags) => write!(f, "flags {flags:#06x} khác 0"),
            Self::NonZeroReserved(v) => write!(f, "reserved {v} khác 0"),
            Self::LengthMismatch { declared, actual } => write!(
                f,
                "payload_len khai {declared} nhưng frame có {actual} byte payload"
            ),
            Self::PayloadTooLarge { len, max } => {
                write!(f, "payload {len} byte vượt trần {max}")
            }
            Self::FrameTooLarge { len, max } => write!(f, "frame {len} byte vượt trần {max}"),
            Self::TrailingBytes(n) => write!(f, "còn {n} byte thừa sau payload"),
            Self::KindMismatch { expected, got } => {
                write!(f, "cần kind {expected:?} nhưng envelope khai {got:?}")
            }
            Self::CountLimit { field, len, max } => {
                write!(f, "{field} có {len} phần tử, vượt trần {max}")
            }
            Self::Invalid { field, detail } => write!(f, "{field} sai: {detail}"),
            Self::Decode(e) => write!(f, "payload không decode được: {e}"),
        }
    }
}

impl std::error::Error for ProtoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<postcard::Error> for ProtoError {
    fn from(e: postcard::Error) -> Self {
        Self::Decode(e)
    }
}
