//! Vẽ scene từ bundle: nạp trang atlas và đặt sprite theo quy ước trình diễn.
//!
//! Phần tính toạ độ và z-order nằm ở [`crate::scene`] (module thuần, test được
//! không cần GPU); module này chỉ lo phần texture của macroquad (ADR 0010).
//!
//! Vẽ vào **render target** rồi phóng nguyên khối bằng nearest (đã có ở
//! `main.rs`) nên sprite không bị scale lẻ; texture atlas cũng dùng
//! [`FilterMode::Nearest`].

use std::fmt;
use std::path::{Path, PathBuf};

use macroquad::prelude::*;
use macroquad::texture::{Image, Texture2D};

use crate::scene::{AtlasPageInfo, DrawItem};

/// Các trang atlas của bundle đã nạp thành texture.
pub struct Atlas {
    pages: Vec<Texture2D>,
}

/// Lỗi khi nạp trang atlas.
#[derive(Debug)]
pub enum AtlasError {
    /// Lỗi đọc file atlas.
    Io {
        /// Đường dẫn file.
        path: PathBuf,
        /// Thông điệp lỗi gốc.
        message: String,
    },
    /// File atlas không giải được thành ảnh.
    Decode {
        /// Đường dẫn tương đối trong bundle.
        file: String,
        /// Thông điệp lỗi gốc.
        message: String,
    },
    /// Kích thước ảnh khác kích thước metadata khai.
    Size {
        /// Đường dẫn tương đối trong bundle.
        file: String,
        /// Kích thước metadata khai.
        expected: (u32, u32),
        /// Kích thước ảnh thật.
        found: (u32, u32),
    },
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Decode { file, message } => {
                write!(f, "file atlas {file} không giải được: {message}")
            }
            Self::Size {
                file,
                expected,
                found,
            } => write!(
                f,
                "file atlas {file} có kích thước {}x{}, metadata ghi {}x{}",
                found.0, found.1, expected.0, expected.1
            ),
        }
    }
}

impl std::error::Error for AtlasError {}

impl Atlas {
    /// Nạp các trang atlas của bundle từ thư mục bundle.
    ///
    /// Lỗi đọc file, giải ảnh hoặc lệch kích thước đều trả về rõ ràng — không
    /// panic (bundle thiếu file hoặc sai hash bị chặn ở tầng `scene`).
    pub fn load(dir: &Path, pages: &[AtlasPageInfo]) -> Result<Self, AtlasError> {
        let mut textures = Vec::with_capacity(pages.len());
        for page in pages {
            let path = dir.join(&page.file);
            let bytes = std::fs::read(&path).map_err(|error| AtlasError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
            let image =
                Image::from_file_with_format(&bytes, None).map_err(|error| AtlasError::Decode {
                    file: page.file.clone(),
                    message: format!("{error:?}"),
                })?;
            let found = (u32::from(image.width), u32::from(image.height));
            if found != (page.width, page.height) {
                return Err(AtlasError::Size {
                    file: page.file.clone(),
                    expected: (page.width, page.height),
                    found,
                });
            }
            let texture = Texture2D::from_image(&image);
            texture.set_filter(FilterMode::Nearest);
            textures.push(texture);
        }
        Ok(Self { pages: textures })
    }

    /// Vẽ một lệnh vẽ: cắt vùng nguồn trong trang atlas rồi đặt ở toạ độ đích.
    ///
    /// `origin` là góc trên-trái của vùng map trong không gian logic (HUD chiếm
    /// dải phía trên); toạ độ đích của lệnh vẽ tính trong không gian map.
    pub fn draw(&self, item: &DrawItem, origin: (f32, f32)) {
        self.draw_scaled(item, origin, 1.0);
    }

    /// Vẽ UI vừa khung với cùng tỷ lệ hai chiều; đường vẽ map vẫn dùng scale 1.
    pub fn draw_scaled(&self, item: &DrawItem, origin: (f32, f32), scale: f32) {
        let Some(texture) = self.pages.get(item.page as usize) else {
            return;
        };
        draw_texture_ex(
            texture,
            origin.0 + item.dest.x as f32,
            origin.1 + item.dest.y as f32,
            WHITE,
            DrawTextureParams {
                dest_size: Some(macroquad::prelude::vec2(
                    item.source.width as f32 * scale,
                    item.source.height as f32 * scale,
                )),
                source: Some(Rect::new(
                    item.source.x as f32,
                    item.source.y as f32,
                    item.source.width as f32,
                    item.source.height as f32,
                )),
                ..Default::default()
            },
        );
    }
}
