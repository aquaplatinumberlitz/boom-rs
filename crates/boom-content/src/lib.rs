//! Schema runtime dùng chung cho core, assets và công cụ.
//!
//! - [`rules`]: rules profile TOML có nguồn/trạng thái cho từng giá trị (AGENTS §4, ARCHITECTURE §4).
//! - [`map`]: lưới ID theo build, bảng phân loại ô và địa hình đã phân loại.
//! - [`animation`]: schema clip/frame/pivot và thời lượng frame của V1.
//! - [`assets`]: loader bảng mapping content (`data/assets/*.toml`) có nhãn bằng chứng.
//! - [`bundle`]: bundle nội dung (atlas ảnh + metadata ô) cho client, không cần bộ giải mã pack.
//! - [`hash`]: FNV-1a 64 dùng cho định danh profile và state hash (ADR 0004).
//! - [`l10n`]: chuỗi UI V1 đã Việt hóa, nạp từ `data/l10n/vi-v1.toml`.

pub mod animation;
pub mod assets;
pub mod bundle;
pub mod hash;
pub mod l10n;
pub mod map;
pub mod rules;

/// Manifest ảnh UI Việt hóa đã duyệt, không chứa game media.
pub mod ui_localization;
