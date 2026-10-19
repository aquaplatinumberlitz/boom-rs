//! Bộ trích animation V1 từ pack của client gốc: entry → clip (frame, pivot, thời lượng).
//!
//! Module này **không giải mã ảnh**: `crate::sprite` đã giải mã blob `CDataI16Alpha`
//! v2; ở đây chỉ ghép entry thành clip và kiểm dãi frame (chỉ số ảnh trong miền, rect
//! nằm trong ảnh) theo schema của [`boom_content::animation`].
//!
//! Nguồn:
//!
//! - [MAPS §3](../../../../docs/specification/assets/MAPS.md) — layout frame 48 byte.
//! - [PACKS](../../../../docs/specification/assets/PACKS.md) — định dạng pack `.idx/.idd`.
//! - `docs/specification/gameplay/RULES_V1.md` — claim ánh xạ state → animation (R-077…R-079).
//!
//! **Thời lượng frame không có nguồn trong pack**: blob `CDataI16Alpha` v2 chỉ có 12 số
//! mỗi frame và không mang trường thời gian (MAPS §3), còn consumer phía client thì
//! chưa định vị được. Vì vậy mọi clip ở đây mang `FrameTiming::Unknown` kèm lý do —
//! không tự đặt một hằng số thời lượng (R-079, Q-003).

use std::collections::BTreeMap;
use std::path::Path;

use boom_content::animation::{
    AnimationClip, AnimationError, AnimationFrame, AnimationSet, CanvasSize, Direction, FlameKind,
    FrameTiming, ImageSize, Pivot, PresentationState, SourceRect, SpriteLayer, validate_frames,
};
use boom_content::rules::{Source, Status};

use crate::legacy_map::Build;
use crate::pack::{Pack, PackError};
use crate::sprite::{self, SpriteError};

/// Pack chứa sprite nhân vật.
pub const CHARACTER_PACK: &str = "FxCharacter";

/// Pack chứa sprite bom, lửa và bong bóng bị nhốt.
pub const BOMB_PACK: &str = "FxDesktop";

/// Pack chứa icon item và lửa nhạt (motion blur).
pub const FX_PACK: &str = "fx";

/// Claim RULES_V1: entry animation V1 có trong pack KR 00967 và dãi frame hợp lệ.
pub const CLAIM_CONFIRMED: &str = "R-077";

/// Claim RULES_V1: mapping state → animation mới là ứng viên (chưa có consumer).
pub const CLAIM_CANDIDATE: &str = "R-078";

/// Lý do dùng cho mọi clip khi chưa có nguồn thời lượng frame.
pub const NO_TIMING_SOURCE: &str = "pack client gốc không mang trường thời lượng frame \
    (blob CDataI16Alpha v2 chỉ có 12 số mỗi frame, MAPS §3) và consumer phía client chưa \
    định vị được; V1 chưa chốt thời lượng nên không tự đặt (R-079, Q-003)";

/// Nhân vật V1 (SCOPE_V1 §1).
const CHARACTERS: &[&str] = &["C00", "C01"];

/// Hướng đi của V1.
const DIRECTIONS: &[Direction] = &[
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

/// Lớp sprite của nhân vật: nền và lớp ngoại hình điều chỉnh.
const LAYERS: &[SpriteLayer] = &[SpriteLayer::Static, SpriteLayer::Adjust];

/// Icon item rơi của V1 (SCOPE_V1 §1).
const ITEMS: &[&str] = &["Bomb", "Fire", "Roller"];

/// Một entry pack cần trích thành clip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipSource {
    /// ID ổn định của clip, ví dụ `c00.walk.down.static`.
    pub id: String,
    /// State trình diễn mà clip phục vụ.
    pub state: PresentationState,
    /// Pack chứa entry.
    pub pack: String,
    /// Đường dẫn entry trong pack.
    pub entry: String,
    /// Lớp sprite; `None` cho clip không chia lớp.
    pub layer: Option<SpriteLayer>,
    /// Nguồn bằng chứng (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng.
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    pub claims: Vec<String>,
    /// Thời lượng frame của clip.
    pub timing: FrameTiming,
    /// Ghi chú: lý do, giới hạn, ứng viên.
    pub note: Option<String>,
}

/// Khai báo một clip cần trích; dựng thành [`ClipSource`] bằng [`ClipDecl::build`].
struct ClipDecl {
    id: String,
    pack: &'static str,
    entry: String,
    layer: Option<SpriteLayer>,
    state: PresentationState,
    status: Status,
    claim: &'static str,
    note: &'static str,
}

impl ClipDecl {
    fn build(self) -> ClipSource {
        ClipSource {
            id: self.id,
            state: self.state,
            pack: self.pack.to_owned(),
            entry: self.entry,
            layer: self.layer,
            source: Source::KrStatic,
            status: self.status,
            claims: vec![self.claim.to_owned()],
            timing: FrameTiming::Unknown {
                reason: NO_TIMING_SOURCE.to_owned(),
            },
            note: Some(self.note.to_owned()),
        }
    }
}

/// Bảng entry animation cần có cho state trình diễn của V1.
///
/// Bảng này là **dữ liệu content** (RULES_V1 R-077/R-078/R-084), không phải luật chơi: state
/// nào dùng clip nào là quyết định chọn nội dung của V1, còn frame/pivot đọc từ pack.
///
/// Trạng thái bằng chứng theo từng nhóm:
///
/// - `walk` (4 hướng), `dead`, bom đếm fuse, icon item và hiệu ứng nổ: `confirmed` —
///   tên entry khớp state và dãi frame đọc được trong pack KR 00967.
/// - `idle` (`Blink`/`BlinkSmall`), `start`, `trapped` (`FaintLayer` + bong bóng) và
///   lửa: `candidate` — entry có trong pack nhưng **chưa có consumer nào xác nhận**
///   state nào dùng nó (Q-003); cùng mức với manifest closure (issue #34).
#[must_use]
pub fn v1_clip_sources() -> Vec<ClipSource> {
    let mut out = Vec::new();
    for character in CHARACTERS {
        let id_prefix = character.to_lowercase();
        for direction in DIRECTIONS {
            for layer in LAYERS {
                out.push(
                    ClipDecl {
                        id: format!(
                            "{id_prefix}.walk.{}.{}",
                            direction.as_str().to_lowercase(),
                            layer.as_str().to_lowercase()
                        ),
                        pack: CHARACTER_PACK,
                        entry: format!(
                            "Character/{character}-Walk{}_ALayer{}",
                            direction.as_str(),
                            layer.as_str()
                        ),
                        layer: Some(*layer),
                        state: PresentationState::Walk(*direction),
                        status: Status::Confirmed,
                        claim: CLAIM_CONFIRMED,
                        note: "entry `Walk<Dir>_ALayer<Lớp>` của nhân vật V1; 5 frame, xác nhận \
                               theo tên entry và dãi frame đọc được trong pack KR 00967",
                    }
                    .build(),
                );
            }
        }
        for layer in LAYERS {
            let layer_name = layer.as_str().to_lowercase();
            out.push(
                ClipDecl {
                    id: format!("{id_prefix}.idle.blink.{layer_name}"),
                    pack: CHARACTER_PACK,
                    entry: format!("Character/{character}-Blink_ALayer{}", layer.as_str()),
                    layer: Some(*layer),
                    state: PresentationState::Idle,
                    status: Status::Candidate,
                    claim: CLAIM_CANDIDATE,
                    note: "ứng viên clip đứng yên (nháy mắt): chưa có consumer nào xác nhận \
                           state idle dùng clip này (Q-003)",
                }
                .build(),
            );
            out.push(
                ClipDecl {
                    id: format!("{id_prefix}.idle.blink_small.{layer_name}"),
                    pack: CHARACTER_PACK,
                    entry: format!("Character/{character}-BlinkSmall_ALayer{}", layer.as_str()),
                    layer: Some(*layer),
                    state: PresentationState::Idle,
                    status: Status::Candidate,
                    claim: CLAIM_CANDIDATE,
                    note: "ứng viên clip đứng yên (nháy mắt, biến thể nhỏ): chưa có consumer nào \
                           xác nhận state idle dùng clip này (Q-003)",
                }
                .build(),
            );
            out.push(
                ClipDecl {
                    id: format!("{id_prefix}.start.{layer_name}"),
                    pack: CHARACTER_PACK,
                    entry: format!("Character/{character}-Start_ALayer{}", layer.as_str()),
                    layer: Some(*layer),
                    state: PresentationState::Start,
                    status: Status::Candidate,
                    claim: CLAIM_CANDIDATE,
                    note: "ứng viên clip bắt đầu: chưa có consumer nào xác nhận state start của \
                           V1 dùng clip này (Q-003)",
                }
                .build(),
            );
            out.push(
                ClipDecl {
                    id: format!("{id_prefix}.trapped.{layer_name}"),
                    pack: CHARACTER_PACK,
                    entry: format!("Character/{character}-FaintLayer{}", layer.as_str()),
                    layer: Some(*layer),
                    state: PresentationState::Trapped,
                    status: Status::Candidate,
                    claim: CLAIM_CANDIDATE,
                    note: "ứng viên clip bị nhốt: chưa có consumer nào xác nhận state trapped dùng \
                           clip này (Q-003)",
                }
                .build(),
            );
            out.push(
                ClipDecl {
                    id: format!("{id_prefix}.dead.{layer_name}"),
                    pack: CHARACTER_PACK,
                    entry: format!("Character/{character}-DieLayer{}", layer.as_str()),
                    layer: Some(*layer),
                    state: PresentationState::Dead,
                    status: Status::Confirmed,
                    claim: CLAIM_CONFIRMED,
                    note: "entry `DieLayer<Lớp>` của nhân vật V1; 14 frame (khung cuối trỏ ảnh \
                           0×0 — dữ liệu thật của client gốc)",
                }
                .build(),
            );
        }
    }

    out.push(
        ClipDecl {
            id: "bomb.fuse".to_owned(),
            pack: BOMB_PACK,
            entry: "Bomb/Bomb00".to_owned(),
            layer: None,
            state: PresentationState::BombFuse,
            status: Status::Confirmed,
            claim: CLAIM_CONFIRMED,
            note: "hình bom đặt trên ô; 4 frame là 4 mức đếm fuse, còn tần suất đổi mức thì \
                   chưa có nguồn",
        }
        .build(),
    );
    out.push(
        ClipDecl {
            id: "trapped.bubble".to_owned(),
            pack: BOMB_PACK,
            entry: "Bomb/Bubble00".to_owned(),
            layer: None,
            state: PresentationState::Trapped,
            status: Status::Candidate,
            claim: CLAIM_CANDIDATE,
            note: "bong bóng bị nhốt; 9 frame là chuỗi bong bóng to lên rồi vỡ, chưa có consumer \
                   nào xác nhận (Q-003)",
        }
        .build(),
    );
    out.push(
        ClipDecl {
            id: "flame.main".to_owned(),
            pack: BOMB_PACK,
            entry: "Bomb/Fire00".to_owned(),
            layer: None,
            state: PresentationState::Flame(FlameKind::Main),
            status: Status::Candidate,
            claim: CLAIM_CANDIDATE,
            note: "hình lửa: **một** blob 100 frame gộp nhiều phần; cách chia tâm/thân/đuôi và \
                   thứ tự hướng chưa xác nhận được (Q-003)",
        }
        .build(),
    );
    for direction in DIRECTIONS {
        out.push(
            ClipDecl {
                id: format!("flame.motion_blur.{}", direction.as_str().to_lowercase()),
                pack: FX_PACK,
                entry: format!("MotionBlur/Blur_Fire01_{}", direction.as_str()),
                layer: None,
                state: PresentationState::Flame(FlameKind::MotionBlur(*direction)),
                status: Status::Candidate,
                claim: CLAIM_CANDIDATE,
                note: "lửa nhạt (motion blur) theo hướng: bằng chứng client phân biệt hướng lửa, \
                       nhưng chưa có consumer nào xác nhận state nào vẽ clip này (Q-003)",
            }
            .build(),
        );
    }
    out.push(
        ClipDecl {
            id: "explosion.main".to_owned(),
            pack: BOMB_PACK,
            entry: "Bomb/BombExplosion001".to_owned(),
            layer: None,
            state: PresentationState::Explosion,
            status: Status::Confirmed,
            claim: CLAIM_CONFIRMED,
            note: "hiệu ứng nổ: **một** clip 10 frame liên tục, pack không tách tâm/nhánh/đuôi \
                   nên không có clip riêng theo phần (R-084)",
        }
        .build(),
    );
    for item in ITEMS {
        out.push(
            ClipDecl {
                id: format!("item.{}", item.to_lowercase()),
                pack: FX_PACK,
                entry: format!("Item/{item}"),
                layer: None,
                state: PresentationState::Item,
                status: Status::Confirmed,
                claim: CLAIM_CONFIRMED,
                note: "icon item rơi trên nền; 1 frame",
            }
            .build(),
        );
    }
    out
}

/// Lỗi khi trích clip animation từ pack.
#[derive(Debug)]
pub enum ExtractError {
    /// Lỗi đọc pack.
    Pack(PackError),
    /// Lỗi parse blob sprite.
    Sprite(SpriteError),
    /// Clip không hợp lệ (chỉ số ảnh, rect, thời lượng).
    Animation(AnimationError),
    /// Hai clip trùng ID.
    DuplicateId(String),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pack(e) => write!(f, "{e}"),
            Self::Sprite(e) => write!(f, "{e}"),
            Self::Animation(e) => write!(f, "{e}"),
            Self::DuplicateId(id) => write!(f, "ID clip {id} bị trùng"),
        }
    }
}

impl std::error::Error for ExtractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pack(e) => Some(e),
            Self::Sprite(e) => Some(e),
            Self::Animation(e) => Some(e),
            Self::DuplicateId(_) => None,
        }
    }
}

impl From<PackError> for ExtractError {
    fn from(value: PackError) -> Self {
        Self::Pack(value)
    }
}

impl From<SpriteError> for ExtractError {
    fn from(value: SpriteError) -> Self {
        Self::Sprite(value)
    }
}

impl From<AnimationError> for ExtractError {
    fn from(value: AnimationError) -> Self {
        Self::Animation(value)
    }
}

/// Trích clip từ pack theo bảng entry.
///
/// Mỗi pack chỉ được mở một lần (theo nhóm entry) để không giữ nhiều `.idd` cùng lúc.
/// Kết quả giữ **thứ tự của `sources`**.
///
/// # Errors
///
/// Trả [`ExtractError`] khi đọc pack, parse sprite, hoặc khi clip không hợp lệ
/// (chỉ số ảnh ngoài miền, rect nằm ngoài ảnh, thời lượng không hợp lệ).
pub fn extract_clips(
    pack_dir: &Path,
    build: Build,
    sources: &[ClipSource],
) -> Result<AnimationSet, ExtractError> {
    let mut by_pack: BTreeMap<&str, Vec<&ClipSource>> = BTreeMap::new();
    for source in sources {
        by_pack
            .entry(source.pack.as_str())
            .or_default()
            .push(source);
    }

    let mut extracted: BTreeMap<&str, AnimationClip> = BTreeMap::new();
    for (pack_name, group) in by_pack {
        let pack = Pack::open(pack_dir, pack_name, build)?;
        for source in group {
            if extracted.contains_key(source.id.as_str()) {
                return Err(ExtractError::DuplicateId(source.id.clone()));
            }
            let clip = read_clip(&pack, source)?;
            extracted.insert(source.id.as_str(), clip);
        }
    }

    let clips = sources
        .iter()
        .filter_map(|source| extracted.get(source.id.as_str()).cloned())
        .collect();
    Ok(AnimationSet { clips })
}

/// Đọc một entry trong pack và dựng clip.
fn read_clip(pack: &Pack, source: &ClipSource) -> Result<AnimationClip, ExtractError> {
    let blob = pack.read_named(&source.entry)?;
    let parsed = sprite::parse(&blob)?;
    let images: Vec<ImageSize> = parsed
        .images
        .iter()
        .map(|image| ImageSize {
            width: image.width,
            height: image.height,
        })
        .collect();
    let mut frames = Vec::with_capacity(parsed.frames.len());
    for (index, frame) in parsed.frames.iter().enumerate() {
        frames.push(convert_frame(&source.id, index, frame)?);
    }

    let clip = AnimationClip {
        id: source.id.clone(),
        state: source.state,
        pack: source.pack.clone(),
        entry: source.entry.clone(),
        layer: source.layer,
        sprite_unk: parsed.unk,
        frames,
        timing: source.timing.clone(),
        source: source.source,
        status: source.status,
        claims: source.claims.clone(),
        note: source.note.clone(),
    };
    validate_frames(&clip, &images)?;
    clip.timing.validate(&clip.id)?;
    Ok(clip)
}

/// Đổi một frame của blob sang frame của schema.
fn convert_frame(
    clip: &str,
    index: usize,
    frame: &sprite::Frame,
) -> Result<AnimationFrame, ExtractError> {
    let image_index =
        u32::try_from(frame.image_index).map_err(|_| AnimationError::NegativeImageIndex {
            clip: clip.to_owned(),
            frame: index,
            index: frame.image_index,
        })?;
    Ok(AnimationFrame {
        image_index,
        source: SourceRect {
            x0: frame.source.x0,
            y0: frame.source.y0,
            x1: frame.source.x1,
            y1: frame.source.y1,
        },
        canvas: CanvasSize {
            width: frame.canvas.width,
            height: frame.canvas.height,
        },
        pivot: Pivot {
            x: frame.pivot.x,
            y: frame.pivot.y,
        },
        offset_x: frame.offset_x,
        offset_y: frame.offset_y,
        flag: frame.flag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thư mục chứa pack KR thật; test bỏ qua khi chưa đặt `BOOM_KR_PACK_DIR`.
    fn pack_root() -> Option<std::path::PathBuf> {
        let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
        Some(std::path::PathBuf::from(path))
    }

    #[test]
    fn v1_table_covers_every_state_and_has_unique_ids() {
        let sources = v1_clip_sources();
        let mut ids: Vec<&str> = sources.iter().map(|source| source.id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "ID clip phải là duy nhất");

        for character in CHARACTERS {
            let prefix = character.to_lowercase();
            for direction in DIRECTIONS {
                let id = format!("{prefix}.walk.{}.static", direction.as_str().to_lowercase());
                assert!(
                    sources.iter().any(|source| source.id == id),
                    "thiếu clip {id}"
                );
            }
            for suffix in [
                "idle.blink.static",
                "idle.blink_small.static",
                "start.static",
                "trapped.static",
                "dead.static",
            ] {
                let id = format!("{prefix}.{suffix}");
                assert!(
                    sources.iter().any(|source| source.id == id),
                    "thiếu clip {id}"
                );
            }
        }
        for id in [
            "bomb.fuse",
            "trapped.bubble",
            "flame.main",
            "flame.motion_blur.up",
            "flame.motion_blur.down",
            "flame.motion_blur.left",
            "flame.motion_blur.right",
            "item.bomb",
            "item.fire",
            "item.roller",
            "explosion.main",
        ] {
            assert!(
                sources.iter().any(|source| source.id == id),
                "thiếu clip {id}"
            );
        }
    }

    #[test]
    fn v1_table_entry_paths_match_pack_naming() {
        let sources = v1_clip_sources();
        let entry_of = |id: &str| {
            sources
                .iter()
                .find(|source| source.id == id)
                .map(|source| source.entry.clone())
                .unwrap_or_default()
        };
        assert_eq!(
            entry_of("c00.walk.down.static"),
            "Character/C00-WalkDown_ALayerStatic"
        );
        assert_eq!(
            entry_of("c01.walk.up.adjust"),
            "Character/C01-WalkUp_ALayerAdjust"
        );
        assert_eq!(
            entry_of("c00.idle.blink.static"),
            "Character/C00-Blink_ALayerStatic"
        );
        assert_eq!(
            entry_of("c00.idle.blink_small.static"),
            "Character/C00-BlinkSmall_ALayerStatic"
        );
        assert_eq!(
            entry_of("c00.start.static"),
            "Character/C00-Start_ALayerStatic"
        );
        assert_eq!(
            entry_of("c00.trapped.static"),
            "Character/C00-FaintLayerStatic"
        );
        assert_eq!(entry_of("c00.dead.static"), "Character/C00-DieLayerStatic");
        assert_eq!(entry_of("bomb.fuse"), "Bomb/Bomb00");
        assert_eq!(entry_of("trapped.bubble"), "Bomb/Bubble00");
        assert_eq!(entry_of("flame.main"), "Bomb/Fire00");
        assert_eq!(
            entry_of("flame.motion_blur.left"),
            "MotionBlur/Blur_Fire01_Left"
        );
        assert_eq!(entry_of("item.roller"), "Item/Roller");
        assert_eq!(entry_of("explosion.main"), "Bomb/BombExplosion001");
    }

    /// Không clip nào được mang thời lượng đoán mò: chưa có nguồn thì là `unknown`.
    #[test]
    fn no_clip_carries_a_guessed_frame_duration() {
        for source in v1_clip_sources() {
            assert_eq!(
                source.timing.ms(),
                None,
                "clip {} không được mang thời lượng khi chưa có nguồn",
                source.id
            );
            assert!(matches!(source.timing, FrameTiming::Unknown { .. }));
        }
    }

    /// Nghiệm thu trên pack KR thật: mọi state V1 có entry và dãi frame hợp lệ.
    #[test]
    fn every_v1_state_has_entry_and_valid_frames() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let sources = v1_clip_sources();
        let set = extract_clips(&root, Build::Kr, &sources).expect("trích clip V1");
        assert_eq!(set.clips.len(), sources.len());
        set.validate().expect("tập clip phải hợp lệ");

        for clip in &set.clips {
            assert!(!clip.frames.is_empty(), "{} không có frame", clip.id);
            for frame in &clip.frames {
                // Pivot phải nằm trong canvas (quan sát trên dữ liệu V1: pivot ở đáy).
                assert!(
                    frame.pivot.x >= 0 && frame.pivot.x < frame.canvas.width,
                    "{}: pivot X {} nằm ngoài canvas {}",
                    clip.id,
                    frame.pivot.x,
                    frame.canvas.width
                );
                assert!(
                    frame.pivot.y >= 0 && frame.pivot.y < frame.canvas.height,
                    "{}: pivot Y {} nằm ngoài canvas {}",
                    clip.id,
                    frame.pivot.y,
                    frame.canvas.height
                );
            }
        }

        // Số frame đọc được trong pack KR 00967 (khớp `boom-assets sprite-info`).
        let frames_of = |id: &str| {
            set.get(id)
                .unwrap_or_else(|| panic!("thiếu clip {id}"))
                .frame_count()
        };
        assert_eq!(frames_of("c00.walk.down.static"), 5);
        assert_eq!(frames_of("c01.walk.left.adjust"), 5);
        assert_eq!(frames_of("c00.idle.blink.static"), 3);
        assert_eq!(frames_of("c00.idle.blink_small.static"), 3);
        assert_eq!(frames_of("c00.start.static"), 10);
        assert_eq!(frames_of("c00.trapped.static"), 3);
        assert_eq!(frames_of("c00.dead.static"), 14);
        assert_eq!(frames_of("bomb.fuse"), 4);
        assert_eq!(frames_of("trapped.bubble"), 9);
        assert_eq!(frames_of("flame.main"), 100);
        assert_eq!(frames_of("flame.motion_blur.up"), 10);
        assert_eq!(frames_of("item.bomb"), 1);
        assert_eq!(frames_of("item.fire"), 1);
        assert_eq!(frames_of("item.roller"), 1);
        assert_eq!(frames_of("explosion.main"), 10);
    }

    /// Clip chết có khung cuối trỏ ảnh 0×0: dữ liệu thật, không phải lỗi.
    #[test]
    fn dead_clip_ends_with_an_empty_frame() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let sources = v1_clip_sources();
        let set = extract_clips(&root, Build::Kr, &sources).expect("trích clip V1");
        let clip = set.get("c00.dead.static").expect("clip chết");
        let last = clip.frames.last().expect("clip chết có frame");
        assert_eq!(last.source.width(), 0);
        assert_eq!(last.source.height(), 0);
    }
}
