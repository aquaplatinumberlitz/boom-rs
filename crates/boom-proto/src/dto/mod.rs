//! DTO của giao thức V1.
//!
//! Mỗi DTO là một *bản mô tả* trên dây: thứ tự field là thứ tự encode (postcard struct), đổi
//! thứ tự/kiểu field là tăng `state_schema` (ADR 0009 §4.1). DTO không chứa luật chơi và không
//! phụ thuộc `boom-core`: chuyển đổi với `SimulationState` nằm ở crate adapter `boom-sync`.

pub mod events;
pub mod handshake;
pub mod input;
pub mod ping;
pub mod snapshot;

use crate::limits::WIRE_MAX_CAPABILITIES;

/// Danh sách capability trong `Hello`, có trần định dạng.
pub(crate) fn de_capabilities<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, String, WIRE_MAX_CAPABILITIES>(d)
}

/// Thuật toán hash của `rules_hash` (`fnv1a64-v0`).
///
/// `rules_hash` trong handshake là hash định danh rules profile, dùng cùng thuật toán với state
/// hash của core (ADR 0004) nên hai bên so sánh được bằng cùng một hàm.
pub const RULES_HASH_ALGO: &str = boom_content::hash::HASH_ALGO;

/// Kiểm một định danh schema của DTO handshake khớp V1.
pub(crate) fn check_schema(
    field: &'static str,
    schema: u16,
) -> Result<(), crate::error::ProtoError> {
    if schema == crate::limits::WIRE_SCHEMA {
        Ok(())
    } else {
        Err(crate::error::ProtoError::Invalid {
            field,
            detail: format!("schema {schema} không được hỗ trợ (V1 chỉ nhận 1)"),
        })
    }
}

/// Mã hướng trong byte `bits` của một lệnh (ADR 0009 §3.1).
pub const DIR_NONE: u8 = 0;
/// Hướng lên.
pub const DIR_UP: u8 = 1;
/// Hướng xuống.
pub const DIR_DOWN: u8 = 2;
/// Hướng trái.
pub const DIR_LEFT: u8 = 3;
/// Hướng phải.
pub const DIR_RIGHT: u8 = 4;

/// Mã hướng lớn nhất hợp lệ trong `bits`.
pub const DIR_MAX: u8 = DIR_RIGHT;
