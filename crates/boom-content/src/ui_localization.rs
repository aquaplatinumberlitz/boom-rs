//! Quyết định ảnh Việt hóa V1; chỉ metadata được commit, media nằm ngoài repo.
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Component, Path};

/// Nguồn pixel đã được duyệt, không phải asset KR mặc định.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    /// Sprite I16 từ pack VN, giữ toàn bộ frame của chính sprite VN.
    VnPack,
    /// PNG đã biên tập được duyệt, dùng một frame phủ toàn ảnh.
    Png,
}

/// Một ảnh đã duyệt theo ID phần tử UI.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalizedImage {
    pub id: String,
    pub kind: ImageKind,
    pub sha256: String,
    pub pack: Option<String>,
    pub entry: Option<String>,
    pub file: Option<String>,
}

/// Manifest approval có nguồn và hash, dùng khi dựng bundle Việt hóa.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiLocalization {
    pub schema: u32,
    pub locale: String,
    pub reviewed_by: String,
    pub reviewed_at: String,
    pub images: Vec<LocalizedImage>,
}

fn relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

impl UiLocalization {
    /// Nạp và kiểm quyết định. File local phải khớp hash trước khi được hiển thị.
    ///
    /// # Errors
    /// Schema/locale sai, ID trùng, đường dẫn không tương đối hoặc nguồn không đầy đủ.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let result: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        result.validate()?;
        Ok(result)
    }

    /// Kiểm cả manifest tạo trực tiếp từ Rust trước khi nạp file.
    ///
    /// # Errors
    /// Trả lỗi khi schema, hash, ID hoặc đường dẫn nguồn không hợp lệ.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1
            || self.locale != "vi-VN"
            || self.reviewed_by.trim().is_empty()
            || self.reviewed_at.trim().is_empty()
        {
            return Err(
                "manifest Việt hóa: schema/locale hoặc quyết định duyệt không hợp lệ".into(),
            );
        }
        let mut ids = BTreeSet::new();
        for image in &self.images {
            if image.id.trim().is_empty()
                || !ids.insert(&image.id)
                || image.sha256.len() != 64
                || !image
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(format!(
                    "ảnh Việt hóa: ID/hash không hợp lệ hoặc trùng: {}",
                    image.id
                ));
            }
            let valid = match image.kind {
                ImageKind::VnPack => {
                    image.file.is_none()
                        && image.pack.as_deref().is_some_and(|s| {
                            s.bytes().all(|b| b.is_ascii_alphanumeric()) && !s.is_empty()
                        })
                        && image.entry.as_deref().is_some_and(relative)
                }
                ImageKind::Png => {
                    image.pack.is_none()
                        && image.entry.is_none()
                        && image.file.as_deref().is_some_and(relative)
                }
            };
            if !valid {
                return Err(format!("ảnh Việt hóa: nguồn không hợp lệ: {}", image.id));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approvals_reject_duplicate_or_traversal_paths() {
        let source = format!(
            "schema=1\nlocale='vi-VN'\nreviewed_by='owner'\nreviewed_at='2026-10-24'\n\
             [[images]]\nid='result.draw'\nkind='png'\nfile='draw.png'\nsha256='{}'\n",
            "a".repeat(64)
        );
        assert!(UiLocalization::from_toml(&source).is_ok());
        assert!(UiLocalization::from_toml(&source.replace("draw.png", "../draw.png")).is_err());
        let second = source.split("[[images]]").nth(1).unwrap();
        assert!(UiLocalization::from_toml(&format!("{source}[[images]]{second}")).is_err());
    }
}
