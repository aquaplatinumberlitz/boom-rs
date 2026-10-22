//! UI luồng V1 (V1-032) và chuỗi Việt hóa (V1-035).
//!
//! Module `ui` chia làm hai phần:
//!
//! - **Phần thuần dữ liệu** ([`model`], [`layout`], [`text`]): không đụng macroquad,
//!   kiểm được **không cần GPU** — đây là phần nghiệm thu chính (luồng màn hình
//!   đủ, không dead end, text lấy đúng khóa l10n, ảnh `needs_review_text` không hiện).
//! - **Phần vẽ** ([`draw`]) và **vòng lặp app** ([`app`]): mỏng, chỉ đưa layout ra
//!   màn hình.
//!
//! ## Nguồn
//!
//! - [UI_STRINGS_V1](../../../../docs/specification/localization/UI_STRINGS_V1.md) —
//!   danh mục khóa chuỗi V1 và trạng thái dùng của ảnh UI.
//! - [BUNDLES §3.3](../../../../docs/specification/assets/BUNDLES.md) — bảng `[[ui]]`
//!   của bundle: màn hình, vai trò, trạng thái dùng (`selected` / `needs_review_text`
//!   / `excluded`).
//! - [SCOPE_V1 §3](../../../../docs/planning/SCOPE_V1.md) — policy phòng và vòng đời
//!   (kết quả do server quyết, rời trận chủ động là bị loại).
//!
//! ## Nguồn dữ liệu
//!
//! Nguồn dữ liệu là trait [`model::UiBackend`]. Bản offline giả lập là
//! [`demo::DemoBackend`] (chế độ `--ui-demo`); bản mạng là
//! [`crate::net::ui_backend::NetBackend`] (server thật) — model và layout không đổi.

pub mod app;
pub mod demo;
pub mod draw;
pub mod layout;
pub mod model;
pub mod text;

#[cfg(test)]
mod tests;

pub use app::{UiOptions, run_demo};
pub use demo::{DemoBackend, ShotScreen};
pub use layout::{HEIGHT, WIDTH};
pub use model::UiModel;
pub use text::Texts;
