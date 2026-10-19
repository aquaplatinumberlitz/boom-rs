//! Schema runtime dùng chung cho core, assets và công cụ.
//!
//! - [`rules`]: rules profile TOML có nguồn/trạng thái cho từng giá trị (AGENTS §4, ARCHITECTURE §4).
//! - [`map`]: lưới ID theo build, bảng phân loại ô và địa hình đã phân loại.
//! - [`hash`]: FNV-1a 64 dùng cho định danh profile và state hash (ADR 0004).

pub mod hash;
pub mod map;
pub mod rules;
