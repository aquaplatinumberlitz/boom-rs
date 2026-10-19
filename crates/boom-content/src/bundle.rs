//! Bundle nội dung: ảnh (atlas) + metadata ô của map, đóng gói cho client.
//!
//! Bundle là **output cục bộ** của `boom-assets bundle` (không commit vào repo). Nó gom
//! ảnh đã giải mã từ pack KR thành các trang atlas PNG, kèm metadata ánh xạ ô của map →
//! sprite/frame, kích thước, canvas và pivot. Client đọc bundle bằng kiểu ở đây, **không
//! cần bộ giải mã pack** (ARCHITECTURE §7).
//!
//! ## Phạm vi
//!
//! - Bundle chỉ mang **nội dung**: ảnh và ánh xạ id. Phân loại ô/va chạm vẫn nằm ở bảng
//!   [`crate::map::MapProfile`] (`data/maps/*.toml`) — một thông tin, một nguồn chuẩn
//!   (AGENTS §3). Bundle không nhân bản loại ô.
//! - Định dạng bundle là quyết định **design** của dự án (ARCHITECTURE §8 "Định dạng
//!   bundle content"), không phải định dạng của client gốc. Mô tả ở
//!   `docs/specification/assets/BUNDLES.md`.
//!
//! ## Định danh nội dung
//!
//! Trường `content_hash` là SHA-256 của encoding chuẩn ([`Bundle::canonical_bytes`]):
//! số, chuỗi và danh sách theo thứ tự cố định, **không** phụ thuộc thứ tự key, cách
//! ghi file hay bộ mã PNG. Hash tính trên **pixel thô** (RGBA) nên đổi bộ mã PNG không
//! làm đổi `content_hash`; đổi bố cục atlas hoặc đổi ảnh thì có.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::rules::Status;

/// Schema hiện hành của bundle nội dung.
pub const SCHEMA: u32 = 1;

/// Tên file metadata trong thư mục bundle.
pub const METADATA_FILE: &str = "bundle.toml";

/// Thư mục con chứa các trang atlas PNG, tính từ thư mục bundle.
pub const ATLAS_DIR: &str = "atlas";

/// Vai trò sprite: nền của ô (ID tile trong file map).
pub const ROLE_TILE: &str = "tile";

/// Vai trò sprite: vật cản của ô (ID obj trong file map).
pub const ROLE_OBJECT: &str = "object";

/// Trạng thái bằng chứng của một entry trong bundle (nhãn AGENTS §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Confirmed,
    Candidate,
    Design,
    Unknown,
}

impl From<Status> for EvidenceStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::Confirmed => Self::Confirmed,
            Status::Candidate => Self::Candidate,
            Status::Design => Self::Design,
            Status::Unknown => Self::Unknown,
        }
    }
}

/// Một trang atlas: một file PNG chứa nhiều ảnh đặt cạnh nhau.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlasPage {
    /// Đường dẫn tương đối trong thư mục bundle, ví dụ `atlas/page-000.png`.
    pub file: String,
    /// Chiều rộng trang (px).
    pub width: u32,
    /// Chiều cao trang (px).
    pub height: u32,
    /// SHA-256 của **file** PNG (hex).
    ///
    /// Đây là kiểm tra toàn vẹn file, **không** nằm trong `content_hash`: bộ mã PNG khác
    /// vẫn cho cùng nội dung. `content_hash` băm pixel thô.
    pub sha256: String,
}

/// Một ảnh của sprite, đã đặt trong trang atlas.
///
/// Ảnh **rỗng** (kích thước 0×0) là ảnh giữ chỗ của client gốc: blob sprite có thể
/// khai một ảnh 0×0 cho frame cuối (`src = (0,0,0,0)`). Ảnh rỗng không đặt trong atlas
/// nên `page` là `None`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteImage {
    /// Chỉ số ảnh trong sprite (theo thứ tự blob gốc).
    pub index: u32,
    /// Trang atlas chứa ảnh; `None` nếu ảnh rỗng (không đặt trong atlas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Toạ độ góc trên-trái trong trang (px).
    pub x: u32,
    /// Toạ độ góc trên-trái trong trang (px).
    pub y: u32,
    /// Chiều rộng ảnh (px).
    pub width: u32,
    /// Chiều cao ảnh (px).
    pub height: u32,
    /// SHA-256 của buffer RGBA thô của ảnh (hex), 4 byte/pixel, thứ tự R, G, B, A.
    pub pixel_sha256: String,
}

/// Một frame của sprite, theo layout 48 byte của client gốc (MAPS §3).
///
/// Toạ độ nguồn (`x0..y1`) là tương đối so với ảnh của sprite; toạ độ tuyệt đối trong
/// atlas là `image.x + x0`, `image.y + y0`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    /// Chỉ số ảnh trong [`Sprite::images`].
    pub image: u32,
    /// `f0` — offset đăng ký khung hình theo trục X.
    pub offset_x: i32,
    /// `f1` — offset đăng ký khung hình theo trục Y.
    pub offset_y: i32,
    /// `f2` — flag/lớp; nghĩa chưa chốt (MAPS §3).
    pub flag: i32,
    /// `f4` — cạnh trái của hình chữ nhật nguồn trong ảnh.
    pub x0: i32,
    /// `f5` — cạnh trên của hình chữ nhật nguồn trong ảnh.
    pub y0: i32,
    /// `f6` — cạnh phải của hình chữ nhật nguồn trong ảnh.
    pub x1: i32,
    /// `f7` — cạnh dưới của hình chữ nhật nguồn trong ảnh.
    pub y1: i32,
    /// `f8` — chiều rộng canvas của frame.
    pub canvas_width: i32,
    /// `f9` — chiều cao canvas của frame.
    pub canvas_height: i32,
    /// `f10` — pivot X.
    pub pivot_x: i32,
    /// `f11` — pivot Y.
    pub pivot_y: i32,
}

/// Một sprite: một entry pack đã giải mã thành ảnh + frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sprite {
    /// Định danh trong bundle; bằng đường dẫn entry pack (`<pack>/<path>`).
    pub id: String,
    /// Đường dẫn entry pack nguồn.
    pub entry: String,
    /// Vai trò của entry trong trận V1 (`tile`, `object`, `character_walk`, …).
    pub role: String,
    /// Trạng thái bằng chứng của entry.
    pub status: EvidenceStatus,
    /// SHA-256 của blob sprite gốc trong pack (hex).
    pub source_sha256: String,
    /// ID tile/obj trong file map; chỉ có với vai trò `tile`/`object`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_id: Option<u32>,
    /// Tên entry trong `fx/ObjectTable/ObjectTable`; chỉ có với vai trò `tile`/`object`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_name: Option<String>,
    /// Các ảnh của sprite, đặt trong atlas.
    pub images: Vec<SpriteImage>,
    /// Các frame của sprite.
    pub frames: Vec<Frame>,
}

/// Một ô của map: ID tile/obj theo file `.map` nguồn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    /// Cột của ô (0 = cột đầu).
    pub x: u16,
    /// Hàng của ô (0 = hàng đầu).
    pub y: u16,
    /// ID tile (nền) của ô; ô trống thì không có.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile: Option<u32>,
    /// ID obj (vật cản) của ô; không có thì ô trống.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<u32>,
}

/// Một entry trong closure **không phải ảnh** (file map, BGM, bảng chỉ số).
///
/// Vẫn được ghi vào bundle để không bỏ qua tài nguyên âm thầm: client/server đối chiếu
/// được SHA-256 của file map với bản đang dựng, và biết entry nào chưa có trong atlas.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonImageAsset {
    /// Đường dẫn entry pack.
    pub entry: String,
    /// Vai trò của entry trong trận V1.
    pub role: String,
    /// Trạng thái bằng chứng của entry.
    pub status: EvidenceStatus,
    /// SHA-256 của payload gốc trong pack (hex).
    pub source_sha256: String,
    /// Kích thước payload gốc (byte).
    pub bytes: u32,
    /// Vì sao entry không nằm trong atlas.
    pub reason: String,
}

/// Một phần chưa xác định được, kèm lý do — không bỏ qua âm thầm.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unknown {
    /// Loại phần chưa xác định (`map_cell`, `missing_entry`, `character_state`, …).
    pub kind: String,
    /// Định danh phần chưa xác định, ví dụ `tile:1942`.
    pub id: String,
    /// Vì sao chưa xác định được.
    pub reason: String,
    /// Entry ứng viên (có thể rỗng).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
}

/// Bundle nội dung của một map: ảnh + metadata ô.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// Schema của bundle.
    pub schema: u32,
    /// Build của pack nguồn (`kr`).
    pub build: String,
    /// Tên map.
    pub map: String,
    /// Tile set của map; rỗng nếu map dùng nhiều tile set.
    pub tileset: String,
    /// Số cột của lưới ô.
    pub width: u16,
    /// Số hàng của lưới ô.
    pub height: u16,
    /// Spawn của map, theo thứ tự file nguồn.
    pub spawns: Vec<(u16, u16)>,
    /// Định danh nội dung: SHA-256 của [`Bundle::canonical_bytes`] (hex).
    pub content_hash: String,
    /// Các trang atlas.
    pub atlas: Vec<AtlasPage>,
    /// Các sprite của entry ảnh trong closure.
    pub sprites: Vec<Sprite>,
    /// Các entry trong closure không phải ảnh (map, BGM, bảng chỉ số).
    pub assets: Vec<NonImageAsset>,
    /// Các ô của map.
    pub cells: Vec<Cell>,
    /// Các phần chưa xác định, kèm lý do.
    pub unknown: Vec<Unknown>,
}

impl Bundle {
    /// Đọc bundle từ chuỗi TOML.
    ///
    /// # Lỗi
    ///
    /// Trả lỗi nếu TOML sai, schema lạ, hoặc metadata không nhất quán
    /// (xem [`Bundle::validate`]).
    pub fn from_toml(text: &str) -> Result<Self, BundleError> {
        let bundle: Self = toml::from_str(text).map_err(|e| BundleError::Parse(e.to_string()))?;
        bundle.validate()?;
        Ok(bundle)
    }

    /// Xuất metadata bundle ra TOML (kèm dòng tiêu đề).
    pub fn to_toml(&self) -> Result<String, BundleError> {
        let body = toml::to_string(self).map_err(|e| BundleError::Encode(e.to_string()))?;
        Ok(format!(
            "# Bundle nội dung — map {map} (build {build}), schema {schema}.\n\
             # Sinh bởi `boom-assets bundle`; content_hash băm pixel thô, không phụ thuộc bộ mã PNG.\n\
             # Định dạng: docs/specification/assets/BUNDLES.md\n\n\
             {body}",
            map = self.map,
            build = self.build,
            schema = self.schema,
        ))
    }

    /// Encoding chuẩn của nội dung bundle: SHA-256 của chuỗi này là `content_hash`.
    ///
    /// Thứ tự phần tử theo đúng thứ tự trong struct, chuỗi có tiền tố độ dài, số là
    /// little-endian. `content_hash` và `sha256` của file atlas **không** nằm trong
    /// encoding này (vòng lặp, và phụ thuộc bộ mã PNG).
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Canonical::new();
        out.bytes(b"boom-bundle-v1\0");
        out.u32(self.schema);
        out.str(&self.build);
        out.str(&self.map);
        out.str(&self.tileset);
        out.u32(u32::from(self.width));
        out.u32(u32::from(self.height));
        out.u32(self.spawns.len() as u32);
        for (x, y) in &self.spawns {
            out.u32(u32::from(*x));
            out.u32(u32::from(*y));
        }
        out.u32(self.atlas.len() as u32);
        for page in &self.atlas {
            out.str(&page.file);
            out.u32(page.width);
            out.u32(page.height);
        }
        out.u32(self.sprites.len() as u32);
        for sprite in &self.sprites {
            out.str(&sprite.id);
            out.str(&sprite.entry);
            out.str(&sprite.role);
            out.str(sprite.status.as_str());
            out.str(&sprite.source_sha256);
            out.i32(
                sprite
                    .map_id
                    .map_or(-1, |id| i32::try_from(id).unwrap_or(i32::MAX)),
            );
            out.str(sprite.object_name.as_deref().unwrap_or(""));
            out.u32(sprite.images.len() as u32);
            for image in &sprite.images {
                out.u32(image.index);
                out.u32(image.page.unwrap_or(u32::MAX));
                out.u32(image.x);
                out.u32(image.y);
                out.u32(image.width);
                out.u32(image.height);
                out.str(&image.pixel_sha256);
            }
            out.u32(sprite.frames.len() as u32);
            for frame in &sprite.frames {
                out.u32(frame.image);
                out.i32(frame.offset_x);
                out.i32(frame.offset_y);
                out.i32(frame.flag);
                out.i32(frame.x0);
                out.i32(frame.y0);
                out.i32(frame.x1);
                out.i32(frame.y1);
                out.i32(frame.canvas_width);
                out.i32(frame.canvas_height);
                out.i32(frame.pivot_x);
                out.i32(frame.pivot_y);
            }
        }
        out.u32(self.cells.len() as u32);
        for cell in &self.cells {
            out.u32(u32::from(cell.x));
            out.u32(u32::from(cell.y));
            out.u32(cell.tile.unwrap_or(u32::MAX));
            out.u32(cell.object.unwrap_or(u32::MAX));
        }
        out.u32(self.assets.len() as u32);
        for asset in &self.assets {
            out.str(&asset.entry);
            out.str(&asset.role);
            out.str(asset.status.as_str());
            out.str(&asset.source_sha256);
            out.u32(asset.bytes);
            out.str(&asset.reason);
        }
        out.u32(self.unknown.len() as u32);
        for unknown in &self.unknown {
            out.str(&unknown.kind);
            out.str(&unknown.id);
            out.str(&unknown.reason);
            out.u32(unknown.candidates.len() as u32);
            for candidate in &unknown.candidates {
                out.str(candidate);
            }
        }
        out.finish()
    }

    /// `content_hash` tính từ nội dung hiện tại.
    #[must_use]
    pub fn compute_content_hash(&self) -> String {
        sha256_hex(&self.canonical_bytes())
    }

    /// Kiểm mọi tham chiếu trong metadata.
    ///
    /// Kiểm: lưới ô đủ và không trùng; mỗi ảnh nằm trong trang atlas và nằm trong
    /// trang; mỗi frame trỏ tới ảnh có thật, hình chữ nhật nguồn nằm trong ảnh, canvas
    /// không nhỏ hơn hình nguồn, pivot nằm trong canvas; mỗi ô có sprite nền (hoặc nằm
    /// trong `unknown` kèm lý do) và sprite vật cản nếu có; mọi `unknown` đều có lý do.
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.schema != SCHEMA {
            return Err(BundleError::Schema { found: self.schema });
        }
        if self.build.is_empty() || self.map.is_empty() {
            return Err(BundleError::Reference(
                "bundle thiếu build hoặc tên map".to_owned(),
            ));
        }
        if self.width == 0 || self.height == 0 {
            return Err(BundleError::Reference(format!(
                "lưới ô {}x{} không hợp lệ",
                self.width, self.height
            )));
        }
        let expected_cells = usize::from(self.width) * usize::from(self.height);
        if self.cells.len() != expected_cells {
            return Err(BundleError::Reference(format!(
                "bundle có {} ô, lưới {}x{} cần {expected_cells} ô",
                self.cells.len(),
                self.width,
                self.height
            )));
        }

        let mut seen_cells: Vec<(u16, u16)> = Vec::with_capacity(self.cells.len());
        for cell in &self.cells {
            if cell.x >= self.width || cell.y >= self.height {
                return Err(BundleError::Reference(format!(
                    "ô ({}, {}) nằm ngoài lưới {}x{}",
                    cell.x, cell.y, self.width, self.height
                )));
            }
            if seen_cells.contains(&(cell.x, cell.y)) {
                return Err(BundleError::Reference(format!(
                    "ô ({}, {}) bị lặp",
                    cell.x, cell.y
                )));
            }
            seen_cells.push((cell.x, cell.y));
        }

        let mut seen_sprites: Vec<&str> = Vec::with_capacity(self.sprites.len());
        for sprite in &self.sprites {
            if sprite.id.is_empty() {
                return Err(BundleError::Reference("sprite thiếu id".to_owned()));
            }
            if seen_sprites.contains(&sprite.id.as_str()) {
                return Err(BundleError::Reference(format!(
                    "sprite {} bị lặp",
                    sprite.id
                )));
            }
            seen_sprites.push(sprite.id.as_str());
            if sprite.id != sprite.entry {
                return Err(BundleError::Reference(format!(
                    "sprite {} có id khác entry {}",
                    sprite.id, sprite.entry
                )));
            }
            for (position, image) in sprite.images.iter().enumerate() {
                if image.index as usize != position {
                    return Err(BundleError::Reference(format!(
                        "sprite {}: ảnh thứ {position} có index {}",
                        sprite.id, image.index
                    )));
                }
                match image.page {
                    // Ảnh rỗng: không đặt trong atlas.
                    None => {
                        if image.width != 0 || image.height != 0 {
                            return Err(BundleError::Reference(format!(
                                "sprite {}: ảnh {position} không có trang atlas nhưng kích thước {}x{}",
                                sprite.id, image.width, image.height
                            )));
                        }
                        continue;
                    }
                    Some(page_index) => {
                        if image.width == 0 || image.height == 0 {
                            return Err(BundleError::Reference(format!(
                                "sprite {}: ảnh {position} có kích thước 0",
                                sprite.id
                            )));
                        }
                        let Some(page) = self.atlas.get(page_index as usize) else {
                            return Err(BundleError::Reference(format!(
                                "sprite {}: ảnh {position} trỏ tới trang {} không có",
                                sprite.id, page_index
                            )));
                        };
                        let right = image.x.checked_add(image.width);
                        let bottom = image.y.checked_add(image.height);
                        if right.is_none_or(|right| right > page.width)
                            || bottom.is_none_or(|bottom| bottom > page.height)
                        {
                            return Err(BundleError::Reference(format!(
                                "sprite {}: ảnh {position} ({},{} {}x{}) tràn trang {} ({}x{})",
                                sprite.id,
                                image.x,
                                image.y,
                                image.width,
                                image.height,
                                page_index,
                                page.width,
                                page.height
                            )));
                        }
                    }
                }
            }
            for (position, frame) in sprite.frames.iter().enumerate() {
                let Some(image) = sprite.images.get(frame.image as usize) else {
                    return Err(BundleError::Reference(format!(
                        "sprite {}: frame {position} trỏ tới ảnh {} không có",
                        sprite.id, frame.image
                    )));
                };
                let (Ok(x0), Ok(y0), Ok(x1), Ok(y1)) = (
                    u32::try_from(frame.x0),
                    u32::try_from(frame.y0),
                    u32::try_from(frame.x1),
                    u32::try_from(frame.y1),
                ) else {
                    return Err(BundleError::Reference(format!(
                        "sprite {}: frame {position} có toạ độ nguồn âm",
                        sprite.id
                    )));
                };
                if x1 < x0 || y1 < y0 || x1 > image.width || y1 > image.height {
                    return Err(BundleError::Reference(format!(
                        "sprite {}: frame {position} nguồn ({x0},{y0})-({x1},{y1}) không nằm trong ảnh {}x{}",
                        sprite.id, image.width, image.height
                    )));
                }
                if frame.canvas_width < i32::try_from(x1 - x0).unwrap_or(i32::MAX)
                    || frame.canvas_height < i32::try_from(y1 - y0).unwrap_or(i32::MAX)
                {
                    return Err(BundleError::Reference(format!(
                        "sprite {}: frame {position} có canvas {}x{} nhỏ hơn hình nguồn",
                        sprite.id, frame.canvas_width, frame.canvas_height
                    )));
                }
                if frame.pivot_x < 0
                    || frame.pivot_y < 0
                    || frame.pivot_x > frame.canvas_width
                    || frame.pivot_y > frame.canvas_height
                {
                    return Err(BundleError::Reference(format!(
                        "sprite {}: frame {position} có pivot ({},{}) nằm ngoài canvas {}x{}",
                        sprite.id,
                        frame.pivot_x,
                        frame.pivot_y,
                        frame.canvas_width,
                        frame.canvas_height
                    )));
                }
            }
        }

        for unknown in &self.unknown {
            if unknown.id.is_empty() || unknown.reason.is_empty() {
                return Err(BundleError::Reference(format!(
                    "unknown {} phải có id và lý do",
                    unknown.kind
                )));
            }
        }

        let mut seen_assets: Vec<&str> = Vec::with_capacity(self.assets.len());
        for asset in &self.assets {
            if asset.entry.is_empty() || asset.reason.is_empty() {
                return Err(BundleError::Reference(
                    "entry không phải ảnh phải có đường dẫn và lý do không đưa vào atlas"
                        .to_owned(),
                ));
            }
            if seen_assets.contains(&asset.entry.as_str()) {
                return Err(BundleError::Reference(format!(
                    "entry {} bị lặp",
                    asset.entry
                )));
            }
            seen_assets.push(asset.entry.as_str());
        }

        for cell in &self.cells {
            if let Some(tile) = cell.tile
                && self.sprite_for_map_id(ROLE_TILE, tile).is_none()
                && !self.has_unknown(&format!("tile:{tile}"))
            {
                return Err(BundleError::Reference(format!(
                    "ô ({}, {}) không có sprite nền cho tile {tile} và cũng không nằm trong unknown",
                    cell.x, cell.y
                )));
            }
            if let Some(object) = cell.object
                && self.sprite_for_map_id(ROLE_OBJECT, object).is_none()
                && !self.has_unknown(&format!("obj:{object}"))
            {
                return Err(BundleError::Reference(format!(
                    "ô ({}, {}) không có sprite vật cản cho obj {object} và cũng không nằm trong unknown",
                    cell.x, cell.y
                )));
            }
        }
        Ok(())
    }

    /// Sprite có vai trò `role` và ID map `id`, nếu có (phải là duy nhất).
    fn sprite_for_map_id(&self, role: &str, id: u32) -> Option<&Sprite> {
        let mut found = self
            .sprites
            .iter()
            .filter(|sprite| sprite.role == role && sprite.map_id == Some(id));
        let first = found.next()?;
        found.next().is_none().then_some(first)
    }

    /// `unknown` có đúng ID này không.
    fn has_unknown(&self, id: &str) -> bool {
        self.unknown.iter().any(|unknown| unknown.id == id)
    }
}

impl EvidenceStatus {
    /// Tên trạng thái dùng trong metadata.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Candidate => "candidate",
            Self::Design => "design",
            Self::Unknown => "unknown",
        }
    }
}

/// Bộ ghi encoding chuẩn: số little-endian, chuỗi có tiền tố độ dài.
struct Canonical {
    buf: Vec<u8>,
}

impl Canonical {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn bytes(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    fn str(&mut self, value: &str) {
        self.u32(u32::try_from(value.len()).expect("chuỗi quá dài"));
        self.buf.extend_from_slice(value.as_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.buf
    }
}

/// Nạp bundle từ thư mục: đọc metadata, kiểm tham chiếu, kiểm hash và file atlas.
///
/// # Lỗi
///
/// Trả lỗi nếu thiếu file, metadata sai schema, tham chiếu hỏng, `content_hash` không
/// khớp, hoặc file atlas không khớp SHA-256 đã ghi.
pub fn load(dir: &Path) -> Result<Bundle, BundleError> {
    let meta_path = dir.join(METADATA_FILE);
    let text = std::fs::read_to_string(&meta_path).map_err(|e| BundleError::Io {
        path: meta_path.clone(),
        message: e.to_string(),
    })?;
    let bundle = Bundle::from_toml(&text)?;

    let computed = bundle.compute_content_hash();
    if computed != bundle.content_hash {
        return Err(BundleError::ContentHash {
            recorded: bundle.content_hash.clone(),
            computed,
        });
    }

    for page in &bundle.atlas {
        let path = dir.join(&page.file);
        let data = std::fs::read(&path).map_err(|e| BundleError::Io {
            path: path.clone(),
            message: e.to_string(),
        })?;
        let actual = sha256_hex(&data);
        if actual != page.sha256 {
            return Err(BundleError::FileHash {
                file: page.file.clone(),
                recorded: page.sha256.clone(),
                computed: actual,
            });
        }
        // Kích thước trang trong metadata phải khớp header PNG thật.
        let (width, height) = png_size(&data)?;
        if width != page.width || height != page.height {
            return Err(BundleError::Png(format!(
                "trang {} có kích thước PNG {width}x{height}, metadata ghi {}x{}",
                page.file, page.width, page.height
            )));
        }
    }
    Ok(bundle)
}

/// Kích thước ảnh PNG: đọc chunk IHDR đầu tiên (8 byte signature + length + type +
/// 13 byte dữ liệu + 4 byte CRC).
///
/// Chỉ đọc header: reader không giải ảnh, client giải khi nạp texture. CRC của chunk
/// IHDR được kiểm để header hỏng không lọt qua.
fn png_size(data: &[u8]) -> Result<(u32, u32), BundleError> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if data.len() < 33 {
        return Err(BundleError::Png("file atlas quá ngắn để là PNG".to_owned()));
    }
    if data[..8] != SIGNATURE {
        return Err(BundleError::Png(
            "file atlas không có signature PNG".to_owned(),
        ));
    }
    let length = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    if &data[12..16] != b"IHDR" || length != 13 {
        return Err(BundleError::Png(
            "chunk đầu tiên của file atlas không phải IHDR".to_owned(),
        ));
    }
    let expected = u32::from_be_bytes([data[29], data[30], data[31], data[32]]);
    let computed = crc32_ieee(&data[12..29]);
    if expected != computed {
        return Err(BundleError::Png(format!(
            "CRC chunk IHDR sai: file ghi {expected:#010x}, tính {computed:#010x}"
        )));
    }
    Ok((
        u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
    ))
}

/// CRC-32 chuẩn của PNG (đa thức `0xEDB88320`, đảo kết quả).
fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Lỗi khi đọc hoặc kiểm bundle.
#[derive(Debug, PartialEq, Eq)]
pub enum BundleError {
    /// Lỗi đọc file.
    Io {
        /// Đường dẫn file.
        path: PathBuf,
        /// Thông điệp lỗi gốc.
        message: String,
    },
    /// Metadata TOML không parse được.
    Parse(String),
    /// Metadata không xuất ra TOML được.
    Encode(String),
    /// Schema không hỗ trợ.
    Schema {
        /// Schema đọc được.
        found: u32,
    },
    /// `content_hash` ghi trong metadata không khớp nội dung.
    ContentHash {
        /// Hash ghi trong metadata.
        recorded: String,
        /// Hash tính lại từ nội dung.
        computed: String,
    },
    /// File atlas không khớp SHA-256 đã ghi.
    FileHash {
        /// Đường dẫn tương đối của file.
        file: String,
        /// Hash ghi trong metadata.
        recorded: String,
        /// Hash tính lại từ file.
        computed: String,
    },
    /// File atlas không phải PNG hợp lệ, hoặc kích thước không khớp metadata.
    Png(String),
    /// Tham chiếu trong metadata không nhất quán.
    Reference(String),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Parse(e) => write!(f, "TOML bundle sai: {e}"),
            Self::Encode(e) => write!(f, "không xuất được TOML bundle: {e}"),
            Self::Schema { found } => {
                write!(f, "schema bundle {found} không hỗ trợ (cần {SCHEMA})")
            }
            Self::ContentHash { recorded, computed } => write!(
                f,
                "content_hash không khớp: metadata ghi {recorded}, nội dung tính {computed}"
            ),
            Self::FileHash {
                file,
                recorded,
                computed,
            } => write!(
                f,
                "file {file} không khớp SHA-256: metadata ghi {recorded}, file tính {computed}"
            ),
            Self::Png(e) => write!(f, "file atlas không hợp lệ: {e}"),
            Self::Reference(e) => write!(f, "bundle không nhất quán: {e}"),
        }
    }
}

impl std::error::Error for BundleError {}

/// SHA-256 (FIPS 180-4) của dữ liệu, hex lowercase.
///
/// Tự triển khai để `boom-content` không phải thêm dependency chỉ để băm một chuỗi.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    let mut out = String::with_capacity(64);
    for byte in sha256(data) {
        let _ = fmt::Write::write_fmt(&mut out, format_args!("{byte:02x}"));
    }
    out
}

/// SHA-256 (FIPS 180-4) của dữ liệu.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut schedule = [0u32; 64];
        for (index, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for (&k, &w) in K.iter().zip(schedule.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k)
                .wrapping_add(w);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bundle tối thiểu: một trang atlas, một sprite tile, một ô.
    fn sample_bundle() -> Bundle {
        Bundle {
            schema: SCHEMA,
            build: "kr".to_owned(),
            map: "Village00".to_owned(),
            tileset: "Tile00".to_owned(),
            width: 1,
            height: 1,
            spawns: vec![(0, 0)],
            content_hash: String::new(),
            atlas: vec![AtlasPage {
                file: "atlas/page-000.png".to_owned(),
                width: 64,
                height: 64,
                sha256: "aa".repeat(32),
            }],
            sprites: vec![Sprite {
                id: "FxTile/Tile00/Grass00".to_owned(),
                entry: "FxTile/Tile00/Grass00".to_owned(),
                role: ROLE_TILE.to_owned(),
                status: EvidenceStatus::Confirmed,
                source_sha256: "bb".repeat(32),
                map_id: Some(1942),
                object_name: Some("Tile00_Ground/Grass00".to_owned()),
                images: vec![SpriteImage {
                    index: 0,
                    page: Some(0),
                    x: 0,
                    y: 0,
                    width: 40,
                    height: 40,
                    pixel_sha256: "cc".repeat(32),
                }],
                frames: vec![Frame {
                    image: 0,
                    offset_x: 0,
                    offset_y: 39,
                    flag: -1,
                    x0: 0,
                    y0: 0,
                    x1: 40,
                    y1: 40,
                    canvas_width: 40,
                    canvas_height: 40,
                    pivot_x: 0,
                    pivot_y: 39,
                }],
            }],
            cells: vec![Cell {
                x: 0,
                y: 0,
                tile: Some(1942),
                object: None,
            }],
            assets: vec![NonImageAsset {
                entry: "FxMap/Map/Village00".to_owned(),
                role: "map".to_owned(),
                status: EvidenceStatus::Confirmed,
                source_sha256: "dd".repeat(32),
                bytes: 1834,
                reason: "file map không phải ảnh; bản đọc được ở bảng phân loại ô".to_owned(),
            }],
            unknown: Vec::new(),
        }
    }

    #[test]
    fn sha256_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(&b"a".repeat(1_000_000)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn content_hash_is_stable_and_ignores_the_recorded_hash() {
        let mut bundle = sample_bundle();
        let first = bundle.compute_content_hash();
        assert_eq!(first.len(), 64);
        // Hash không phụ thuộc giá trị `content_hash` đang ghi trong metadata.
        bundle.content_hash = "00".repeat(32);
        assert_eq!(bundle.compute_content_hash(), first);
        // Hash đổi khi nội dung đổi.
        bundle.cells[0].tile = Some(1943);
        assert_ne!(bundle.compute_content_hash(), first);
    }

    #[test]
    fn round_trips_through_toml() {
        let mut bundle = sample_bundle();
        bundle.content_hash = bundle.compute_content_hash();
        let text = bundle.to_toml().expect("xuất TOML");
        assert!(text.contains("content_hash = "));
        let parsed = Bundle::from_toml(&text).expect("đọc lại TOML");
        assert_eq!(parsed, bundle);
        assert_eq!(parsed.compute_content_hash(), bundle.content_hash);
    }

    #[test]
    fn rejects_broken_references() {
        let mut bundle = sample_bundle();
        // Frame trỏ tới ảnh không có.
        bundle.sprites[0].frames[0].image = 7;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));

        // Ảnh tràn ra ngoài trang atlas.
        let mut bundle = sample_bundle();
        bundle.sprites[0].images[0].x = 40;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));

        // Frame nguồn nằm ngoài ảnh.
        let mut bundle = sample_bundle();
        bundle.sprites[0].frames[0].x1 = 41;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));

        // Canvas nhỏ hơn hình nguồn.
        let mut bundle = sample_bundle();
        bundle.sprites[0].frames[0].canvas_width = 10;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));

        // Pivot nằm ngoài canvas.
        let mut bundle = sample_bundle();
        bundle.sprites[0].frames[0].pivot_y = 41;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));

        // Ô không có sprite nền và cũng không nằm trong unknown.
        let mut bundle = sample_bundle();
        bundle.sprites[0].map_id = Some(9999);
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    #[test]
    fn unknown_entry_covers_a_cell_without_sprite() {
        let mut bundle = sample_bundle();
        bundle.sprites[0].map_id = Some(9999);
        bundle.unknown.push(Unknown {
            kind: "map_cell".to_owned(),
            id: "tile:1942".to_owned(),
            reason: "pack FxTile không có entry cho ô này".to_owned(),
            candidates: Vec::new(),
        });
        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn unknown_without_reason_is_rejected() {
        let mut bundle = sample_bundle();
        bundle.unknown.push(Unknown {
            kind: "map_cell".to_owned(),
            id: "tile:1942".to_owned(),
            reason: String::new(),
            candidates: Vec::new(),
        });
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    #[test]
    fn rejects_wrong_schema_and_grid_size() {
        let mut bundle = sample_bundle();
        bundle.schema = SCHEMA + 1;
        assert!(matches!(bundle.validate(), Err(BundleError::Schema { .. })));

        let mut bundle = sample_bundle();
        bundle.cells.clear();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// File PNG tối thiểu: signature + chunk IHDR (13 byte dữ liệu + CRC).
    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut data: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&13u32.to_be_bytes());
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&[8, 6, 0, 0, 0]); // bit depth, RGBA, nén, lọc, xen kẽ
        let crc = crc32_ieee(&data[12..29]);
        data.extend_from_slice(&crc.to_be_bytes());
        data
    }

    /// Thư mục tạm cho test; tự xoá khi test xong.
    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("boom-content-bundle-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tạo thư mục tạm");
        dir
    }

    /// Reader nạp lại bundle từ đĩa: file atlas phải tồn tại, khớp hash và khớp kích thước.
    #[test]
    fn load_checks_files_hashes_and_png_size() {
        let dir = temp_dir("load");
        let atlas_dir = dir.join(ATLAS_DIR);
        std::fs::create_dir_all(&atlas_dir).expect("tạo thư mục atlas");
        let png = minimal_png(64, 64);
        std::fs::write(atlas_dir.join("page-000.png"), &png).expect("ghi PNG");

        let mut bundle = sample_bundle();
        bundle.atlas[0].sha256 = sha256_hex(&png);
        bundle.content_hash = bundle.compute_content_hash();
        std::fs::write(
            dir.join(METADATA_FILE),
            bundle.to_toml().expect("xuất TOML"),
        )
        .expect("ghi metadata");

        let loaded = load(&dir).expect("nạp lại bundle");
        assert_eq!(loaded.content_hash, bundle.content_hash);

        // PNG hỏng → báo lỗi, không âm thầm bỏ qua.
        let mut broken = png.clone();
        broken[20] ^= 0xFF; // đổi byte trong dữ liệu IHDR → CRC sai
        std::fs::write(atlas_dir.join("page-000.png"), &broken).expect("ghi PNG hỏng");
        assert!(matches!(load(&dir), Err(BundleError::FileHash { .. })));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Metadata khai kích thước trang khác kích thước PNG thật thì bị từ chối.
    #[test]
    fn load_rejects_page_size_that_does_not_match_the_png() {
        let dir = temp_dir("size");
        let atlas_dir = dir.join(ATLAS_DIR);
        std::fs::create_dir_all(&atlas_dir).expect("tạo thư mục atlas");
        let png = minimal_png(64, 64);
        std::fs::write(atlas_dir.join("page-000.png"), &png).expect("ghi PNG");

        let mut bundle = sample_bundle();
        bundle.atlas[0].sha256 = sha256_hex(&png);
        bundle.content_hash = bundle.compute_content_hash();
        std::fs::write(
            dir.join(METADATA_FILE),
            bundle.to_toml().expect("xuất TOML"),
        )
        .expect("ghi metadata");
        assert!(load(&dir).is_ok());

        // File PNG nhỏ hơn kích thước khai trong metadata → bị từ chối.
        let smaller = minimal_png(64, 32);
        std::fs::write(atlas_dir.join("page-000.png"), &smaller).expect("ghi PNG");
        bundle.atlas[0].sha256 = sha256_hex(&smaller);
        bundle.content_hash = bundle.compute_content_hash();
        std::fs::write(
            dir.join(METADATA_FILE),
            bundle.to_toml().expect("xuất TOML"),
        )
        .expect("ghi metadata");
        assert!(matches!(load(&dir), Err(BundleError::Png(_))));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
