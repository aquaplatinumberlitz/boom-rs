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
//!
//! Nhân vật, bom và lửa cũng theo cùng thứ tự hàng (BUNDLES §7): trong **cùng ô**,
//! thứ tự là vật cản → lửa → bom → nhân vật. Lửa là hiệu ứng nằm trên nền còn bom
//! là vật thể đứng trên nền, nên bom vẽ sau lửa.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use boom_content::bundle::{Bundle, BundleError, Clip, Frame, ROLE_OBJECT, ROLE_TILE, Sprite};
use boom_content::map::{
    CellClassifier, CellIds, CellKind, MapGrid, MapProfile, MapProfileError, Terrain,
};
use boom_core::{Cell, Life, SimulationState, World};

use crate::anim::{self, PresentationClock};

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
    /// Độ đục khi vẽ (255 = đục hẳn); bong bóng nhốt vẽ trong mờ (#320).
    pub alpha: u8,
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

/// Thứ tự vẽ trong **cùng một ô** (z-order).
///
/// Vật cản trước, rồi lửa (hiệu ứng trên nền), rồi bom (vật thể đứng trên nền), rồi
/// nhân vật (đứng trên bom khi mới đặt). BUNDLES §7.
const ORDER_OBJECT: u8 = 0;
/// Thứ tự vẽ: item nằm trên nền (dưới lửa/bom/nhân vật cùng chân).
const ORDER_ITEM: u8 = 1;
/// Thứ tự vẽ: lửa.
///
/// Hiệu ứng nổ dùng **cùng** bậc này: nó được đẩy **sau** lửa nên vẽ đè lên lửa ở
/// cùng ô (BUNDLES §7.1, R-088).
const ORDER_FLAME: u8 = 2;
/// Thứ tự vẽ: bom.
const ORDER_BOMB: u8 = 3;
/// Thứ tự vẽ: nhân vật.
const ORDER_CHARACTER: u8 = 4;
/// Thứ tự vẽ: bong bóng phủ lên người bị nhốt.
const ORDER_BUBBLE: u8 = 5;
/// Thứ tự vẽ: chữ HELP! trên bong bóng (video VN, F-02 #327).
const ORDER_HELP: u8 = 6;

/// Một lệnh vẽ của kế hoạch trình diễn.
#[derive(Clone, Debug, PartialEq)]
pub enum PlanItem {
    /// Vật cản của map (không phải clip): ô nền có vật cản.
    Object(DrawItem),
    /// Cắt vùng của trang atlas rồi đặt ở toạ độ logic.
    Sprite {
        /// ID clip dùng cho đối tượng này.
        clip: String,
        /// Chỉ số frame trong clip.
        frame: usize,
        /// Lệnh vẽ.
        item: DrawItem,
    },
    /// Ô màu thay thế: bundle **không** có clip cần cho trạng thái này.
    ///
    /// Client vẽ ô màu và ghi cảnh báo thay vì bỏ qua âm thầm (AGENTS §4) — và không
    /// panic.
    Fallback {
        /// ID clip mà bundle thiếu.
        clip: String,
        /// Ô thay thế.
        rect: PixelRect,
    },
}

/// Kế hoạch vẽ của một lượt: nền, rồi các đối tượng theo z-order.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// Lệnh vẽ nền, theo thứ tự hàng rồi cột.
    pub ground: Vec<DrawItem>,
    /// Lệnh vẽ vật cản, lửa, bom và nhân vật, theo z-order.
    pub items: Vec<PlanItem>,
    /// ID clip mà bundle **không** có (không trùng lặp).
    ///
    /// Client ghi cảnh báo cho từng clip thiếu; các đối tượng tương ứng vẽ ô màu
    /// thay thế.
    pub missing_clips: Vec<String>,
}

/// Dựng kế hoạch vẽ từ bundle, scene và state mô phỏng.
///
/// Hàm thuần: test được **không cần GPU** (acceptance V1-031). `previous` là vị trí
/// nhân vật ở tick trước (để biết nhân vật đang đi hay đứng yên — xem
/// [`anim::is_moving`]); thiếu thì coi như đứng yên.
///
/// `clock` là nhịp phát frame: frame tiến theo **tick mô phỏng**, không theo đồng hồ
/// wall clock (ADR 0004).
///
/// `journal` là sổ trình diễn (R-089): hiệu ứng nổ, chết, nhốt và mở đầu lượt. Trong khoảng
/// clip, ô của bom vẽ clip nổ `explosion.main` đè lên lửa; hết khoảng clip thì ô đó
/// về lại `flame.main` (BUNDLES §7.1, R-088). Chuỗi chết phát trọn clip rồi giữ khung
/// cuối có ảnh.
///
/// `clock` là **đồng hồ trình diễn**: tick của nó bằng `state.tick` khi mô phỏng còn chạy,
/// và **tiến nốt** sau khi lượt kết thúc để chuỗi chết phát trọn clip (R-089). Frame chọn
/// theo đồng hồ này, không theo đồng hồ wall clock (ADR 0004).
///
/// # Panics
///
/// Không panic: bundle thiếu clip thì đối tượng tương ứng được vẽ bằng ô màu thay thế và
/// ID clip được ghi vào [`Plan::missing_clips`]; frame **có** clip nhưng không có ảnh
/// (ảnh rỗng 0×0) thì đối tượng **không** được vẽ.
#[must_use]
pub fn presentation_plan(
    bundle: &Bundle,
    scene: &Scene,
    state: &SimulationState,
    world: &World,
    clock: &PresentationClock,
    previous: &BTreeMap<u8, (i32, i32)>,
    journal: &anim::PresentationJournal,
) -> Plan {
    let tile_px = scene.tile_px;
    let frames = frame_counts(bundle);
    let mut missing: Vec<String> = Vec::new();
    // (y chân theo px, cột, thứ tự cùng chân, lệnh vẽ) — đối tượng theo ô có chân ở
    // `hàng·T + T − 1` (R-210), nhân vật có chân theo toạ độ thật (#305).
    let mut items: Vec<(i32, u16, u8, PlanItem)> = Vec::new();

    for item in &scene.objects {
        // Gạch đã phá (ô chuyển sang nền) thì không vẽ nữa, còn vật cản cứng thì luôn vẽ.
        let current = state
            .field
            .kind(Cell::new(i32::from(item.column), i32::from(item.row)));
        if !object_visible(item.kind, current) {
            continue;
        }
        items.push((
            row_foot(item.row, tile_px),
            item.column,
            ORDER_OBJECT,
            PlanItem::Object(*item),
        ));
    }

    // Item trên nền (#305: trước đây không được vẽ — người chơi không thấy item rơi ra).
    for floor in &state.items {
        let Some(clip_id) = anim::item_clip(floor.kind.0) else {
            continue;
        };
        let (column, row) = cell_u16(floor.cell.x, floor.cell.y);
        let frame = bundle
            .clips
            .iter()
            .find(|clip| clip.id == clip_id)
            .map(|clip| anim::bomb_frame(clock, clip.frames.len()));
        if let Some(item) =
            clip_plan_item(bundle, clip_id, frame, column, row, tile_px, &mut missing)
        {
            items.push((row_foot(row, tile_px), column, ORDER_ITEM, item));
        }
    }

    for flame in &state.flames {
        let (column, row) = cell_u16(flame.cell.x, flame.cell.y);
        let frame = bundle
            .clips
            .iter()
            .find(|clip| clip.id == anim::FLAME_CLIP)
            .map(|clip| anim::flame_frame(clock, clip.frames.len()));
        if let Some(item) = clip_plan_item(
            bundle,
            anim::FLAME_CLIP,
            frame,
            column,
            row,
            tile_px,
            &mut missing,
        ) {
            items.push((row_foot(row, tile_px), column, ORDER_FLAME, item));
        }
    }

    for bomb in &state.bombs {
        let (column, row) = cell_u16(bomb.cell.x, bomb.cell.y);
        let frame = bundle
            .clips
            .iter()
            .find(|clip| clip.id == anim::BOMB_CLIP)
            .map(|clip| anim::bomb_frame(clock, clip.frames.len()));
        if let Some(item) = clip_plan_item(
            bundle,
            anim::BOMB_CLIP,
            frame,
            column,
            row,
            tile_px,
            &mut missing,
        ) {
            items.push((row_foot(row, tile_px), column, ORDER_BOMB, item));
        }
    }

    for player in &state.players {
        let cell = world.cell_at(player.x, player.y);
        let (column, row) = cell_u16(cell.x, cell.y);
        let moving = anim::is_moving((player.x.0, player.y.0), previous.get(&player.id).copied());
        let subject = anim::Subject::Player(player.id);
        // Hiệu ứng đang phát trong sổ trình diễn (chết/nhốt/mở đầu lượt) được ưu tiên
        // hơn clip theo state: chuỗi chết phát trọn clip rồi mới giữ khung cuối (R-089).
        let mut death_frame = None;
        let (clip_id, frame) = match journal.active_for(subject, clock, &frames) {
            Some(active) => {
                if active.kind == anim::PresentationKind::Death {
                    death_frame = Some(active.frame);
                }
                let clip = anim::presentation_clip(active.kind, subject).unwrap_or_else(|| {
                    anim::character_clip(anim::character_of(player.id), player.life, player.facing)
                });
                (clip, Some(active.frame))
            }
            None => {
                let clip =
                    anim::character_clip(anim::character_of(player.id), player.life, player.facing);
                let frame = match player.life {
                    // Nhân vật **chết**: giữ khung cuối **có ảnh** của clip chết. Khung
                    // cuối của clip chết trỏ ảnh rỗng 0×0 (dữ liệu thật của client gốc)
                    // nên nhân vật chết **không được vẽ** thành ô màu (R-089, BUNDLES §7.1).
                    Life::Dead => last_visible_frame(bundle, &clip),
                    // Đứng yên: giữ frame cuối của clip đi theo hướng cuối (R-198, #320).
                    Life::Alive if !moving => {
                        let count = clip_frame_count(bundle, &clip);
                        count.checked_sub(1)
                    }
                    // Đang đi hoặc bị nhốt: frame lặp theo tick mô phỏng.
                    _ => {
                        let count = clip_frame_count(bundle, &clip);
                        (count > 0).then(|| anim::character_loop_frame(clock, count))
                    }
                };
                (clip, frame)
            }
        };
        // Nhân vật vẽ theo toạ độ thật (không làm tròn về ô) và đủ hai lớp `static` +
        // `adjust`, mỗi lớp neo bằng offset của frame (#305).
        let (anchor_x, anchor_y) = actor_anchor(player.x.0, player.y.0, tile_px);
        for item in actor_plan_items(
            bundle,
            &clip_id,
            frame,
            (column, row),
            (anchor_x, anchor_y),
            tile_px,
            &mut missing,
        ) {
            items.push((anchor_y, column, ORDER_CHARACTER, item));
        }
        // Bong bóng phủ lên người bị nhốt: lớn dần một lần, giữ bóng tròn trong mờ, đục hẳn
        // trước khi vỡ; khi chết thì phủ chuỗi vỡ lên ba frame đầu của chuỗi chết (#320).
        let bubble = bundle
            .clips
            .iter()
            .find(|clip| clip.id == anim::TRAPPED_BUBBLE_CLIP);
        let look = match (player.life, bubble) {
            (Life::Trapped { since }, Some(bubble)) => {
                anim::bubble_look(clock, since, world.rules().trap_ticks, bubble.frames.len())
            }
            (Life::Dead, Some(bubble)) => death_frame
                .and_then(|frame| anim::bubble_pop_frame(frame, bubble.frames.len()))
                .map(|frame| anim::BubbleLook {
                    frame,
                    alpha: u8::MAX,
                }),
            _ => None,
        };
        if let (Some(look), Some(bubble)) = (look, bubble)
            && let Some(mut item) =
                clip_frame_item(bundle, bubble, look.frame, column, row, tile_px)
        {
            item.dest = actor_placement(&bubble.frames[look.frame], anchor_x, anchor_y);
            item.alpha = look.alpha;
            items.push((
                anchor_y,
                column,
                ORDER_BUBBLE,
                PlanItem::Sprite {
                    clip: bubble.id.clone(),
                    frame: look.frame,
                    item,
                },
            ));
        }
        // Chữ HELP! trên bóng suốt lúc bị nhốt, lặp 4 frame (F-02 #327). Bundle cũ không có
        // clip này thì bỏ qua: không phải clip bắt buộc.
        if matches!(player.life, Life::Trapped { .. })
            && let Some(help) = bundle
                .clips
                .iter()
                .find(|clip| clip.id == anim::TRAPPED_HELP_CLIP)
            && !help.frames.is_empty()
        {
            let index = anim::help_frame(clock, help.frames.len());
            if let Some(mut item) = clip_frame_item(bundle, help, index, column, row, tile_px) {
                item.dest = help_placement(bubble, &item.dest, anchor_x, anchor_y)
                    .unwrap_or_else(|| actor_placement(&help.frames[index], anchor_x, anchor_y));
                // Chữ HELP! nằm trên mọi thứ (video VN: đè cả vật cản cạnh bên).
                items.push((
                    i32::MAX,
                    column,
                    ORDER_HELP,
                    PlanItem::Sprite {
                        clip: help.id.clone(),
                        frame: index,
                        item,
                    },
                ));
            }
        }
    }

    // Hiệu ứng nổ: phát tại ô bom khi core báo nổ (`bomb_exploded`), trong khoảng
    // clip; hết khoảng clip thì ô đó về lại lửa. Đẩy **sau** lửa nên vẽ đè lên lửa
    // ở cùng ô (BUNDLES §7.1, R-088).
    //
    // Bundle không có clip nổ ⇒ không có hiệu ứng nổ để vẽ (không vẽ ô màu: ô đó
    // vẫn có lửa, đúng R-088).
    if bundle
        .clips
        .iter()
        .any(|clip| clip.id == anim::EXPLOSION_CLIP)
    {
        for active in journal.active(clock, &frames) {
            if active.kind != anim::PresentationKind::Explosion {
                continue;
            }
            let anim::Subject::Cell(cell) = active.subject else {
                continue;
            };
            let (column, row) = cell_u16(cell.x, cell.y);
            if let Some(item) = clip_plan_item(
                bundle,
                anim::EXPLOSION_CLIP,
                Some(active.frame),
                column,
                row,
                tile_px,
                &mut missing,
            ) {
                items.push((row_foot(row, tile_px), column, ORDER_FLAME, item));
            }
        }
    }

    // Z-order: chân thấp hơn (y lớn hơn) vẽ sau — gần người xem hơn; cùng chân thì cột lớn
    // hơn vẽ sau, rồi theo thứ tự trong ô. Nhân vật đi giữa hai hàng được che đúng theo y
    // chân thật thay vì theo ô làm tròn (#305).
    items.sort_by_key(|(row, column, order, _)| (*row, *column, *order));

    Plan {
        ground: scene.ground.clone(),
        items: items.into_iter().map(|(_, _, _, item)| item).collect(),
        missing_clips: missing,
    }
}

/// Lệnh vẽ của một clip tại ô; ô màu thay thế khi bundle thiếu clip.
///
/// `None` khi **không có gì để vẽ**: clip có trong bundle nhưng frame được chọn không
/// có ảnh (ảnh rỗng 0×0) — đối tượng **không** được vẽ thành ô màu (BUNDLES §7.1,
/// R-089). Clip **thiếu** trong bundle thì vẽ ô màu thay thế và ghi tên clip vào
/// [`Plan::missing_clips`] để client cảnh báo.
fn clip_plan_item(
    bundle: &Bundle,
    clip_id: &str,
    frame: Option<usize>,
    column: u16,
    row: u16,
    tile_px: u32,
    missing: &mut Vec<String>,
) -> Option<PlanItem> {
    let Some(clip) = bundle.clips.iter().find(|clip| clip.id == clip_id) else {
        note_missing(clip_id, missing);
        return Some(PlanItem::Fallback {
            clip: clip_id.to_owned(),
            rect: fallback_rect(column, row, tile_px),
        });
    };
    let index = frame?;
    // Frame không có ảnh (ảnh rỗng 0×0) thì không có gì để vẽ, nhưng clip vẫn có
    // mặt nên không coi là thiếu.
    // Hiệu ứng/item theo ô (bom, lửa, nổ, item) đặt theo offset frame tại neo ô (R-209/R-210),
    // không theo quy tắc canvas của tile — bản rà dsh1 #305: nổ lệch tới 33 px.
    let tile = i32::try_from(tile_px).unwrap_or(i32::MAX);
    let anchor = (
        i32::from(column).saturating_mul(tile),
        row_foot(row, tile_px),
    );
    clip_frame_item(bundle, clip, index, column, row, tile_px)
        .map(|mut item| {
            if let Some(frame) = clip.frames.get(index) {
                item.dest = actor_placement(frame, anchor.0, anchor.1);
            }
            item
        })
        .map(|item| PlanItem::Sprite {
            clip: clip_id.to_owned(),
            frame: index,
            item,
        })
}

/// Y chân (px) của đối tượng theo ô: `hàng·T + T − 1` (R-210).
fn row_foot(row: u16, tile_px: u32) -> i32 {
    let tile = i32::try_from(tile_px).unwrap_or(i32::MAX);
    i32::from(row).saturating_mul(tile).saturating_add(tile - 1)
}

/// Neo vẽ của nhân vật từ toạ độ core (mpx, tâm hộp): `x` = mép trái ô, `y` = chân
/// (`hàng·T + T − 1` khi đứng đúng ô) — cùng mốc với tile/object R-210 và R-009, nhưng giữ
/// phần lẻ pixel để nhân vật trượt mượt.
#[must_use]
pub fn actor_anchor(x_mpx: i32, y_mpx: i32, tile_px: u32) -> (i32, i32) {
    let tile = i32::try_from(tile_px).unwrap_or(i32::MAX);
    let x = x_mpx.div_euclid(boom_core::Mpx::PER_PX);
    let y = y_mpx.div_euclid(boom_core::Mpx::PER_PX);
    (x - tile / 2, y + tile / 2 - 1)
}

/// Đặt frame của actor: góc trên-trái vùng ảnh = neo − offset của frame (`f0`, `f1`; R-209).
///
/// Đối chiếu ảnh client gốc VN (`battle_village01`): lớp `static` + `adjust` của C00 đặt
/// theo công thức này đứng giữa ô, đầu nhô lên ô trên; quy tắc "đáy canvas = mép dưới ô"
/// của tile (BUNDLES §7) làm nhân vật lệch xuống và bị vật cản hàng dưới che (#305).
#[must_use]
pub fn actor_placement(frame: &Frame, anchor_x: i32, anchor_y: i32) -> PixelRect {
    PixelRect {
        x: anchor_x - frame.offset_x,
        y: anchor_y - frame.offset_y,
        width: u32::try_from((frame.x1 - frame.x0).max(0)).unwrap_or(0),
        height: u32::try_from((frame.y1 - frame.y0).max(0)).unwrap_or(0),
    }
}

/// Lệnh vẽ của nhân vật: lớp `static` của clip rồi lớp `adjust` cùng tên (nếu bundle có),
/// mỗi lớp đặt theo [`actor_placement`]. Thiếu clip `static` thì ô màu thay thế như
/// [`clip_plan_item`]; frame không có ảnh thì không vẽ lớp đó.
fn actor_plan_items(
    bundle: &Bundle,
    clip_id: &str,
    frame: Option<usize>,
    (column, row): (u16, u16),
    (anchor_x, anchor_y): (i32, i32),
    tile_px: u32,
    missing: &mut Vec<String>,
) -> Vec<PlanItem> {
    let Some(clip) = bundle.clips.iter().find(|clip| clip.id == clip_id) else {
        note_missing(clip_id, missing);
        return vec![PlanItem::Fallback {
            clip: clip_id.to_owned(),
            rect: fallback_rect(column, row, tile_px),
        }];
    };
    let Some(index) = frame else {
        return Vec::new();
    };
    let adjust_id = clip_id
        .strip_suffix(".static")
        .map(|base| format!("{base}.adjust"));
    let adjust = adjust_id
        .as_deref()
        .and_then(|id| bundle.clips.iter().find(|clip| clip.id == id));
    std::iter::once(clip)
        .chain(adjust)
        .filter_map(|layer| {
            let layer_index = index.min(layer.frames.len().checked_sub(1)?);
            let mut item = clip_frame_item(bundle, layer, layer_index, column, row, tile_px)?;
            item.dest = actor_placement(&layer.frames[layer_index], anchor_x, anchor_y);
            Some(PlanItem::Sprite {
                clip: layer.id.clone(),
                frame: layer_index,
                item,
            })
        })
        .collect()
}

/// Ghi lại clip thiếu (không trùng lặp) để client cảnh báo.
fn note_missing(clip_id: &str, missing: &mut Vec<String>) {
    if !missing.iter().any(|id| id == clip_id) {
        missing.push(clip_id.to_owned());
    }
}

/// Lệnh vẽ của một frame của clip; `None` nếu frame không có ảnh để vẽ.
///
/// Frame cuối của clip chết trỏ ảnh rỗng 0×0 (dữ liệu thật của client gốc) ⇒ không
/// có gì để vẽ.
#[must_use]
pub fn clip_frame_item(
    bundle: &Bundle,
    clip: &Clip,
    frame_index: usize,
    cell_x: u16,
    cell_y: u16,
    tile_px: u32,
) -> Option<DrawItem> {
    let frame = clip.frames.get(frame_index)?;
    let sprite = bundle
        .sprites
        .iter()
        .find(|sprite| sprite.id == clip.sprite)?;
    let image = sprite.images.get(frame.image as usize)?;
    let page = image.page?;
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let source = PixelRect {
        x: i32::try_from(image.x).unwrap_or(i32::MAX) + frame.x0,
        y: i32::try_from(image.y).unwrap_or(i32::MAX) + frame.y0,
        width: u32::try_from((frame.x1 - frame.x0).max(0)).unwrap_or(0),
        height: u32::try_from((frame.y1 - frame.y0).max(0)).unwrap_or(0),
    };
    Some(DrawItem {
        page,
        source,
        dest: frame_placement(frame, cell_x, cell_y, tile_px),
        row: cell_y,
        column: cell_x,
        kind: CellKind::Floor,
        alpha: u8::MAX,
    })
}

/// Chỗ đặt chữ HELP!: căn giữa ngang trên bóng tròn (frame giữ), đáy chữ cách đỉnh bóng
/// `HELP_GAP_PX` (video VN 26/10, quy về pixel game; F-02 #327, `vn_runtime` candidate).
///
/// `None` khi bundle không có bóng hoặc frame giữ để làm mốc.
fn help_placement(
    bubble: Option<&Clip>,
    help: &PixelRect,
    anchor_x: i32,
    anchor_y: i32,
) -> Option<PixelRect> {
    const HELP_GAP_PX: i32 = 2;
    let bubble = bubble?;
    let hold = bubble
        .frames
        .get(anim::BUBBLE_HOLD_FRAME.min(bubble.frames.len().checked_sub(1)?))?;
    let sphere = actor_placement(hold, anchor_x, anchor_y);
    let width = i32::try_from(help.width).ok()?;
    let height = i32::try_from(help.height).ok()?;
    let sphere_width = i32::try_from(sphere.width).ok()?;
    Some(PixelRect {
        x: sphere.x + (sphere_width - width) / 2,
        y: sphere.y - height + HELP_GAP_PX,
        width: help.width,
        height: help.height,
    })
}

/// Ô màu thay thế khi bundle thiếu clip: ô vuông đúng kích thước ô.
#[must_use]
pub fn fallback_rect(cell_x: u16, cell_y: u16, tile_px: u32) -> PixelRect {
    let tile = i32::try_from(tile_px).unwrap_or(i32::MAX);
    PixelRect {
        x: i32::from(cell_x) * tile,
        y: i32::from(cell_y) * tile,
        width: tile_px,
        height: tile_px,
    }
}

/// Ô của lưới (cột, hàng); ngoài lưới thì dùng giá trị lớn nhất để vẽ ở cuối.
fn cell_u16(x: i32, y: i32) -> (u16, u16) {
    (
        u16::try_from(x).unwrap_or(u16::MAX),
        u16::try_from(y).unwrap_or(u16::MAX),
    )
}

/// Số frame của mỗi clip trong bundle — bảng tra của sổ trình diễn.
///
/// Sổ trình diễn không biết bundle: caller đưa vào bảng số frame theo clip ID, nên đổi
/// bundle không phải sửa sổ (R-089).
#[must_use]
pub fn frame_counts(bundle: &Bundle) -> anim::FrameCounts {
    let mut counts = anim::FrameCounts::default();
    for clip in &bundle.clips {
        counts.set(&clip.id, clip.frames.len());
    }
    counts
}

/// Số frame của một clip trong bundle (0 nếu bundle không có clip đó).
#[must_use]
pub fn clip_frame_count(bundle: &Bundle, clip_id: &str) -> usize {
    bundle
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .map_or(0, |clip| clip.frames.len())
}

/// Khung cuối **có ảnh** của một clip; `None` nếu clip không có trong bundle hoặc
/// mọi frame đều trỏ ảnh rỗng 0×0.
///
/// Clip chết của pack KR 00967 kết thúc bằng **ảnh rỗng 0×0** (dữ liệu thật của
/// client gốc), nên nhân vật chết giữ khung cuối **có ảnh** để còn thấy xác, không
/// vẽ thành ô màu (R-089, BUNDLES §7.1).
#[must_use]
pub fn last_visible_frame(bundle: &Bundle, clip_id: &str) -> Option<usize> {
    let clip = bundle.clips.iter().find(|clip| clip.id == clip_id)?;
    let sprite = bundle
        .sprites
        .iter()
        .find(|sprite| sprite.id == clip.sprite)?;
    clip.frames
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, frame)| {
            let image = sprite.images.get(frame.image as usize)?;
            if image.width == 0 || image.height == 0 {
                return None;
            }
            image.page?;
            Some(index)
        })
}

/// Clip V1 mà client cần; bundle thiếu thì client vẽ ô màu thay thế.
///
/// Danh sách lấy từ bảng clip của V1 (R-077/R-078) cho hai nhân vật C00/C01, cộng
/// clip bom, clip lửa và clip của sổ trình diễn (nổ, chết, nhốt, mở đầu lượt — R-089).
#[must_use]
pub fn required_clip_ids() -> Vec<String> {
    let mut ids = Vec::new();
    for character in [anim::Character::C00, anim::Character::C01] {
        let key = character.key();
        for direction in ["up", "down", "left", "right"] {
            ids.push(format!("{key}.walk.{direction}.static"));
        }
        ids.push(format!("{key}.idle.blink.static"));
    }
    // Clip của sổ trình diễn: chết, nhốt, mở đầu lượt và nổ.
    ids.extend(anim::presentation_clip_ids());
    ids.push(anim::BOMB_CLIP.to_owned());
    ids.push(anim::FLAME_CLIP.to_owned());
    ids
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
        alpha: u8::MAX,
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
    use boom_content::animation::{Direction, FlameKind, FrameTiming, PresentationState};
    use boom_content::bundle::{
        AtlasPage, Cell, Clip, EvidenceStatus, Frame, Sprite, SpriteImage, Unknown,
    };
    use boom_content::rules::Source;
    use boom_core::{
        Bomb, CharacterStats, EntityId, EventId, EventKind, Flame, PlayerSetup, Rules,
        SimulationEvent, SimulationState, Stat, World, mode_by_id,
    };

    const TILE: u32 = 40;

    /// Nhịp test: 33 Hz, frame 100 ms (như profile dev V1).
    const CLOCK: crate::anim::FrameClock = crate::anim::FrameClock {
        frame_ms: 100,
        ticks_per_second: 33,
    };

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
            audio: Vec::new(),
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

    /// Sprite một ảnh cho clip: 40×40 trên trang atlas 0.
    fn clip_sprite(id: &str) -> Sprite {
        Sprite {
            id: id.to_owned(),
            entry: id.to_owned(),
            role: "character_walk".to_owned(),
            status: EvidenceStatus::Confirmed,
            source_sha256: "00".repeat(32),
            map_id: None,
            object_name: None,
            images: vec![SpriteImage {
                index: 0,
                page: Some(0),
                x: 0,
                y: 0,
                width: 40,
                height: 40,
                pixel_sha256: "11".repeat(32),
            }],
            frames: vec![ground_frame()],
        }
    }

    /// Clip test: `frame_count` frame, tất cả trỏ ảnh 0 của sprite.
    fn clip(id: &str, state: PresentationState, sprite: &str, frame_count: usize) -> Clip {
        Clip {
            id: id.to_owned(),
            state,
            pack: "FxCharacter".to_owned(),
            entry: "Character/Test".to_owned(),
            layer: None,
            sprite_unk: 0,
            sprite: sprite.to_owned(),
            source: Source::KrStatic,
            status: EvidenceStatus::Confirmed,
            claims: vec!["R-077".to_owned()],
            note: None,
            timing: FrameTiming::Unknown {
                reason: "chưa có nguồn".to_owned(),
            },
            frames: (0..frame_count).map(|_| ground_frame()).collect(),
        }
    }

    /// Bundle test có clip của V1: nhân vật (đi/đứng/chết), bom, lửa và nổ.
    fn clip_bundle() -> Bundle {
        let mut bundle = two_object_bundle();
        bundle.sprites.push(clip_sprite("c00"));
        bundle.sprites.push(clip_sprite("bomb"));
        bundle.sprites.push(clip_sprite("flame"));
        bundle.sprites.push(clip_sprite("explosion"));
        bundle.clips = vec![
            clip(
                "c00.walk.down.static",
                PresentationState::Walk(Direction::Down),
                "c00",
                5,
            ),
            clip("c00.idle.blink.static", PresentationState::Idle, "c00", 3),
            clip("c00.dead.static", PresentationState::Dead, "c00", 14),
            clip("bomb.fuse", PresentationState::BombFuse, "bomb", 4),
            clip(
                "flame.main",
                PresentationState::Flame(FlameKind::Main),
                "flame",
                100,
            ),
            clip(
                "explosion.main",
                PresentationState::Explosion,
                "explosion",
                10,
            ),
        ];
        bundle
    }

    /// Thế giới test 2×2: spawn ở (0,0), còn lại là nền.
    fn test_world() -> World {
        let terrain = Terrain::from_ascii(&["0.", ".."]).expect("bản đồ test");
        let profile = boom_content::rules::RulesProfile::from_toml(include_str!(
            "../../../data/rules/v1-dev.toml"
        ))
        .expect("rules");
        let rules = Rules::from_profile(&profile).expect("rules");
        let mode = mode_by_id("battle_ffa").expect("mode");
        World::new(terrain, rules, mode).expect("world")
    }

    /// Lượt test: một nhân vật ở spawn (0,0).
    fn test_state(world: &World) -> SimulationState {
        let roster = [PlayerSetup {
            id: 0,
            stats: CharacterStats {
                speed: Stat::new(1, 1),
                bombs: Stat::new(1, 1),
                range: Stat::new(1, 1),
            },
        }];
        SimulationState::new_round(world, &roster, 7).expect("lượt")
    }

    /// Đồng hồ trình diễn tại tick của state test.
    fn clock_at(tick: u32) -> PresentationClock {
        PresentationClock::new(CLOCK, tick)
    }

    /// Sổ trình diễn rỗng: chưa có hiệu ứng nào.
    fn empty_journal() -> anim::PresentationJournal {
        anim::PresentationJournal::new(0)
    }

    /// Event server đã xác nhận (nổ/chết/nhốt) để đưa vào sổ trình diễn.
    fn event(tick: u32, seq: u16, kind: EventKind) -> SimulationEvent {
        SimulationEvent {
            id: EventId { tick, seq },
            kind,
        }
    }

    /// Vụ nổ của bom ở `cell`.
    fn explosion_event(tick: u32, seq: u16, cell: boom_core::Cell) -> SimulationEvent {
        event(
            tick,
            seq,
            EventKind::BombExploded {
                bomb: EntityId(1),
                owner: 0,
                cell,
                chained: false,
            },
        )
    }

    /// Nhân vật `player` chết.
    fn died_event(tick: u32, seq: u16, player: u8) -> SimulationEvent {
        event(tick, seq, EventKind::PlayerDied { player, by: None })
    }

    /// Nghiệm thu V1-031 (không GPU): state core → clip/frame đúng cho đi, đứng,
    /// chết, bom và lửa.
    #[test]
    fn floor_items_and_trapped_bubble_are_drawn() {
        let mut bundle = clip_bundle();
        // Bong bóng: mượn một clip có ảnh, đổi ID (test không cần ảnh thật).
        let mut bubble = bundle
            .clips
            .iter()
            .find(|clip| clip.id == "c00.idle.blink.static")
            .expect("clip đứng")
            .clone();
        bubble.id = anim::TRAPPED_BUBBLE_CLIP.to_owned();
        bundle.clips.push(bubble);
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        state.items.push(boom_core::FloorItem {
            id: EntityId(9),
            kind: boom_core::ItemKind(1),
            cell: boom_core::Cell::new(1, 0),
        });
        state.players[0].life = Life::Trapped { since: state.tick };
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &empty_journal(),
        );
        let clips: Vec<&str> = plan
            .items
            .iter()
            .filter_map(|item| match item {
                PlanItem::Sprite { clip, .. } | PlanItem::Fallback { clip, .. } => {
                    Some(clip.as_str())
                }
                PlanItem::Object(_) => None,
            })
            .collect();
        // Item trên nền có lệnh vẽ (bundle test thiếu clip ⇒ ô màu thay thế, vẫn hiện).
        assert!(
            clips.contains(&"item.fire"),
            "item trên nền phải được vẽ: {clips:?}"
        );
        // Người bị nhốt có bong bóng, vẽ sau nhân vật.
        let character = clips.iter().position(|clip| clip.starts_with("c00."));
        let bubble = clips
            .iter()
            .position(|clip| *clip == anim::TRAPPED_BUBBLE_CLIP);
        assert!(
            bubble.is_some(),
            "người bị nhốt phải có bong bóng: {clips:?}"
        );
        assert!(character < bubble);
    }

    #[test]
    fn help_sits_on_top_of_the_trapped_bubble_and_over_everything() {
        let mut bundle = clip_bundle();
        let base = bundle
            .clips
            .iter()
            .find(|clip| clip.id == "c00.idle.blink.static")
            .expect("clip đứng")
            .clone();
        for id in [anim::TRAPPED_BUBBLE_CLIP, anim::TRAPPED_HELP_CLIP] {
            let mut clip = base.clone();
            clip.id = id.to_owned();
            bundle.clips.push(clip);
        }
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        let sprites = |state: &SimulationState| {
            presentation_plan(
                &bundle,
                &scene,
                state,
                &world,
                &clock_at(state.tick),
                &BTreeMap::new(),
                &empty_journal(),
            )
            .items
            .into_iter()
            .filter_map(|item| match item {
                PlanItem::Sprite { clip, item, .. } => Some((clip, item.dest)),
                _ => None,
            })
            .collect::<Vec<_>>()
        };
        // Còn sống: không có HELP!.
        assert!(
            sprites(&state)
                .iter()
                .all(|(clip, _)| clip != anim::TRAPPED_HELP_CLIP)
        );
        // Bị nhốt: HELP! vẽ **sau cùng**, căn giữa trên bóng tròn, đáy cách đỉnh bóng 2 px.
        state.players[0].life = Life::Trapped { since: state.tick };
        let drawn = sprites(&state);
        let (last, help) = drawn.last().expect("có lệnh vẽ");
        assert_eq!(last, anim::TRAPPED_HELP_CLIP);
        let bubble = bundle
            .clips
            .iter()
            .find(|clip| clip.id == anim::TRAPPED_BUBBLE_CLIP)
            .expect("bóng");
        let hold = &bubble.frames[anim::BUBBLE_HOLD_FRAME.min(bubble.frames.len() - 1)];
        let (anchor_x, anchor_y) = actor_anchor(state.players[0].x.0, state.players[0].y.0, TILE);
        let sphere = actor_placement(hold, anchor_x, anchor_y);
        assert_eq!(help.y + help.height as i32, sphere.y + 2);
        assert_eq!(
            help.x + help.width as i32 / 2,
            sphere.x + sphere.width as i32 / 2
        );
    }

    #[test]
    fn actor_anchor_keeps_sub_tile_pixels() {
        // Tâm ô (1, 1) ở 60 px ⇒ neo = mép trái ô và chân `hàng·40 + 39` (R-210/R-009).
        assert_eq!(actor_anchor(60_000, 60_000, 40), (40, 79));
        // Lệch nửa ô: neo trượt theo pixel, không làm tròn về ô (#305).
        assert_eq!(actor_anchor(80_000, 60_500, 40), (60, 79));
        assert_eq!(actor_anchor(61_999, 60_000, 40), (41, 79));
    }

    #[test]
    fn actor_placement_anchors_the_frame_by_its_offset() {
        let mut frame = ground_frame();
        frame.offset_x = 6;
        frame.offset_y = 60;
        frame.x0 = 0;
        frame.y0 = 0;
        frame.x1 = 52;
        frame.y1 = 55;
        assert_eq!(
            actor_placement(&frame, 40, 79),
            PixelRect {
                x: 34,
                y: 19,
                width: 52,
                height: 55
            }
        );
    }

    #[test]
    fn character_moves_by_pixels_draws_both_layers_and_sorts_by_foot() {
        let mut bundle = clip_bundle();
        // Lớp adjust của clip đứng: cùng frame, khác ID — client phải vẽ cả hai (#305).
        let mut adjust = bundle
            .clips
            .iter()
            .find(|clip| clip.id == "c00.walk.down.static")
            .expect("clip đứng")
            .clone();
        adjust.id = "c00.walk.down.adjust".to_owned();
        bundle.clips.push(adjust);
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        let sprites = |state: &SimulationState| {
            presentation_plan(
                &bundle,
                &scene,
                state,
                &world,
                &clock_at(state.tick),
                &BTreeMap::new(),
                &empty_journal(),
            )
            .items
            .into_iter()
            .filter_map(|item| match item {
                PlanItem::Sprite { clip, item, .. } if clip.starts_with("c00.") => {
                    Some((clip, item.dest))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
        };
        let before = sprites(&state);
        let layers: Vec<&str> = before.iter().map(|(clip, _)| clip.as_str()).collect();
        assert_eq!(layers, ["c00.walk.down.static", "c00.walk.down.adjust"]);
        // Dịch 7 px (chưa sang ô khác): ảnh dịch đúng 7 px.
        state.players[0].x = boom_core::Mpx(state.players[0].x.0 + 7_000);
        let after = sprites(&state);
        assert_eq!(after[0].1.x - before[0].1.x, 7);
        assert_eq!(after[0].1.y, before[0].1.y);
    }

    #[test]
    fn plan_picks_the_clip_of_the_core_state() {
        let bundle = clip_bundle();
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        // Nhân vật đã đi (vị trí khác tick trước) → clip đi theo hướng đang quay.
        let mut previous = BTreeMap::new();
        previous.insert(0, (state.players[0].x.0 - 1000, state.players[0].y.0));

        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &previous,
            &empty_journal(),
        );
        let clip_of = |item: &PlanItem| match item {
            PlanItem::Sprite { clip, frame, .. } => (clip.clone(), *frame),
            PlanItem::Fallback { clip, .. } => (clip.clone(), usize::MAX),
            PlanItem::Object(_) => (String::new(), usize::MAX),
        };
        let character = plan
            .items
            .iter()
            .map(clip_of)
            .find(|(clip, _)| clip.starts_with("c00."))
            .expect("nhân vật phải có lệnh vẽ");
        assert_eq!(character.0, "c00.walk.down.static");

        // Đứng yên (vị trí không đổi) → clip đi theo hướng cuối, giữ frame cuối (R-198, #320).
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &empty_journal(),
        );
        let character = plan
            .items
            .iter()
            .map(clip_of)
            .find(|(clip, _)| clip.starts_with("c00."))
            .expect("nhân vật phải có lệnh vẽ");
        let walk_frames = clip_frame_count(&bundle, "c00.walk.down.static");
        assert_eq!(
            character,
            ("c00.walk.down.static".to_owned(), walk_frames - 1)
        );

        // Bom đang chờ và lửa trên ô: mỗi ô một clip riêng.
        state.bombs.push(Bomb {
            id: EntityId(1),
            owner: 0,
            cell: boom_core::Cell::new(1, 0),
            range: 1,
            explodes_at: 100,
            grace: Vec::new(),
        });
        state.flames.push(Flame {
            cell: boom_core::Cell::new(0, 1),
            until: 100,
            source: EntityId(1),
            owner: 0,
        });
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &previous,
            &empty_journal(),
        );
        let clips: Vec<&str> = plan
            .items
            .iter()
            .map(|item| match item {
                PlanItem::Sprite { clip, .. } => clip.as_str(),
                PlanItem::Fallback { clip, .. } => clip.as_str(),
                PlanItem::Object(_) => "",
            })
            .collect();
        assert!(clips.contains(&"bomb.fuse"), "thiếu clip bom: {clips:?}");
        assert!(clips.contains(&"flame.main"), "thiếu clip lửa: {clips:?}");
        // Frame lửa nằm trong dải lõi của clip.
        let flame_frame = plan
            .items
            .iter()
            .find_map(|item| match item {
                PlanItem::Sprite { clip, frame, .. } if clip == "flame.main" => Some(*frame),
                _ => None,
            })
            .expect("lửa phải có frame");
        assert!((28..=59).contains(&flame_frame));
    }

    /// Nghiệm thu V1-031 (không GPU): clip nổ phát đúng ô bom theo event core, phát
    /// trong khoảng clip rồi ô đó về lại lửa.
    #[test]
    fn plan_plays_the_explosion_clip_at_the_bomb_cell() {
        let bundle = clip_bundle();
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        // Lửa ở ô bom (0, 1) — ô đặt bom luôn có lửa sau khi nổ.
        let bomb_cell = boom_core::Cell::new(0, 1);
        state.flames.push(Flame {
            cell: bomb_cell,
            until: 100,
            source: EntityId(1),
            owner: 0,
        });
        // Core báo nổ ở tick 10 ⇒ hiệu ứng nổ bắt đầu từ tick đó.
        let mut journal = empty_journal();
        assert!(journal.record_event(&explosion_event(10, 0, bomb_cell)));

        // Tick ngay sau event: ô đó vẽ clip nổ (đè lên lửa) và vẫn vẽ lửa.
        state.tick = 11;
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &journal,
        );
        let clips: Vec<&str> = plan
            .items
            .iter()
            .map(|item| match item {
                PlanItem::Sprite { clip, .. } => clip.as_str(),
                PlanItem::Fallback { clip, .. } => clip.as_str(),
                PlanItem::Object(_) => "",
            })
            .collect();
        assert!(
            clips.contains(&"explosion.main"),
            "thiếu clip nổ: {clips:?}"
        );
        assert!(clips.contains(&"flame.main"), "thiếu clip lửa: {clips:?}");

        // Frame nổ tính từ tick event, không theo đồng hồ wall clock.
        let explosion_frame = plan
            .items
            .iter()
            .find_map(|item| match item {
                PlanItem::Sprite { clip, frame, .. } if clip == "explosion.main" => Some(*frame),
                _ => None,
            })
            .expect("hiệu ứng nổ phải có frame");
        assert_eq!(explosion_frame, 0);
        // Lửa vẽ trước, nổ vẽ sau (nổ đè lên lửa ở cùng ô).
        let order = |clip: &str| {
            plan.items
                .iter()
                .position(|item| matches!(item, PlanItem::Sprite { clip: c, .. } if c == clip))
        };
        assert!(order("flame.main") < order("explosion.main"));

        // Hết khoảng clip (10 frame × 100 ms): ô đó về lại lửa, không còn clip nổ.
        state.tick = 10 + 33;
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &journal,
        );
        let clips: Vec<&str> = plan
            .items
            .iter()
            .map(|item| match item {
                PlanItem::Sprite { clip, .. } => clip.as_str(),
                PlanItem::Fallback { clip, .. } => clip.as_str(),
                PlanItem::Object(_) => "",
            })
            .collect();
        assert!(clips.contains(&"flame.main"), "thiếu clip lửa: {clips:?}");
        assert!(
            !clips.contains(&"explosion.main"),
            "hết khoảng clip thì không còn nổ: {clips:?}"
        );
    }

    /// Bundle không có clip nổ ⇒ không có hiệu ứng nổ để vẽ, không panic.
    #[test]
    fn missing_explosion_clip_draws_nothing_extra() {
        let mut bundle = clip_bundle();
        bundle.clips.retain(|clip| clip.id != "explosion.main");
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        let bomb_cell = boom_core::Cell::new(0, 1);
        state.flames.push(Flame {
            cell: bomb_cell,
            until: 100,
            source: EntityId(1),
            owner: 0,
        });
        let mut journal = empty_journal();
        assert!(journal.record_event(&explosion_event(10, 0, bomb_cell)));
        state.tick = 11;

        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &journal,
        );
        let clips: Vec<&str> = plan
            .items
            .iter()
            .map(|item| match item {
                PlanItem::Sprite { clip, .. } => clip.as_str(),
                PlanItem::Fallback { clip, .. } => clip.as_str(),
                PlanItem::Object(_) => "",
            })
            .collect();
        assert!(clips.contains(&"flame.main"), "thiếu clip lửa: {clips:?}");
        assert!(!clips.contains(&"explosion.main"), "{clips:?}");
    }

    /// Clip thiếu trong bundle → ô màu thay thế + ghi tên clip thiếu, không panic.
    #[test]
    fn missing_clip_falls_back_to_a_color_box() {
        let mut bundle = clip_bundle();
        bundle.clips.retain(|clip| clip.id != "flame.main");
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        state.flames.push(Flame {
            cell: boom_core::Cell::new(0, 1),
            until: 100,
            source: EntityId(1),
            owner: 0,
        });

        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &empty_journal(),
        );
        assert_eq!(plan.missing_clips, vec!["flame.main".to_owned()]);
        let fallback = plan
            .items
            .iter()
            .find_map(|item| match item {
                PlanItem::Fallback { clip, rect } => Some((clip.clone(), *rect)),
                _ => None,
            })
            .expect("thiếu clip thì phải có ô màu thay thế");
        assert_eq!(fallback.0, "flame.main");
        // Ô màu nằm đúng chỗ ô lửa (0, 1) và đúng kích thước ô.
        assert_eq!(
            fallback.1,
            PixelRect {
                x: 0,
                y: 40,
                width: TILE,
                height: TILE
            }
        );
    }

    /// Bundle có clip chết của pack: khung cuối trỏ **ảnh rỗng 0×0**.
    ///
    /// Dữ liệu thật của client gốc (R-086) — nhân vật chết phải giữ khung cuối **có
    /// ảnh**, không phải ô màu.
    fn bundle_with_dead_clip() -> Bundle {
        let mut bundle = clip_bundle();
        let sprite = bundle
            .sprites
            .iter_mut()
            .find(|sprite| sprite.id == "c00")
            .expect("sprite c00");
        sprite.images.push(SpriteImage {
            index: 1,
            page: Some(0),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            pixel_sha256: "22".repeat(32),
        });
        let dead = bundle
            .clips
            .iter_mut()
            .find(|clip| clip.id == "c00.dead.static")
            .expect("clip chết");
        dead.frames.last_mut().expect("khung cuối").image = 1;
        bundle
    }

    /// Nghiệm thu V1-031 (không GPU): chuỗi chết phát trọn clip rồi nhân vật chết giữ
    /// khung cuối **có ảnh** (không vẽ thành ô màu) — R-089.
    #[test]
    fn plan_plays_the_death_clip_then_holds_the_last_visible_frame() {
        let bundle = bundle_with_dead_clip();
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        // Nhân vật chết (core đã báo event `player_died`).
        state.players[0].life = Life::Dead;
        // Sổ trình diễn có event chết ở tick 10 ⇒ chuỗi chết bắt đầu từ tick đó.
        let mut journal = empty_journal();
        assert!(journal.record_event(&died_event(10, 0, 0)));

        let frame_of_dead = |plan: &Plan| {
            plan.items.iter().find_map(|item| match item {
                PlanItem::Sprite { clip, frame, .. } if clip == "c00.dead.static" => Some(*frame),
                _ => None,
            })
        };
        let has_fallback_of_dead = |plan: &Plan| {
            plan.items.iter().any(
                |item| matches!(item, PlanItem::Fallback { clip, .. } if clip == "c00.dead.static"),
            )
        };

        // Trong khoảng clip: nhân vật chết vẽ clip chết theo frame của sổ trình diễn.
        state.tick = 11;
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &journal,
        );
        assert_eq!(
            frame_of_dead(&plan),
            Some(0),
            "chuỗi chết bắt đầu ở frame 0"
        );
        assert!(!has_fallback_of_dead(&plan), "nhân vật chết không vẽ ô màu");

        // Hết khoảng clip (14 frame × 100 ms ≈ 14 × 3,3 tick): giữ khung cuối **có
        // ảnh** của clip chết (khung cuối trong pack là ảnh rỗng 0×0).
        state.tick = 10 + 14 * 4;
        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &journal,
        );
        assert_eq!(
            frame_of_dead(&plan),
            Some(12),
            "giữ khung cuối có ảnh của clip chết"
        );
        assert!(!has_fallback_of_dead(&plan), "nhân vật chết không vẽ ô màu");
    }

    /// Nhân vật chết mà clip chết **không có** khung nào có ảnh ⇒ không vẽ gì
    /// (không phải ô màu) — BUNDLES §7.1.
    #[test]
    fn dead_character_without_any_visible_frame_is_not_drawn() {
        let mut bundle = bundle_with_dead_clip();
        // Mọi khung của clip chết trỏ ảnh rỗng.
        let dead = bundle
            .clips
            .iter_mut()
            .find(|clip| clip.id == "c00.dead.static")
            .expect("clip chết");
        for frame in &mut dead.frames {
            frame.image = 1;
        }
        let scene = Scene::from_bundle(&bundle, &classifier(), TILE).expect("dựng scene");
        let world = test_world();
        let mut state = test_state(&world);
        state.players[0].life = Life::Dead;
        state.tick = 11;

        let plan = presentation_plan(
            &bundle,
            &scene,
            &state,
            &world,
            &clock_at(state.tick),
            &BTreeMap::new(),
            &empty_journal(),
        );
        let dead_items: Vec<&PlanItem> = plan
            .items
            .iter()
            .filter(|item| match item {
                PlanItem::Sprite { clip, .. } | PlanItem::Fallback { clip, .. } => {
                    clip == "c00.dead.static"
                }
                PlanItem::Object(_) => false,
            })
            .collect();
        assert!(
            dead_items.is_empty(),
            "nhân vật chết không có ảnh thì không vẽ: {dead_items:?}"
        );
    }
}
