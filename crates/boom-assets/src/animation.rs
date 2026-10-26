//! Bộ trích animation V1 từ pack của client gốc: entry → clip (frame, pivot, thời lượng).
//!
//! Module này **không giải mã ảnh**: `crate::sprite` đã giải mã blob `CDataI16Alpha`
//! v2; ở đây chỉ ghép entry thành clip và kiểm dãi frame (chỉ số ảnh trong miền, rect
//! nằm trong ảnh) theo schema của [`boom_content::animation`].
//!
//! **Bảng mapping** (state → entry) là **data**: [`boom_content::assets`] nạp từ
//! `data/assets/animation-v1.toml`, mỗi dòng mang nhãn bằng chứng và claim RULES_V1.
//! Module này chỉ đọc pack theo bảng đó, không khai báo tên entry.
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
//! không tự đặt một hằng số thời lượng (R-079, Q-003). Lý do nằm ở bảng data, không
//! hard-code trong code.

use std::collections::BTreeMap;
use std::path::Path;

use boom_content::animation::{
    AnimationClip, AnimationError, AnimationSet, CanvasSize, ImageSize, Pivot, SourceRect,
    validate_frames,
};
use boom_content::assets::{AnimationMapping, AssetError, AssetTables};

use crate::legacy_map::Build;
use crate::pack::{Pack, PackError};
use crate::sprite::{self, SpriteError};

/// Bảng clip animation V1, nạp từ `data/assets/animation-v1.toml`.
///
/// # Errors
///
/// Trả [`AssetError`] khi bảng data sai quy tắc (dữ liệu nhúng trong crate được test
/// chặn nên trên build thường lỗi này không xảy ra).
pub fn v1_clip_sources() -> Result<Vec<AnimationMapping>, AssetError> {
    Ok(AssetTables::v1()?.animation.clips)
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
    /// Bảng mapping data sai quy tắc.
    Assets(AssetError),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pack(e) => write!(f, "{e}"),
            Self::Sprite(e) => write!(f, "{e}"),
            Self::Animation(e) => write!(f, "{e}"),
            Self::DuplicateId(id) => write!(f, "ID clip {id} bị trùng"),
            Self::Assets(e) => write!(f, "bảng mapping animation sai quy tắc: {e}"),
        }
    }
}

impl std::error::Error for ExtractError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pack(e) => Some(e),
            Self::Sprite(e) => Some(e),
            Self::Animation(e) => Some(e),
            Self::Assets(e) => Some(e),
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

impl From<AssetError> for ExtractError {
    fn from(value: AssetError) -> Self {
        Self::Assets(value)
    }
}

/// Trích clip từ pack theo bảng mapping.
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
    sources: &[AnimationMapping],
) -> Result<AnimationSet, ExtractError> {
    let mut by_pack: BTreeMap<&str, Vec<&AnimationMapping>> = BTreeMap::new();
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
fn read_clip(pack: &Pack, source: &AnimationMapping) -> Result<AnimationClip, ExtractError> {
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
) -> Result<boom_content::animation::AnimationFrame, ExtractError> {
    let image_index =
        u32::try_from(frame.image_index).map_err(|_| AnimationError::NegativeImageIndex {
            clip: clip.to_owned(),
            frame: index,
            index: frame.image_index,
        })?;
    Ok(boom_content::animation::AnimationFrame {
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
    use boom_content::animation::FrameTiming;
    use boom_content::rules::Status;

    /// Thư mục chứa pack KR thật; test bỏ qua khi chưa đặt `BOOM_KR_PACK_DIR`.
    fn pack_root() -> Option<std::path::PathBuf> {
        let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
        Some(std::path::PathBuf::from(path))
    }

    /// Bảng clip V1 từ data (nguồn chuẩn duy nhất).
    fn v1_table() -> Vec<AnimationMapping> {
        v1_clip_sources().expect("bảng clip V1 trong data/assets/ phải hợp lệ")
    }

    #[test]
    fn v1_table_covers_every_state_and_has_unique_ids() {
        let sources = v1_table();
        let mut ids: Vec<&str> = sources.iter().map(|source| source.id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "ID clip phải là duy nhất");

        // Mọi state của V1 phải có clip: đi 4 hướng, đứng yên, bắt đầu, bị nhốt, chết.
        for character in ["c00", "c01"] {
            for direction in ["up", "down", "left", "right"] {
                let id = format!("{character}.walk.{direction}.static");
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
                let id = format!("{character}.{suffix}");
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

    /// Tên entry trong bảng phải khớp quy ước đặt tên của pack KR 00967.
    #[test]
    fn v1_table_entry_paths_match_pack_naming() {
        let sources = v1_table();
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
        for source in v1_table() {
            assert_eq!(
                source.timing.ms(),
                None,
                "clip {} không được mang thời lượng khi chưa có nguồn",
                source.id
            );
            assert!(matches!(source.timing, FrameTiming::Unknown { .. }));
        }
    }

    /// Nhãn bằng chứng của clip phải tách hai tầng (R-077 `confirmed` / R-078 `candidate`).
    #[test]
    fn clip_labels_follow_the_evidence_claims() {
        for source in v1_table() {
            let expected = match source.claims.first().map(String::as_str) {
                Some("R-077") | Some("R-084") => Status::Confirmed,
                Some("R-078") => Status::Candidate,
                // Chữ HELP! (R-220/R-250): mapping candidate có nguồn riêng.
                Some("R-220") => Status::Candidate,
                other => panic!("claim không lạ trong bảng clip: {other:?}"),
            };
            assert_eq!(
                source.status, expected,
                "nhãn bằng chứng của clip {} sai",
                source.id
            );
        }
    }

    /// Nghiệm thu trên pack KR thật: mọi state V1 có entry và dãi frame hợp lệ.
    #[test]
    fn every_v1_state_has_entry_and_valid_frames() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let sources = v1_table();
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
        let sources = v1_table();
        let set = extract_clips(&root, Build::Kr, &sources).expect("trích clip V1");
        let clip = set.get("c00.dead.static").expect("clip chết");
        let last = clip.frames.last().expect("clip chết có frame");
        assert_eq!(last.source.width(), 0);
        assert_eq!(last.source.height(), 0);
    }
}
