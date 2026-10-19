//! Dựng scene từ bundle nội dung: địa hình, vị trí vẽ và z-order.
//!
//! Module thuần (không dùng macroquad) để test được **toạ độ vẽ** và **thứ tự
//! z-order** mà không cần GPU — acceptance của V1-030 (issue #71).
//!
//! Nguồn dữ liệu: [`boom_content::bundle::Bundle`] (atlas + metadata ô của map,
//! định dạng ở `docs/specification/assets/BUNDLES.md`) và bảng phân loại ô
//! `data/maps/*.toml` — **một nguồn chuẩn** cho loại ô (AGENTS §3): bundle không
//! nhân bản phân loại, client nạp cả hai.
//!
//! ## Quy ước trình diễn
//!
//! Dưới đây là quyết định **presentation** của client (`design`), không phải luật chơi:
//! mọi giá trị luật vẫn lấy từ rules profile (`grid.tile_px`, claim R-001). Nghĩa
//! `f0..f11` của client gốc còn là câu hỏi mở **Q-003**, nên quy ước được ghi ở
//! nguồn chuẩn ([BUNDLES §7](../../../../docs/specification/assets/BUNDLES.md)) để đổi
//! được khi có trace runtime.
//!
//! 1. Ô `(cx, cy)` rộng `T` px: mép trái ở `x = cx·T`, mép dưới ở `y = (cy+1)·T`.
//! 2. **Canvas** của frame (`f8`, `f9`) đặt sao cho *đáy canvas trùng mép dưới ô*
//!    và *trục X của pivot (`f10`) trùng mép trái ô*:
//!    `canvas.x = cx·T − pivot_x`, `canvas.y = (cy+1)·T − canvas_h`.
//! 3. **Vùng ảnh thật** của frame (`f4`–`f7`) đặt trong canvas: *canh giữa ngang*,
//!    *canh đáy*.
//!
//! Vì sao trục Y theo canvas chứ không theo `pivot_y`: ở Village00, `pivot_y` của
//! vật cản không trùng đáy canvas (`FxTile/Tile00/Fountain00` lệch 31 px), nên neo
//! theo `pivot_y` sẽ đặt vòi nước ~30 px dưới mặt đất. Canh đáy theo canvas đặt
//! đáy vùng ảnh đúng bằng mép dưới ô cho cả 5 vật cản của Village00.
//!
//! `R-009` (neo hiển thị **nhân vật**) là quan sát khác đối tượng, **không** dùng làm
//! bằng chứng cho neo vật cản.
//!
//! ## Z-order
//!
//! Nền vẽ trước, vật cản sau, vật cản sort theo hàng rồi cột: ô hàng lớn hơn
//! vẽ sau nên đối tượng ở hàng dưới (gần người xem hơn) che đúng đối tượng ở
//! hàng trên. Nhân vật (placeholder) chen vào cùng thứ tự theo hàng của ô đang
//! đứng.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use boom_content::bundle::{Bundle, BundleError, Frame, ROLE_OBJECT, ROLE_TILE, Sprite};
use boom_content::map::{
    CellClassifier, CellIds, CellKind, MapGrid, MapProfile, MapProfileError, Terrain,
};

/// Hình chữ nhật pixel trong không gian logic của client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    /// Toạ độ X góc trên-trái.
    pub x: i32,
    /// Toạ độ Y góc trên-trái.
    pub y: i32,
    /// Chiều rộng (px).
    pub width: u32,
    /// Chiều cao (px).
    pub height: u32,
}

/// Một lệnh vẽ: cắt một vùng của trang atlas rồi đặt ở toạ độ logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawItem {
    /// Chỉ số trang atlas chứa vùng nguồn.
    pub page: u32,
    /// Vùng nguồn trong trang atlas (px).
    pub source: PixelRect,
    /// Vị trí đích trong không gian logic (px).
    pub dest: PixelRect,
    /// Hàng ô — khoá z-order.
    pub row: u16,
    /// Cột ô.
    pub column: u16,
    /// Loại ô **ban đầu** theo bảng phân loại ô: vật cản phá được (`Breakable`) biến
    /// mất khi ô chuyển sang nền, còn vật cản cứng (`Solid`) thì luôn còn.
    pub kind: CellKind,
}

/// Thông tin một trang atlas (file và kích thước) để nạp texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasPageInfo {
    /// Đường dẫn tương đối trong thư mục bundle.
    pub file: String,
    /// Chiều rộng trang (px).
    pub width: u32,
    /// Chiều cao trang (px).
    pub height: u32,
}

/// Scene dựng từ bundle: nền, vật cản và thông tin map.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// Tên map trong bundle.
    pub map: String,
    /// Build của pack nguồn.
    pub build: String,
    /// Định danh nội dung (`content_hash` của bundle).
    pub content_hash: String,
    /// Số cột của lưới ô.
    pub width: u16,
    /// Số hàng của lưới ô.
    pub height: u16,
    /// Kích thước ô (px), lấy từ rules profile.
    pub tile_px: u32,
    /// Spawn của map, theo thứ tự file nguồn.
    pub spawns: Vec<(u16, u16)>,
    /// Các trang atlas cần nạp.
    pub pages: Vec<AtlasPageInfo>,
    /// Lệnh vẽ nền, theo thứ tự hàng rồi cột.
    pub ground: Vec<DrawItem>,
    /// Lệnh vẽ vật cản, sort theo hàng rồi cột (z-order).
    pub objects: Vec<DrawItem>,
    /// Số ô không có sprite trong bundle (bundle ghi kèm lý do trong `unknown`).
    pub unknown_cells: usize,
}

/// Lỗi khi dựng scene hoặc nạp bundle.
#[derive(Debug)]
pub enum SceneError {
    /// Lỗi nạp/kiểm bundle.
    Bundle(BundleError),
    /// Lỗi bảng phân loại ô.
    Map(MapProfileError),
    /// Bundle không có bảng phân loại ô tương ứng.
    NoMapProfile {
        /// Tên map trong bundle.
        map: String,
        /// Build trong bundle.
        build: String,
    },
    /// Ô cần sprite mà bundle không có (bundle đã validate nên hiếm).
    MissingSprite {
        /// Vai trò sprite cần (`tile`/`object`).
        role: &'static str,
        /// ID map của sprite.
        id: u32,
        /// Ô đang cần sprite.
        cell: (u16, u16),
    },
    /// Sprite không có frame nào để vẽ.
    NoFrame {
        /// Định danh sprite.
        sprite: String,
    },
    /// Bundle thiếu ô trong lưới.
    MissingCell {
        /// Ô bị thiếu.
        cell: (u16, u16),
    },
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bundle(e) => write!(f, "không nạp được bundle: {e}"),
            Self::Map(e) => write!(f, "bảng phân loại ô: {e}"),
            Self::NoMapProfile { map, build } => write!(
                f,
                "bundle map {map} (build {build}) không có bảng phân loại ô trong data/maps/"
            ),
            Self::MissingSprite { role, id, cell } => write!(
                f,
                "ô ({}, {}) cần sprite {role} ID {id} mà bundle không có",
                cell.0, cell.1
            ),
            Self::NoFrame { sprite } => write!(f, "sprite {sprite} không có frame để vẽ"),
            Self::MissingCell { cell } => {
                write!(f, "bundle thiếu ô ({}, {})", cell.0, cell.1)
            }
        }
    }
}

impl std::error::Error for SceneError {}

impl From<BundleError> for SceneError {
    fn from(value: BundleError) -> Self {
        Self::Bundle(value)
    }
}

impl From<MapProfileError> for SceneError {
    fn from(value: MapProfileError) -> Self {
        Self::Map(value)
    }
}

/// Bundle đã nạp kèm scene dựng từ nó.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    /// Metadata bundle (dùng cho atlas và đối chiếu `content_hash`).
    pub bundle: Bundle,
    /// Scene dựng từ bundle.
    pub scene: Scene,
}

/// Nạp bundle từ thư mục và dựng scene.
///
/// Lỗi (bundle sai hash, thiếu file atlas, metadata hỏng) trả về rõ ràng — không
/// panic, không đoán.
pub fn load(dir: &Path, profile: &MapProfile, tile_px: u32) -> Result<Loaded, SceneError> {
    // `boom_content::bundle::load` kiểm `content_hash` và SHA-256 từng trang atlas.
    let bundle = boom_content::bundle::load(dir)?;
    check_profile(&bundle, profile)?;
    let classifier = profile.classifier()?;
    let scene = Scene::from_bundle(&bundle, &classifier, tile_px)?;
    Ok(Loaded { bundle, scene })
}

/// Vật cản còn được vẽ không.
///
/// Ô **gạch phá được** (`Breakable`) chuyển sang nền (`Floor`) sau khi bị nổ
/// (`boom-core::systems::blocks`) → hình gạch phải biến mất, nếu không người chơi đi
/// xuyên qua hình. Vật cản cứng (`Solid`) thì luôn còn.
///
/// `current` là loại ô **hiện tại** trong state mô phỏng; `None` = ngoài lưới (không
/// vẽ). Hàm thuần: test được không cần GPU.
#[must_use]
pub fn object_visible(initial: CellKind, current: Option<CellKind>) -> bool {
    if initial != CellKind::Breakable {
        return true;
    }
    !matches!(current, Some(CellKind::Floor))
}

impl Scene {
    /// Dựng scene từ bundle đã nạp.
    ///
    /// `classifier` là bảng phân loại ô theo build (một nguồn chuẩn, `data/maps/`):
    /// scene cần loại ô **ban đầu** để biết vật cản nào là gạch phá được (xem
    /// [`object_visible`]).
    ///
    /// `tile_px` là kích thước ô từ rules profile (`grid.tile_px`, claim R-001).
    pub fn from_bundle(
        bundle: &Bundle,
        classifier: &CellClassifier,
        tile_px: u32,
    ) -> Result<Self, SceneError> {
        // Index sprite theo (vai trò, ID map) để tra ô nhanh.
        let mut sprites: BTreeMap<(&str, u32), &Sprite> = BTreeMap::new();
        for sprite in &bundle.sprites {
            if let Some(map_id) = sprite.map_id {
                sprites.insert((sprite.role.as_str(), map_id), sprite);
            }
        }
        // Ô theo toạ độ: bundle không cam kết thứ tự `cells`.
        let mut cells: BTreeMap<(u16, u16), &boom_content::bundle::Cell> = BTreeMap::new();
        for cell in &bundle.cells {
            cells.insert((cell.x, cell.y), cell);
        }

        let mut ground = Vec::new();
        let mut objects = Vec::new();
        let mut unknown_cells = 0usize;
        for y in 0..bundle.height {
            for x in 0..bundle.width {
                let cell = cells
                    .get(&(x, y))
                    .ok_or(SceneError::MissingCell { cell: (x, y) })?;
                if let Some(tile) = cell.tile {
                    match sprites.get(&(ROLE_TILE, tile)) {
                        Some(sprite) => ground.push(draw_item(sprite, x, y, tile_px)?),
                        // Ô chưa xác định: bundle ghi kèm lý do trong `unknown`.
                        None if has_unknown(bundle, &format!("tile:{tile}")) => unknown_cells += 1,
                        None => {
                            return Err(SceneError::MissingSprite {
                                role: ROLE_TILE,
                                id: tile,
                                cell: (x, y),
                            });
                        }
                    }
                }
                if let Some(object) = cell.object {
                    match sprites.get(&(ROLE_OBJECT, object)) {
                        Some(sprite) => {
                            let kind = classifier.classify(CellIds {
                                tile: cell.tile,
                                obj: cell.object,
                            });
                            let mut item = draw_item(sprite, x, y, tile_px)?;
                            item.kind = kind;
                            objects.push(item);
                        }
                        None if has_unknown(bundle, &format!("obj:{object}")) => unknown_cells += 1,
                        None => {
                            return Err(SceneError::MissingSprite {
                                role: ROLE_OBJECT,
                                id: object,
                                cell: (x, y),
                            });
                        }
                    }
                }
            }
        }
        // Z-order: hàng lớn hơn vẽ sau (gần người xem hơn), cùng hàng thì cột lớn hơn vẽ sau.
        objects.sort_by_key(|item| (item.row, item.column));

        Ok(Self {
            map: bundle.map.clone(),
            build: bundle.build.clone(),
            content_hash: bundle.content_hash.clone(),
            width: bundle.width,
            height: bundle.height,
            tile_px,
            spawns: bundle.spawns.clone(),
            pages: bundle
                .atlas
                .iter()
                .map(|page| AtlasPageInfo {
                    file: page.file.clone(),
                    width: page.width,
                    height: page.height,
                })
                .collect(),
            ground,
            objects,
            unknown_cells,
        })
    }

    /// Kích thước logic của scene (px), chưa gồm HUD.
    #[must_use]
    pub fn map_size(&self) -> [u32; 2] {
        [
            u32::from(self.width) * self.tile_px,
            u32::from(self.height) * self.tile_px,
        ]
    }
}

/// Bundle có ghi ID này trong `unknown` (kèm lý do) không.
fn has_unknown(bundle: &Bundle, id: &str) -> bool {
    bundle.unknown.iter().any(|unknown| unknown.id == id)
}

/// Lệnh vẽ của một ô: sprite đầu tiên có frame nhìn thấy được.
///
/// Frame đầu tiên dùng làm "trạng thái tĩnh" của ô; animation là task khác
/// (V1-031) nên chưa có clip trong bundle.
fn draw_item(
    sprite: &Sprite,
    cell_x: u16,
    cell_y: u16,
    tile_px: u32,
) -> Result<DrawItem, SceneError> {
    let frame = sprite
        .frames
        .iter()
        .find(|frame| {
            sprite
                .images
                .get(frame.image as usize)
                .is_some_and(|image| image.width > 0 && image.height > 0)
        })
        .ok_or_else(|| SceneError::NoFrame {
            sprite: sprite.id.clone(),
        })?;
    let image = &sprite.images[frame.image as usize];
    let page = image.page.ok_or_else(|| SceneError::NoFrame {
        sprite: sprite.id.clone(),
    })?;
    let source = PixelRect {
        x: i32::try_from(image.x).unwrap_or(i32::MAX) + frame.x0,
        y: i32::try_from(image.y).unwrap_or(i32::MAX) + frame.y0,
        width: u32::try_from((frame.x1 - frame.x0).max(0)).unwrap_or(0),
        height: u32::try_from((frame.y1 - frame.y0).max(0)).unwrap_or(0),
    };
    Ok(DrawItem {
        page,
        source,
        dest: frame_placement(frame, cell_x, cell_y, tile_px),
        row: cell_y,
        column: cell_x,
        kind: CellKind::Floor,
    })
}

/// Toạ độ vẽ của một frame trong ô `(cx, cy)` — quy ước trình diễn ở đầu module.
///
/// Hàm thuần: test được bằng số, không cần GPU.
#[must_use]
pub fn frame_placement(frame: &Frame, cell_x: u16, cell_y: u16, tile_px: u32) -> PixelRect {
    let tile = i32::try_from(tile_px).unwrap_or(i32::MAX);
    let cell_left = i32::from(cell_x) * tile;
    let cell_bottom = (i32::from(cell_y) + 1) * tile;
    let canvas_w = u32::try_from(frame.canvas_width.max(0)).unwrap_or(0);
    let canvas_h = u32::try_from(frame.canvas_height.max(0)).unwrap_or(0);
    let src_w = u32::try_from((frame.x1 - frame.x0).max(0)).unwrap_or(0);
    let src_h = u32::try_from((frame.y1 - frame.y0).max(0)).unwrap_or(0);

    // Canvas: đáy trùng mép dưới ô, trục X của pivot trùng mép trái ô.
    let canvas_x = cell_left - frame.pivot_x;
    let canvas_y = cell_bottom - i32::try_from(canvas_h).unwrap_or(i32::MAX);
    // Vùng ảnh: canh giữa ngang, canh đáy trong canvas.
    let x = canvas_x + i32::try_from(canvas_w.saturating_sub(src_w) / 2).unwrap_or(0);
    let y = canvas_y + i32::try_from(canvas_h.saturating_sub(src_h)).unwrap_or(0);
    PixelRect {
        x,
        y,
        width: src_w,
        height: src_h,
    }
}

/// Lưới ô theo build từ bundle: ID tile/obj giữ nguyên như file nguồn.
pub fn map_grid(bundle: &Bundle) -> Result<MapGrid, SceneError> {
    let width = usize::from(bundle.width);
    let height = usize::from(bundle.height);
    let mut cells = vec![
        CellIds {
            tile: None,
            obj: None,
        };
        width * height
    ];
    for cell in &bundle.cells {
        let index = usize::from(cell.y) * width + usize::from(cell.x);
        let Some(slot) = cells.get_mut(index) else {
            return Err(SceneError::MissingCell {
                cell: (cell.x, cell.y),
            });
        };
        *slot = CellIds {
            tile: cell.tile,
            obj: cell.object,
        };
    }
    Ok(MapGrid {
        width: bundle.width,
        height: bundle.height,
        cells,
        spawns: bundle.spawns.clone(),
    })
}

/// Địa hình đã phân loại: bundle + bảng phân loại ô.
///
/// Bundle **không** nhân bản phân loại ô (một nguồn chuẩn, AGENTS §3): client nạp
/// cả hai, và từ chối bundle không khớp bảng phân loại.
pub fn terrain(bundle: &Bundle, profile: &MapProfile) -> Result<Terrain, SceneError> {
    check_profile(bundle, profile)?;
    let grid = map_grid(bundle)?;
    Ok(profile.classify(&grid)?)
}

/// Bundle có khớp bảng phân loại ô không (một nguồn chuẩn, AGENTS §3).
fn check_profile(bundle: &Bundle, profile: &MapProfile) -> Result<(), SceneError> {
    if profile.map != bundle.map || profile.build != bundle.build {
        return Err(SceneError::NoMapProfile {
            map: bundle.map.clone(),
            build: bundle.build.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::bundle::{
        AtlasPage, Cell, EvidenceStatus, Frame, Sprite, SpriteImage, Unknown,
    };

    const TILE: u32 = 40;

    /// Sprite một frame, đặt tại `(x, y)` trong trang atlas `page`.
    fn sprite(id: &str, role: &str, map_id: u32, frame: Frame, at: (u32, u32)) -> Sprite {
        Sprite {
            id: id.to_owned(),
            entry: id.to_owned(),
            role: role.to_owned(),
            status: EvidenceStatus::Confirmed,
            source_sha256: "00".repeat(32),
            map_id: Some(map_id),
            object_name: None,
            images: vec![SpriteImage {
                index: 0,
                page: Some(0),
                x: at.0,
                y: at.1,
                width: 40,
                height: 40,
                pixel_sha256: "11".repeat(32),
            }],
            frames: vec![frame],
        }
    }

    /// Frame 40×40, canvas 40×40, pivot (0, 39) — như tile nền của Village00.
    fn ground_frame() -> Frame {
        Frame {
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
        }
    }

    /// Bundle tối thiểu: một trang atlas, một tile, một vật cản, lưới 2×2.
    fn sample_bundle(cells: Vec<Cell>, sprites: Vec<Sprite>) -> Bundle {
        Bundle {
            schema: boom_content::bundle::SCHEMA,
            build: "kr".to_owned(),
            map: "Village00".to_owned(),
            tileset: "Tile00".to_owned(),
            width: 2,
            height: 2,
            spawns: vec![(0, 0)],
            content_hash: String::new(),
            atlas: vec![AtlasPage {
                file: "atlas/page-000.png".to_owned(),
                width: 2048,
                height: 1241,
                sha256: "22".repeat(32),
            }],
            sprites,
            assets: Vec::new(),
            cells,
            clips: Vec::new(),
            ui: Vec::new(),
            unknown: Vec::new(),
        }
    }

    /// Bảng phân loại ô cho fixture: obj 10 là gạch phá được, obj 11 là vật cản cứng.
    fn classifier() -> CellClassifier {
        let mut classifier = CellClassifier {
            empty: Some(CellKind::Floor),
            ..Default::default()
        };
        classifier.objects.insert(10, CellKind::Breakable);
        classifier.objects.insert(11, CellKind::Solid);
        classifier
    }

    /// Bundle 2×2: ô (0,0) có vật cản, ô (1,1) có vật cản cao hơn.
    fn two_object_bundle() -> Bundle {
        let mut tall = ground_frame();
        tall.y1 = 67;
        tall.canvas_height = 70;
        tall.pivot_y = 69;
        sample_bundle(
            vec![
                Cell {
                    x: 0,
                    y: 0,
                    tile: Some(1),
                    object: Some(10),
                },
                Cell {
                    x: 1,
                    y: 0,
                    tile: Some(1),
                    object: None,
                },
                Cell {
                    x: 0,
                    y: 1,
                    tile: Some(1),
                    object: None,
                },
                Cell {
                    x: 1,
                    y: 1,
                    tile: Some(1),
                    object: Some(11),
                },
            ],
            vec![
                sprite("tile", ROLE_TILE, 1, ground_frame(), (0, 0)),
                sprite("obj-short", ROLE_OBJECT, 10, ground_frame(), (40, 0)),
                sprite("obj-tall", ROLE_OBJECT, 11, tall, (80, 0)),
            ],
        )
    }

    #[test]
    fn frame_placement_uses_canvas_and_pivot() {
        // Tile 40×40: canvas 40×40, pivot (0, 39) → đúng ô (cx, cy).
        let dest = frame_placement(&ground_frame(), 3, 2, TILE);
        assert_eq!(
            dest,
            PixelRect {
                x: 120,
                y: 80,
                width: 40,
                height: 40
            }
        );

        // Vật cản cao: canvas 40×70 → đáy canvas trùng mép dưới ô (y = 120),
        // vùng ảnh 40×67 canh đáy → đỉnh ở y = 120 − 67 = 53.
        let mut tall = ground_frame();
        tall.y1 = 67;
        tall.canvas_height = 70;
        tall.pivot_y = 69;
        let dest = frame_placement(&tall, 3, 2, TILE);
        assert_eq!(
            dest,
            PixelRect {
                x: 120,
                y: 53,
                width: 40,
                height: 67
            }
        );

        // Pivot X lệch mép trái ô → vùng ảnh canh giữa trong canvas.
        let mut boxy = ground_frame();
        boxy.x1 = 40;
        boxy.y1 = 44;
        boxy.canvas_width = 42;
        boxy.canvas_height = 46;
        boxy.pivot_x = 1;
        boxy.pivot_y = 44;
        let dest = frame_placement(&boxy, 0, 0, TILE);
        assert_eq!(
            dest,
            PixelRect {
                x: 0,
                y: -4,
                width: 40,
                height: 44
            }
        );
    }

    #[test]
    fn objects_are_ordered_by_row_then_column() {
        let bundle = two_object_bundle();
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        assert_eq!(scene.ground.len(), 4);
        assert_eq!(scene.objects.len(), 2);
        // Hàng 0 trước hàng 1: đối tượng ở hàng dưới (gần hơn) che đúng.
        assert_eq!((scene.objects[0].row, scene.objects[0].column), (0, 0));
        assert_eq!((scene.objects[1].row, scene.objects[1].column), (1, 1));
        // Vật cản cao vươn lên trên ô của nó nhưng không vượt mép dưới.
        assert_eq!(scene.objects[1].dest.y, 40 + TILE as i32 - 67);
    }

    #[test]
    fn ground_and_objects_keep_their_cell_anchor() {
        let bundle = two_object_bundle();
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        // Ô (0, 0): nền ở (0, 0).
        assert_eq!(scene.ground[0].dest.x, 0);
        assert_eq!(scene.ground[0].dest.y, 0);
        // Ô (1, 1): nền ở (40, 40).
        assert_eq!(scene.ground[3].dest.x, 40);
        assert_eq!(scene.ground[3].dest.y, 40);
    }

    #[test]
    fn objects_keep_their_initial_cell_kind() {
        let bundle = two_object_bundle();
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        // Ô (0, 0) có obj 10 (gạch phá được), ô (1, 1) có obj 11 (vật cản cứng).
        assert_eq!(scene.objects[0].kind, CellKind::Breakable);
        assert_eq!(scene.objects[1].kind, CellKind::Solid);
    }

    #[test]
    fn broken_block_is_not_drawn_anymore_but_solid_object_stays() {
        // Gạch phá được: ô còn là gạch thì vẽ; sau khi nổ ô thành nền thì không vẽ.
        assert!(object_visible(
            CellKind::Breakable,
            Some(CellKind::Breakable)
        ));
        assert!(!object_visible(CellKind::Breakable, Some(CellKind::Floor)));
        // Vật cản cứng: luôn vẽ.
        assert!(object_visible(CellKind::Solid, Some(CellKind::Solid)));
        assert!(object_visible(CellKind::Solid, Some(CellKind::Floor)));
    }

    #[test]
    fn missing_object_sprite_is_reported_not_guessed() {
        let mut bundle = two_object_bundle();
        bundle.sprites.retain(|sprite| sprite.role != ROLE_OBJECT);
        let error =
            Scene::from_bundle(&bundle, &classifier(), TILE).expect_err("thiếu sprite vật cản");
        assert!(matches!(
            error,
            SceneError::MissingSprite {
                role: ROLE_OBJECT,
                ..
            }
        ));
    }

    #[test]
    fn unknown_cell_without_sprite_is_counted_not_fatal() {
        let mut bundle = two_object_bundle();
        bundle.sprites.retain(|sprite| sprite.role != ROLE_OBJECT);
        for id in ["obj:10", "obj:11"] {
            bundle.unknown.push(Unknown {
                kind: "map_cell".to_owned(),
                id: id.to_owned(),
                reason: "pack không có entry cho vật cản này".to_owned(),
                candidates: Vec::new(),
            });
        }
        let scene =
            Scene::from_bundle(&bundle, &classifier(), TILE).expect("ô unknown không chặn scene");
        assert_eq!(scene.unknown_cells, 2);
        assert_eq!(scene.objects.len(), 0);
    }

    #[test]
    fn terrain_comes_from_bundle_cells_and_profile() {
        let profile = MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml"))
            .expect("bảng phân loại ô");
        // Lưới 15×13 như profile: ô (0, 0) là cây (obj 1958 → solid), còn lại nền.
        let mut cells = Vec::new();
        for y in 0..13u16 {
            for x in 0..15u16 {
                cells.push(Cell {
                    x,
                    y,
                    tile: Some(1943),
                    object: if x == 0 && y == 0 { Some(1958) } else { None },
                });
            }
        }
        let bundle = Bundle {
            width: 15,
            height: 13,
            cells,
            spawns: vec![(1, 1)],
            ..sample_bundle(Vec::new(), Vec::new())
        };
        let terrain = terrain(&bundle, &profile).expect("phân loại ô");
        assert_eq!((terrain.width, terrain.height), (15, 13));
        assert_eq!(terrain.spawns, vec![(1, 1)]);
        assert!(!terrain.has_unknown());
        assert_eq!(terrain.kind(0, 0), Some(boom_content::map::CellKind::Solid));
        assert_eq!(terrain.kind(1, 1), Some(boom_content::map::CellKind::Floor));
    }

    #[test]
    fn bundle_of_another_map_is_rejected_with_a_clear_error() {
        let profile = MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml"))
            .expect("bảng phân loại ô");
        let mut bundle = two_object_bundle();
        bundle.map = "Village01".to_owned();
        let error = terrain(&bundle, &profile).expect_err("map khác profile");
        assert!(matches!(error, SceneError::NoMapProfile { .. }));
        assert!(error.to_string().contains("Village01"));
    }

    #[test]
    fn loading_a_broken_bundle_reports_an_error_instead_of_panicking() {
        // Thư mục không có bundle.toml → lỗi rõ, không panic.
        let profile = MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml"))
            .expect("bảng phân loại ô");
        let error = load(
            Path::new("/khong/ton/tai/boom-client-bundle"),
            &profile,
            TILE,
        )
        .expect_err("thư mục không tồn tại");
        assert!(matches!(error, SceneError::Bundle(_)));
        assert!(error.to_string().contains("bundle"));
    }
}
