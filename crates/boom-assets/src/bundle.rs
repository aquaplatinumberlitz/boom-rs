//! Dựng bundle nội dung cho client: atlas ảnh + metadata ô của map.
//!
//! Bundle là **output cục bộ** (không commit vào repo). Nó được dựng từ hai nguồn đã có
//! trong crate này:
//!
//! - [`crate::closure`] — manifest closure của một trận V1: entry pack nào cần có, vai trò,
//!   SHA-256, và danh sách phần chưa xác định kèm lý do (task #34).
//! - [`crate::sprite`] — bộ giải mã `CDataI16Alpha` v2: ảnh RGBA + frame 48 byte (task #33).
//!
//! Kiểu đọc bundle nằm ở [`boom_content::bundle`] để client dùng được mà **không cần bộ giải
//! mã pack** (ARCHITECTURE §7). Định dạng bundle là quyết định `design` của dự án, mô tả ở
//! `docs/specification/assets/BUNDLES.md`.
//!
//! ## Tái lập
//!
//! Cùng pack ⇒ cùng `content_hash`:
//!
//! - Thứ tự sprite theo thứ tự manifest closure; thứ tự ảnh theo thứ tự blob gốc.
//! - Đóng gói atlas dùng thuật toán kệ (shelf) với thứ tự ảnh cố định
//!   (chiều cao giảm dần, rồi chiều rộng, rồi thứ tự ban đầu).
//! - `content_hash` băm **pixel thô** (RGBA), không phụ thuộc bộ mã PNG.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use boom_content::bundle::{
    AtlasPage, Bundle, BundleError, Cell, EvidenceStatus, Frame, NonImageAsset,
    Sprite as BundleSprite, SpriteImage, Unknown, sha256_hex,
};
use boom_content::rules::Status;

use crate::closure::{self, ClosureEntry, ClosureRole, UnknownKind, V1_MAP};
use crate::legacy_map::{Build, MapError, parse as parse_map};
use crate::pack::{Pack, PackError};
use crate::sprite::{self, SpriteError};

/// Chiều rộng tối đa của một trang atlas (px).
///
/// **Giới hạn kỹ thuật tự đặt** (`design` / `design`), không phải giới hạn của client gốc.
/// Số liệu KR đi kèm (`kr_static` / `confirmed`): cạnh ảnh lớn nhất đã quét là rộng 1198 px
/// và cao 1024 px (PACKS §Bộ giải mã Rust). Chọn 2048 để một trang chứa được mọi ảnh
/// Village00 mà vẫn vừa giới hạn texture của client.
pub const ATLAS_WIDTH: u32 = 2048;

/// Chiều cao tối đa của một trang atlas (px) — xem [`ATLAS_WIDTH`].
pub const ATLAS_HEIGHT: u32 = 2048;

/// Tên file metadata trong thư mục bundle.
pub const METADATA_FILE: &str = boom_content::bundle::METADATA_FILE;

/// Thư mục con chứa các trang atlas.
pub const ATLAS_DIR: &str = boom_content::bundle::ATLAS_DIR;

/// Vai trò closure nào là ảnh (đưa vào atlas); vai trò còn lại là dữ liệu/âm thanh.
fn is_image_role(role: ClosureRole) -> bool {
    matches!(
        role,
        ClosureRole::Tile
            | ClosureRole::Object
            | ClosureRole::Theme
            | ClosureRole::Preview
            | ClosureRole::CharacterWalk
            | ClosureRole::CharacterIdle
            | ClosureRole::CharacterTrapped
            | ClosureRole::CharacterDead
            | ClosureRole::BombArt
            | ClosureRole::FlameArt
            | ClosureRole::ExplosionArt
            | ClosureRole::TrappedArt
            | ClosureRole::ItemIcon
    )
}

/// Lý do ghi cho entry không phải ảnh.
fn non_image_reason(role: ClosureRole) -> &'static str {
    match role {
        ClosureRole::Map => "file map: lưới ô đã nằm trong bundle, byte map giữ để đối chiếu",
        ClosureRole::Music => "âm thanh nền: không phải ảnh, chưa đưa vào bundle",
        ClosureRole::CharacterAttr | ClosureRole::ItemAttr => {
            "bảng chỉ số: dữ liệu, không phải ảnh"
        }
        _ => "không phải ảnh",
    }
}

/// Ảnh đã giải mã, chưa đặt vào atlas.
struct RawImage {
    width: u32,
    height: u32,
    /// RGBA thô, 4 byte/pixel, thứ tự R, G, B, A.
    rgba: Vec<u8>,
}

/// Vị trí một ảnh trong atlas.
struct Placement {
    /// Chỉ số ảnh trong danh sách ảnh của bundle.
    image: usize,
    /// Toạ độ góc trên-trái trong trang (px).
    x: u32,
    /// Toạ độ góc trên-trái trong trang (px).
    y: u32,
}

/// Một trang atlas đã đóng gói.
struct PackedPage {
    placements: Vec<Placement>,
    width: u32,
    height: u32,
}

/// Bundle đã dựng kèm byte PNG của các trang atlas.
///
/// `content_hash` đã được chốt trong [`BuiltBundle::bundle`]; dựng lại từ cùng pack là ra
/// cùng hash.
pub struct BuiltBundle {
    /// Metadata bundle.
    pub bundle: Bundle,
    /// Byte PNG của từng trang atlas, theo thứ tình trang.
    pages: Vec<Vec<u8>>,
}

impl BuiltBundle {
    /// Ghi bundle ra thư mục: `bundle.toml` + các trang atlas trong `atlas/`.
    ///
    /// File `atlas/page-*.png` cũ không thuộc bundle này bị xoá để thư mục đầu ra không
    /// mang trang thừa.
    pub fn write(&self, out_dir: &Path) -> Result<(), BundleBuildError> {
        let atlas_dir = out_dir.join(ATLAS_DIR);
        fs::create_dir_all(&atlas_dir).map_err(|e| BundleBuildError::Io {
            path: atlas_dir.clone(),
            message: e.to_string(),
        })?;

        let mut written: Vec<String> = Vec::new();
        for (index, bytes) in self.pages.iter().enumerate() {
            let name = page_file_name(index);
            let path = atlas_dir.join(&name);
            fs::write(&path, bytes).map_err(|e| BundleBuildError::Io {
                path: path.clone(),
                message: e.to_string(),
            })?;
            written.push(name);
        }
        // Dọn trang atlas cũ không còn dùng (chỉ trong thư mục atlas của bundle).
        if let Ok(entries) = fs::read_dir(&atlas_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("page-") && name.ends_with(".png") && !written.contains(&name) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        let meta_path = out_dir.join(METADATA_FILE);
        let text = self.bundle.to_toml().map_err(BundleBuildError::Bundle)?;
        fs::write(&meta_path, text).map_err(|e| BundleBuildError::Io {
            path: meta_path.clone(),
            message: e.to_string(),
        })?;
        Ok(())
    }
}

/// Tên file trang atlas theo chỉ số (trong thư mục `atlas/`).
fn page_file_name(index: usize) -> String {
    format!("page-{index:03}.png")
}

/// Đường dẫn trang atlas tương đối so với thư mục bundle.
fn page_relative_path(index: usize) -> String {
    format!("{ATLAS_DIR}/{}", page_file_name(index))
}

/// Dựng bundle của một map V1 từ thư mục chứa pack KR.
///
/// Thư mục phải có `.idx`/`.idd` của các pack KR (`FxMap`, `FxTile`, `fx`, `FxCharacter`,
/// `FxContents`, `FxDesktop`, `FxSound`).
pub fn build_bundle(pack_dir: &Path, map_name: &str) -> Result<BuiltBundle, BundleBuildError> {
    if map_name != V1_MAP {
        return Err(BundleBuildError::UnsupportedMap(map_name.to_owned()));
    }
    let manifest = closure::build_village00_manifest(pack_dir)?;

    let map_pack = Pack::open(pack_dir, "FxMap", Build::Kr)?;
    let map_raw = map_pack.read_named(&format!("Map/{V1_MAP}"))?;
    drop(map_pack);
    let map = parse_map(&map_raw, Build::Kr)?;

    let mut unknown: Vec<Unknown> = Vec::new();
    for item in &manifest.unknown {
        push_unknown(
            &mut unknown,
            item.kind.as_str(),
            &item.id,
            &item.reason,
            &item.candidates,
        );
    }

    let mut sprites: Vec<BundleSprite> = Vec::new();
    let mut assets: Vec<NonImageAsset> = Vec::new();
    let mut raw_images: Vec<RawImage> = Vec::new();
    // Ảnh nào thuộc sprite nào: (sprite index, chỉ số ảnh trong sprite).
    let mut image_owner: Vec<(usize, u32)> = Vec::new();

    for entry in &manifest.entries {
        if !is_image_role(entry.role) {
            assets.push(NonImageAsset {
                entry: entry.entry.clone(),
                role: entry.role.as_str().to_owned(),
                status: EvidenceStatus::from(entry.status),
                source_sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                reason: non_image_reason(entry.role).to_owned(),
            });
            continue;
        }
        match decode_entry_sprite(pack_dir, entry) {
            Ok((blob, images)) => {
                let sprite_index = sprites.len();
                let mut sprite_images: Vec<SpriteImage> = Vec::with_capacity(images.len());
                for (image_index, image) in images.iter().enumerate() {
                    let empty = image.width == 0 || image.height == 0;
                    if !empty {
                        image_owner.push((sprite_index, image_index as u32));
                        raw_images.push(RawImage {
                            width: image.width,
                            height: image.height,
                            rgba: image.rgba.clone(),
                        });
                    }
                    sprite_images.push(SpriteImage {
                        index: image_index as u32,
                        // Ảnh rỗng không đặt trong atlas; ảnh có pixel sẽ được ghi vị trí
                        // sau khi đóng gói.
                        page: if empty { None } else { Some(0) },
                        x: 0,
                        y: 0,
                        width: image.width,
                        height: image.height,
                        pixel_sha256: sha256_hex(&image.rgba),
                    });
                }
                sprites.push(BundleSprite {
                    id: entry.entry.clone(),
                    entry: entry.entry.clone(),
                    role: entry.role.as_str().to_owned(),
                    status: EvidenceStatus::from(entry.status),
                    source_sha256: entry.sha256.clone(),
                    map_id: entry.map_id,
                    object_name: entry.object_name.clone(),
                    images: sprite_images,
                    frames: blob,
                });
            }
            Err(reason) => {
                // Ảnh không giải được thì ghi rõ lý do, không bỏ qua âm thầm.
                push_unknown(&mut unknown, "sprite", &entry.entry, &reason, &[]);
            }
        }
    }

    // Đóng gói atlas: thứ tự ảnh cố định nên cùng pack cho cùng bố cục.
    let pages = pack_atlas(&raw_images)?;

    // Ghi vị trí ảnh vào metadata và dựng trang atlas.
    let mut atlas: Vec<AtlasPage> = Vec::with_capacity(pages.len());
    let mut page_bytes: Vec<Vec<u8>> = Vec::with_capacity(pages.len());
    for page in &pages {
        let mut buffer = vec![0u8; (page.width as usize) * (page.height as usize) * 4];
        for placement in &page.placements {
            let image = &raw_images[placement.image];
            blit(
                &mut buffer,
                page.width,
                placement.x,
                placement.y,
                image.width,
                image.height,
                &image.rgba,
            );
            let (sprite_index, image_index) = image_owner[placement.image];
            sprites[sprite_index].images[image_index as usize].page =
                Some(u32::try_from(atlas.len()).expect("số trang atlas nhỏ"));
            sprites[sprite_index].images[image_index as usize].x = placement.x;
            sprites[sprite_index].images[image_index as usize].y = placement.y;
        }
        let png = encode_png(&buffer, page.width, page.height)?;
        atlas.push(AtlasPage {
            file: page_relative_path(atlas.len()),
            width: page.width,
            height: page.height,
            sha256: sha256_hex(&png),
        });
        page_bytes.push(png);
    }

    // Ô của map: ID tile/obj theo file nguồn.
    let width = map.grid.width;
    let height = map.grid.height;
    let mut cells: Vec<Cell> = Vec::with_capacity(map.grid.cells.len());
    for (index, cell) in map.grid.cells.iter().enumerate() {
        let x = u16::try_from(index % usize::from(width)).expect("chỉ số ô vừa lưới");
        let y = u16::try_from(index / usize::from(width)).expect("chỉ số ô vừa lưới");
        cells.push(Cell {
            x,
            y,
            tile: cell.tile,
            object: cell.obj,
        });
    }

    // Mọi ô phải có sprite, hoặc nằm trong unknown kèm lý do.
    for cell in &cells {
        if let Some(tile) = cell.tile
            && !sprites
                .iter()
                .any(|s| s.role == boom_content::bundle::ROLE_TILE && s.map_id == Some(tile))
        {
            push_unknown(
                &mut unknown,
                UnknownKind::MapCell.as_str(),
                &format!("tile:{tile}"),
                &format!(
                    "ô ({}, {}) dùng tile {tile} nhưng không có sprite nền trong closure",
                    cell.x, cell.y
                ),
                &[],
            );
        }
        if let Some(object) = cell.object
            && !sprites
                .iter()
                .any(|s| s.role == boom_content::bundle::ROLE_OBJECT && s.map_id == Some(object))
        {
            push_unknown(
                &mut unknown,
                UnknownKind::MapCell.as_str(),
                &format!("obj:{object}"),
                &format!(
                    "ô ({}, {}) dùng obj {object} nhưng không có sprite vật cản trong closure",
                    cell.x, cell.y
                ),
                &[],
            );
        }
    }

    let mut bundle = Bundle {
        schema: boom_content::bundle::SCHEMA,
        build: manifest.build.clone(),
        map: manifest.map.clone(),
        tileset: manifest.tileset.clone(),
        width,
        height,
        spawns: map.grid.spawns.clone(),
        content_hash: String::new(),
        atlas,
        sprites,
        assets,
        cells,
        unknown,
    };
    bundle.content_hash = bundle.compute_content_hash();
    bundle.validate().map_err(BundleBuildError::Bundle)?;

    Ok(BuiltBundle {
        bundle,
        pages: page_bytes,
    })
}

/// Đọc entry pack và giải thành sprite + ảnh RGBA.
///
/// Trả lỗi (dạng lý do) nếu blob không phải sprite, có ảnh rỗng, hoặc frame trỏ tới ảnh
/// không có.
fn decode_entry_sprite(
    pack_dir: &Path,
    entry: &ClosureEntry,
) -> Result<(Vec<Frame>, Vec<RawImage>), String> {
    let (stem, path) = entry
        .entry
        .split_once('/')
        .ok_or_else(|| format!("entry {} thiếu phần pack", entry.entry))?;
    let pack = Pack::open(pack_dir, stem, Build::Kr).map_err(|e| e.to_string())?;
    let blob = pack
        .read_named(path)
        .map_err(|e| format!("đọc entry thất bại: {e}"))?;
    drop(pack);

    let parsed = sprite::parse(&blob).map_err(|e| format!("không parse được sprite: {e}"))?;
    let mut images: Vec<RawImage> = Vec::with_capacity(parsed.images.len());
    for (index, image) in parsed.images.iter().enumerate() {
        if image.width == 0 || image.height == 0 {
            // Ảnh giữ chỗ 0×0 của client gốc: không có pixel, không đặt trong atlas.
            images.push(RawImage {
                width: image.width,
                height: image.height,
                rgba: Vec::new(),
            });
            continue;
        }
        let rgba = parsed
            .decode_image(index)
            .map_err(|e| format!("giải ảnh {index} thất bại: {e}"))?;
        images.push(RawImage {
            width: image.width,
            height: image.height,
            rgba,
        });
    }

    let mut frames: Vec<Frame> = Vec::with_capacity(parsed.frames.len());
    for frame in &parsed.frames {
        let image_index = u32::try_from(frame.image_index).ok();
        let Some(image_index) = image_index.filter(|index| (*index as usize) < images.len()) else {
            return Err(format!(
                "frame trỏ tới ảnh {} không có (blob có {} ảnh)",
                frame.image_index,
                images.len()
            ));
        };
        frames.push(Frame {
            image: image_index,
            offset_x: frame.offset_x,
            offset_y: frame.offset_y,
            flag: frame.flag,
            x0: frame.source.x0,
            y0: frame.source.y0,
            x1: frame.source.x1,
            y1: frame.source.y1,
            canvas_width: frame.canvas.width,
            canvas_height: frame.canvas.height,
            pivot_x: frame.pivot.x,
            pivot_y: frame.pivot.y,
        });
    }
    Ok((frames, images))
}

/// Đóng gói ảnh vào các trang atlas (thuật toán kệ).
///
/// Thứ tự ảnh: chiều cao giảm dần, rồi chiều rộng giảm dần, rồi thứ tự ban đầu — cố định
/// nên cùng đầu vào cho cùng bố cục.
fn pack_atlas(images: &[RawImage]) -> Result<Vec<PackedPage>, BundleBuildError> {
    let mut order: Vec<usize> = (0..images.len()).collect();
    order.sort_by(|&a, &b| {
        let left = &images[a];
        let right = &images[b];
        right
            .height
            .cmp(&left.height)
            .then(right.width.cmp(&left.width))
            .then(a.cmp(&b))
    });

    let mut pages: Vec<PackedPage> = Vec::new();
    let mut current = PackedPage {
        placements: Vec::new(),
        width: ATLAS_WIDTH,
        height: 0,
    };
    let mut x = 0u32;
    let mut y = 0u32;
    let mut shelf = 0u32;
    for index in order {
        let image = &images[index];
        if image.width > ATLAS_WIDTH || image.height > ATLAS_HEIGHT {
            return Err(BundleBuildError::ImageTooLarge {
                width: image.width,
                height: image.height,
            });
        }
        if x + image.width > ATLAS_WIDTH {
            y += shelf;
            x = 0;
            shelf = 0;
        }
        if y + image.height > ATLAS_HEIGHT {
            current.height = y + shelf;
            pages.push(current);
            current = PackedPage {
                placements: Vec::new(),
                width: ATLAS_WIDTH,
                height: 0,
            };
            x = 0;
            y = 0;
            shelf = 0;
        }
        current.placements.push(Placement { image: index, x, y });
        x += image.width;
        shelf = shelf.max(image.height);
    }
    current.height = y + shelf;
    if !current.placements.is_empty() {
        pages.push(current);
    }
    Ok(pages)
}

/// Chép ảnh RGBA vào buffer trang atlas tại `(x, y)`.
fn blit(buffer: &mut [u8], page_width: u32, x: u32, y: u32, width: u32, height: u32, rgba: &[u8]) {
    for row in 0..height {
        let target = ((y + row) as usize) * (page_width as usize) * 4 + (x as usize) * 4;
        let source = (row as usize) * (width as usize) * 4;
        let len = (width as usize) * 4;
        buffer[target..target + len].copy_from_slice(&rgba[source..source + len]);
    }
}

/// Mã hoá buffer RGBA thành PNG (8 bit, kênh RGBA).
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, BundleBuildError> {
    let mut out: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| BundleBuildError::Png(e.to_string()))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| BundleBuildError::Png(e.to_string()))?;
    }
    Ok(out)
}

/// Thêm một mục unknown; không lặp lại cùng một ID.
fn push_unknown(
    unknown: &mut Vec<Unknown>,
    kind: &str,
    id: &str,
    reason: &str,
    candidates: &[String],
) {
    if unknown.iter().any(|item| item.id == id) {
        return;
    }
    unknown.push(Unknown {
        kind: kind.to_owned(),
        id: id.to_owned(),
        reason: reason.to_owned(),
        candidates: candidates.to_vec(),
    });
}

/// Lỗi khi dựng hoặc ghi bundle.
#[derive(Debug)]
pub enum BundleBuildError {
    /// Lỗi dựng manifest closure.
    Closure(crate::closure::ClosureError),
    /// Lỗi đọc pack.
    Pack(PackError),
    /// Lỗi đọc file map.
    Map(MapError),
    /// Lỗi giải sprite.
    Sprite(SpriteError),
    /// Bundle không nhất quán (tham chiếu hỏng, hash lệch).
    Bundle(BundleError),
    /// Map chưa hỗ trợ.
    UnsupportedMap(String),
    /// Ảnh vượt trần của trang atlas.
    ImageTooLarge {
        /// Chiều rộng ảnh (px).
        width: u32,
        /// Chiều cao ảnh (px).
        height: u32,
    },
    /// Lỗi đọc/ghi file.
    Io {
        /// Đường dẫn file.
        path: PathBuf,
        /// Thông điệp lỗi gốc.
        message: String,
    },
    /// Lỗi mã hoá PNG.
    Png(String),
}

impl fmt::Display for BundleBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closure(e) => write!(f, "lỗi closure: {e}"),
            Self::Pack(e) => write!(f, "lỗi pack: {e}"),
            Self::Map(e) => write!(f, "lỗi map: {e}"),
            Self::Sprite(e) => write!(f, "lỗi sprite: {e}"),
            Self::Bundle(e) => write!(f, "lỗi bundle: {e}"),
            Self::UnsupportedMap(name) => {
                write!(f, "chưa hỗ trợ dựng bundle cho map {name:?}")
            }
            Self::ImageTooLarge { width, height } => write!(
                f,
                "ảnh {width}x{height} vượt trần atlas {ATLAS_WIDTH}x{ATLAS_HEIGHT}"
            ),
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Png(e) => write!(f, "lỗi mã hoá PNG: {e}"),
        }
    }
}

impl std::error::Error for BundleBuildError {}

impl From<crate::closure::ClosureError> for BundleBuildError {
    fn from(value: crate::closure::ClosureError) -> Self {
        Self::Closure(value)
    }
}

impl From<PackError> for BundleBuildError {
    fn from(value: PackError) -> Self {
        Self::Pack(value)
    }
}

impl From<MapError> for BundleBuildError {
    fn from(value: MapError) -> Self {
        Self::Map(value)
    }
}

impl From<SpriteError> for BundleBuildError {
    fn from(value: SpriteError) -> Self {
        Self::Sprite(value)
    }
}

impl From<BundleError> for BundleBuildError {
    fn from(value: BundleError) -> Self {
        Self::Bundle(value)
    }
}

/// Chuyển trạng thái bằng chứng của closure sang nhãn của bundle.
#[must_use]
pub fn status_label(status: Status) -> &'static str {
    EvidenceStatus::from(status).as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thư mục pack KR thật; test bỏ qua khi chưa đặt `BOOM_KR_PACK_DIR`.
    fn pack_root() -> Option<PathBuf> {
        let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
        Some(PathBuf::from(path))
    }

    /// Thư mục tạm cho test; tự xoá khi test xong.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("boom-bundle-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("tạo thư mục tạm");
        dir
    }

    /// Ảnh tổng hợp để kiểm thuật toán đóng gói (không cần pack thật).
    fn raw(width: u32, height: u32) -> RawImage {
        RawImage {
            width,
            height,
            rgba: vec![0x7Fu8; (width as usize) * (height as usize) * 4],
        }
    }

    #[test]
    fn atlas_packing_is_deterministic_and_inside_the_page() {
        let images = vec![raw(40, 40), raw(30, 70), raw(10, 10), raw(60, 20)];
        let first = pack_atlas(&images).expect("đóng gói");
        let second = pack_atlas(&images).expect("đóng gói lại");
        assert_eq!(first.len(), second.len());
        for (left, right) in first.iter().zip(second.iter()) {
            assert_eq!(left.height, right.height);
            assert_eq!(left.placements.len(), right.placements.len());
            for (a, b) in left.placements.iter().zip(right.placements.iter()) {
                assert_eq!((a.image, a.x, a.y), (b.image, b.x, b.y));
            }
        }
        // Mọi ảnh nằm trong trang, không chồng nhau theo trục x trong cùng kệ.
        for page in &first {
            assert!(page.height <= ATLAS_HEIGHT);
            for placement in &page.placements {
                let image = &images[placement.image];
                assert!(placement.x + image.width <= ATLAS_WIDTH);
                assert!(placement.y + image.height <= page.height);
            }
        }
    }

    #[test]
    fn atlas_rejects_an_image_larger_than_a_page() {
        let images = vec![raw(ATLAS_WIDTH + 1, 10)];
        assert!(matches!(
            pack_atlas(&images),
            Err(BundleBuildError::ImageTooLarge { .. })
        ));
    }

    /// Nghiệm thu V1-030: dựng bundle hai lần từ pack thật cho cùng `content_hash`, và
    /// reader trong `boom-content` nạp lại được, kiểm mọi tham chiếu.
    #[test]
    fn village00_bundle_is_reproducible_and_reloadable() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let first = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let second = build_bundle(&root, V1_MAP).expect("dựng bundle lần hai");
        assert_eq!(
            first.bundle.content_hash, second.bundle.content_hash,
            "dựng hai lần phải cho cùng content_hash"
        );
        assert_eq!(first.bundle, second.bundle, "metadata phải giống nhau");
        assert_eq!(first.pages, second.pages, "byte PNG phải giống nhau");

        // Ghi ra đĩa rồi nạp lại bằng reader của boom-content.
        let dir = temp_dir("repro");
        first.write(&dir).expect("ghi bundle");
        let loaded = boom_content::bundle::load(&dir).expect("nạp lại bundle");
        assert_eq!(loaded.content_hash, first.bundle.content_hash);
        assert_eq!(loaded.sprites.len(), first.bundle.sprites.len());
        assert_eq!(loaded.cells.len(), 15 * 13, "lưới Village00 là 15x13");

        // Mọi ô có sprite nền, hoặc nằm trong unknown kèm lý do.
        for cell in &loaded.cells {
            let tile = cell.tile.expect("ô Village00 luôn có nền");
            let resolved = loaded
                .sprites
                .iter()
                .any(|s| s.role == "tile" && s.map_id == Some(tile));
            let unknown = loaded
                .unknown
                .iter()
                .any(|u| u.id == format!("tile:{tile}") && !u.reason.is_empty());
            assert!(
                resolved || unknown,
                "ô ({}, {}) chưa có sprite",
                cell.x,
                cell.y
            );
        }
        // Có cả sprite nền lẫn vật cản cho Village00.
        assert!(
            loaded
                .sprites
                .iter()
                .any(|s| s.role == "object" && s.map_id.is_some()),
            "bundle phải có sprite vật cản"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Bundle của map khác chưa hỗ trợ thì báo lỗi rõ ràng.
    #[test]
    fn unsupported_map_is_reported() {
        match build_bundle(Path::new("/tmp/boom-khong-ton-tai"), "Village01") {
            Err(BundleBuildError::UnsupportedMap(name)) => assert_eq!(name, "Village01"),
            Err(other) => panic!("lỗi sai: {other}"),
            Ok(_) => panic!("map khác phải bị từ chối"),
        }
    }

    /// Nghiệm thu V1-030: pixel trong trang atlas phải khớp SHA-256 của ảnh đã ghi.
    ///
    /// Đọc lại PNG đã ghi bằng bộ giải mã PNG, cắt đúng vùng của từng ảnh và so với
    /// `pixel_sha256` trong metadata — kiểm cả đường giải mã → đóng gói → ghi file.
    #[test]
    fn atlas_pixels_match_the_recorded_image_hashes() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let mut checked = 0usize;
        for (page_index, bytes) in built.pages.iter().enumerate() {
            let page = &built.bundle.atlas[page_index];
            let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
            let mut reader = decoder.read_info().expect("đọc header PNG");
            let mut buffer = vec![
                0u8;
                reader
                    .output_buffer_size()
                    .expect("kích thước buffer PNG phải biết trước")
            ];
            let info = reader.next_frame(&mut buffer).expect("giải PNG");
            assert_eq!(info.color_type, png::ColorType::Rgba);
            assert_eq!(info.width, page.width);
            assert_eq!(info.height, page.height);
            let stride = page.width as usize * 4;
            for sprite in &built.bundle.sprites {
                for image in &sprite.images {
                    let Some(index) = image.page else {
                        continue;
                    };
                    assert_eq!(index as usize, page_index);
                    let width = image.width as usize;
                    let mut region = Vec::with_capacity(width * image.height as usize * 4);
                    for row in 0..image.height as usize {
                        let start = (image.y as usize + row) * stride + image.x as usize * 4;
                        region.extend_from_slice(&buffer[start..start + width * 4]);
                    }
                    assert_eq!(
                        sha256_hex(&region),
                        image.pixel_sha256,
                        "ảnh {} của sprite {} không khớp pixel trong atlas",
                        image.index,
                        sprite.id
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "số ảnh kiểm tra quá ít: {checked}");
    }
}
