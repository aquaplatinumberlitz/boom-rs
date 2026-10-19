//! Chuỗi hiển thị của UI V1: lấy nguyên văn từ file chuỗi đã Việt hóa.
//!
//! Nguồn chuẩn là `data/l10n/vi-v1.toml` (output của `tools/l10n_export.py`, xem
//! [UI_STRINGS_V1 §6.1](../../../../docs/specification/localization/UI_STRINGS_V1.md)).
//! Client **không tự soạn chữ**: khóa chưa có text hiển thị placeholder rõ ràng
//! `⟨khóa⟩` (hàm [`boom_content::l10n::placeholder`]).
//!
//! Module thuần — không đụng macroquad — nên test được không cần GPU.

use boom_content::l10n::{L10n, L10nError, Text};

/// File chuỗi V1 đã Việt hóa, nhúng vào client.
///
/// Nhúng (không đọc lúc chạy) để client dùng **đúng** bộ chuỗi đã duyệt cùng
/// bundle content: không có đường đọc file chuỗi khác trên máy người chơi.
const VI_V1: &str = include_str!("../../../../data/l10n/vi-v1.toml");

/// Bộ chuỗi hiển thị cho UI.
#[derive(Clone, Debug)]
pub struct Texts {
    l10n: L10n,
}

impl Texts {
    /// Nạp từ nội dung TOML của file chuỗi.
    pub fn from_toml(text: &str) -> Result<Self, L10nError> {
        Ok(Self {
            l10n: L10n::from_toml(text)?,
        })
    }

    /// Bộ chuỗi V1 nhúng trong client (`data/l10n/vi-v1.toml`).
    pub fn embedded() -> Self {
        Self::from_toml(VI_V1).expect("file chuỗi V1 trong repo phải nạp được")
    }

    /// Text hiển thị của một khóa.
    ///
    /// - Có text chốt → trả nguyên văn (đã chuẩn hóa `\r\n` thành `\n`).
    /// - Khóa thiếu hoặc chưa có text → placeholder `⟨khóa⟩`; client **không** tự viết
    ///   chữ thay (UI_STRINGS_V1 §6.1).
    pub fn get(&self, key: &str) -> String {
        match self.l10n.text(key) {
            Text::Some(text) => normalize(text),
            Text::Missing(_) => boom_content::l10n::placeholder(key),
        }
    }
}

/// Chuẩn hóa chuỗi cho hiển thị: `\r\n` và `\r` thành `\n`.
///
/// Chuỗi trong file giữ nguyên `\r\n` của client gốc (UI_STRINGS_V1 §5.2 yêu cầu giữ
/// nguyên placeholder và ký tự gốc); phần **hiển thị** thì một dòng là một dòng, nên
/// chuẩn hóa khi vẽ — không sửa file nguồn.
#[must_use]
pub fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_file_has_the_v1_keys() {
        let texts = Texts::embedded();
        // Khóa có text chốt: lấy nguyên văn.
        assert_eq!(texts.get("login.password"), "Mật khẩu");
    }

    #[test]
    fn missing_key_shows_placeholder_not_invented_text() {
        let texts = Texts::embedded();
        // `login.title` là `unknown` trong file chuỗi → placeholder.
        assert_eq!(texts.get("login.title"), "⟨login.title⟩");
        // Khóa không có trong file cũng ra placeholder, không phải chuỗi rỗng.
        assert_eq!(texts.get("app.khong_ton_tai"), "⟨app.khong_ton_tai⟩");
    }

    #[test]
    fn crlf_is_normalized_for_display_only() {
        let texts = Texts::embedded();
        let text = texts.get("room.leave_ready_confirm");
        assert!(!text.contains('\r'), "chuỗi hiển thị không được giữ CR");
        assert!(text.contains('\n'));
        assert!(text.lines().count() >= 3);
    }

    #[test]
    fn normalize_handles_lone_cr() {
        assert_eq!(normalize("a\r\nb\rc"), "a\nb\nc");
    }
}
