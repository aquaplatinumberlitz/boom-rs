//! DTO của giao thức V1.
//!
//! Mỗi DTO là một *bản mô tả* trên dây: thứ tự field là thứ tự encode (postcard struct), đổi
//! thứ tự/kiểu field là tăng `state_schema` (ADR 0009 §4.1). DTO không chứa luật chơi và không
//! phụ thuộc `boom-core`: chuyển đổi với `SimulationState` nằm ở crate adapter `boom-sync`.

pub mod auth;
pub mod events;
pub mod handshake;
pub mod health;
pub mod input;
pub mod ping;
pub mod room;
pub mod snapshot;

use crate::limits::{
    WIRE_MAX_CAPABILITIES, WIRE_MAX_LISTEN_ADDRS, WIRE_MAX_PASSWORD_BYTES, WIRE_MAX_PROBLEMS,
    WIRE_MAX_USERNAME_BYTES,
};

/// Danh sách capability trong `Hello`, có trần định dạng.
pub(crate) fn de_capabilities<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, String, WIRE_MAX_CAPABILITIES>(d)
}

/// Danh sách `problems` trong `HealthResponse`, có trần định dạng.
pub(crate) fn de_problems<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, String, WIRE_MAX_PROBLEMS>(d)
}

/// Danh sách `listen_addrs` trong `HealthResponse`, có trần định dạng.
pub(crate) fn de_listen_addrs<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, String, WIRE_MAX_LISTEN_ADDRS>(d)
}

/// Decode một chuỗi có trần định dạng: kiểm độ dài **trước khi** cấp phát.
///
/// Postcard đọc length prefix rồi đưa `&str` **mượn** từ buffer cho visitor, nên visitor kiểm
/// độ dài trước rồi mới `to_owned()`: chuỗi quá trần không được cấp phát theo length prefix của
/// frame (ADR 0009 §5.2).
pub(crate) fn bounded_str<'de, D, const MAX: usize>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct BoundedStr<const MAX: usize>;

    impl<'de, const MAX: usize> serde::de::Visitor<'de> for BoundedStr<MAX> {
        type Value = String;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "chuỗi không quá {MAX} byte")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            if v.len() > MAX {
                return Err(E::custom(format!(
                    "chuỗi {} byte vượt trần định dạng {MAX} byte",
                    v.len()
                )));
            }
            Ok(v.to_owned())
        }

        fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
            self.visit_str(v)
        }
    }

    deserializer.deserialize_string(BoundedStr::<MAX>)
}

/// Decode `username` của `Auth` với trần định dạng.
pub(crate) fn de_username<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    bounded_str::<D, WIRE_MAX_USERNAME_BYTES>(d)
}

/// Decode `password` của `Auth` với trần định dạng.
pub(crate) fn de_password<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    bounded_str::<D, WIRE_MAX_PASSWORD_BYTES>(d)
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
