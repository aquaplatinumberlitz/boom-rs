//! Schema animation V1: clip, frame, pivot và thời lượng frame.
//!
//! Kiểu ở đây là hợp đồng dữ liệu dùng chung cho client, công cụ và bộ trích trong
//! `boom-assets`. Mọi giá trị luật — kể cả **thời lượng frame** — đều mang
//! nguồn/trạng thái/claim như rule profile ([`crate::rules`]); kiểu này không chứa
//! hằng số luật chơi.
//!
//! **Thời lượng frame luôn phải có claim**, kể cả khi nguồn là `design`: rule profile
//! chấp nhận ngoại lệ `design` thuần (rule kiến trúc không cần claim), nhưng thời
//! lượng là quyết định nội dung của V1 nên phải truy được claim dẫn tới — theo
//! AGENTS §4, `design` là "dự án chọn, **có claim dẫn tới**". Vì chưa có nguồn cho
//! thời lượng (R-079) nên mọi clip đang dùng [`FrameTiming::Unknown`] chứ không tự
//! đặt số.
//!
//! Nguồn định dạng frame 48 byte: [MAPS §3](../../../../docs/specification/assets/MAPS.md).
//! Blob `CDataI16Alpha` v2 **không mang trường thời gian** (`f8`–`f11` là canvas và
//! pivot, không phải thời lượng), nên thời lượng frame chỉ có được khi đọc metadata
//! hoặc consumer của client gốc. Chưa có nguồn thì dùng [`FrameTiming::Unknown`]
//! chứ không tự đặt mặc định.
//!
//! Clip nào ứng với state nào là **dữ liệu content của V1** (RULES_V1), không phải
//! luật chơi; bảng ánh xạ nằm ở `boom-assets::animation`.

use serde::Deserialize;

use crate::rules::{Source, Status};

/// Hướng trong V1: hướng đi của nhân vật và hướng lửa.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Lên trên.
    Up,
    /// Xuống dưới.
    Down,
    /// Sang trái.
    Left,
    /// Sang phải.
    Right,
}

impl Direction {
    /// Tên hướng như trong tên entry của client gốc (`WalkUp`, `WalkDown`, ...).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

/// Lớp sprite của nhân vật: lớp nền (`Static`) và lớp ngoại hình điều chỉnh
/// (`Adjust`).
///
/// Client gốc vẽ hai lớp này chồng nhau; **thứ tự và cách blend chưa có consumer
/// đã xác nhận** (Q-003), nên schema chỉ ghi lớp nào là lớp nào.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpriteLayer {
    /// Lớp nền.
    Static,
    /// Lớp ngoại hình điều chỉnh.
    Adjust,
}

impl SpriteLayer {
    /// Tên lớp như trong tên entry của client gốc.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Static => "Static",
            Self::Adjust => "Adjust",
        }
    }
}

/// Dạng hình lửa trong V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlameKind {
    /// Lửa chính của ô: entry `FxDesktop/Bomb/Fire00`.
    ///
    /// Blob này là **một** chuỗi 100 frame; cách chia thành tâm/thân/đuôi và thứ tự
    /// hướng **chưa được xác nhận** (không có consumer, Q-003) — xem
    /// [`AnimationClip::note`].
    Main,
    /// Lửa nhạt (motion blur) theo hướng: entry `fx/MotionBlur/Blur_Fire01_*`.
    MotionBlur(Direction),
}

/// State trình diễn của V1 mà một clip phục vụ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationState {
    /// Nhân vật đi theo hướng.
    Walk(Direction),
    /// Nhân vật đứng yên (nháy mắt).
    Idle,
    /// Nhân vật bắt đầu lượt.
    Start,
    /// Nhân vật bị nhốt trong bong bóng.
    Trapped,
    /// Nhân vật chết.
    Dead,
    /// Bom đặt trên ô (đếm fuse).
    BombFuse,
    /// Lửa trên ô.
    Flame(FlameKind),
    /// Icon item rơi trên nền.
    Item,
}

/// Thời lượng một frame của clip.
///
/// Blob `CDataI16Alpha` v2 không có trường thời gian (MAPS §3), nên thời lượng chỉ
/// có được khi đọc metadata hoặc consumer của client gốc. Khi chưa có nguồn thì
/// dùng [`FrameTiming::Unknown`] — engine không tự đặt mặc định.
///
/// Nhánh [`FrameTiming::Known`] bắt buộc có ít nhất một claim, **kể cả khi nguồn là
/// `design`**: ngoại lệ `design` thuần của rule profile không áp cho thời lượng
/// frame.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameTiming {
    /// Có giá trị kèm nguồn và claim rõ ràng.
    Known {
        /// Thời lượng một frame (ms).
        ms: u32,
        /// Nguồn bằng chứng (AGENTS §4).
        source: Source,
        /// Trạng thái bằng chứng.
        status: Status,
        /// Claim ID trong RULES_V1; bắt buộc có ít nhất một claim, kể cả khi nguồn
        /// là `design`.
        #[serde(default)]
        claims: Vec<String>,
    },
    /// Chưa có nguồn: ghi lý do, không mang giá trị.
    Unknown {
        /// Vì sao chưa có nguồn.
        reason: String,
    },
}

impl FrameTiming {
    /// Thời lượng bằng số (ms) nếu đã có nguồn; `None` nếu chưa có.
    #[must_use]
    pub fn ms(&self) -> Option<u32> {
        match self {
            Self::Known { ms, .. } => Some(*ms),
            Self::Unknown { .. } => None,
        }
    }

    /// Kiểm tra quy tắc: `unknown` không mang giá trị, **mọi** giá trị đều phải có
    /// claim — kể cả khi nguồn là `design` (khác rule profile: ngoại lệ `design` thuần
    /// không áp cho thời lượng frame) — và thời lượng phải khác 0.
    ///
    /// # Errors
    ///
    /// Trả [`AnimationError`] nếu vi phạm một trong ba quy tắc trên.
    pub fn validate(&self, clip: &str) -> Result<(), AnimationError> {
        match self {
            Self::Known {
                ms, status, claims, ..
            } => {
                if *status == Status::Unknown {
                    return Err(AnimationError::TimingUnknownWithValue {
                        clip: clip.to_owned(),
                    });
                }
                if *ms == 0 {
                    return Err(AnimationError::TimingZero {
                        clip: clip.to_owned(),
                    });
                }
                if claims.is_empty() {
                    return Err(AnimationError::TimingMissingClaims {
                        clip: clip.to_owned(),
                    });
                }
                Ok(())
            }
            Self::Unknown { .. } => Ok(()),
        }
    }
}

/// Kích thước ảnh trong blob sprite (px).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct ImageSize {
    /// Bề rộng ảnh (px).
    pub width: u32,
    /// Cao ảnh (px).
    pub height: u32,
}

/// Kích thước canvas của frame — giữ **số có dấu** nguyên như file.
///
/// Layout frame là 12 × `i32` (MAPS §3) nên `f8`/`f9` đọc bằng `i32` và không được
/// ép về số không dấu lúc parse; chuyển sang số không dấu chỉ làm ở bước diễn giải
/// có validate và bằng chứng.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct CanvasSize {
    /// Bề rộng canvas (px).
    pub width: i32,
    /// Cao canvas (px).
    pub height: i32,
}

/// Hình chữ nhật nguồn của frame trong ảnh, theo thứ tự `(x0, y0, x1, y1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct SourceRect {
    /// Cạnh trái (px).
    pub x0: i32,
    /// Cạnh trên (px).
    pub y0: i32,
    /// Cạnh phải (px).
    pub x1: i32,
    /// Cạnh dưới (px).
    pub y1: i32,
}

impl SourceRect {
    /// Bề rộng rect (px); có thể âm nếu rect bị đảo.
    #[must_use]
    pub fn width(&self) -> i32 {
        self.x1 - self.x0
    }

    /// Cao rect (px); có thể âm nếu rect bị đảo.
    #[must_use]
    pub fn height(&self) -> i32 {
        self.y1 - self.y0
    }
}

/// Điểm pivot của frame (px, trong canvas).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct Pivot {
    /// Toạ độ X của pivot.
    pub x: i32,
    /// Toạ độ Y của pivot.
    pub y: i32,
}

/// Một frame của clip animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationFrame {
    /// Chỉ số ảnh trong blob sprite (MAPS §3 `f3`, tin cậy cao).
    pub image_index: u32,
    /// Rect nguồn trong ảnh (MAPS §3 `f4`–`f7`, tin cậy cao).
    pub source: SourceRect,
    /// Kích thước canvas (MAPS §3 `f8`, `f9`).
    pub canvas: CanvasSize,
    /// Điểm pivot (MAPS §3 `f10`, `f11`).
    pub pivot: Pivot,
    /// Offset đăng ký khung hình theo trục X (MAPS §3 `f0`) — có thể âm.
    pub offset_x: i32,
    /// Offset đăng ký khung hình theo trục Y (MAPS §3 `f1`) — có thể âm.
    pub offset_y: i32,
    /// Field `f2` của frame: nghĩa chưa chốt (MAPS §3, tin cậy thấp), giữ nguyên số.
    pub flag: i32,
}

/// Một clip animation của V1: entry pack + dãi frame + thời lượng.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationClip {
    /// ID ổn định của clip, ví dụ `c00.walk.down.static`.
    pub id: String,
    /// State trình diễn mà clip phục vụ.
    pub state: PresentationState,
    /// Tên pack chứa entry, ví dụ `FxCharacter`.
    pub pack: String,
    /// Đường dẫn entry trong pack, ví dụ `Character/C00-WalkDown_ALayerStatic`.
    pub entry: String,
    /// Lớp sprite; `None` cho clip không chia lớp.
    pub layer: Option<SpriteLayer>,
    /// Dword thứ tư của header sprite — nghĩa chưa chốt, giữ nguyên số.
    pub sprite_unk: u32,
    /// Dãi frame của clip, theo thứ tự phát.
    pub frames: Vec<AnimationFrame>,
    /// Thời lượng một frame.
    pub timing: FrameTiming,
    /// Nguồn bằng chứng cho clip (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng cho clip.
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    #[serde(default)]
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn, ứng viên.
    pub note: Option<String>,
}

impl AnimationClip {
    /// Đường dẫn entry đầy đủ theo dạng `<pack>/<entry>`.
    #[must_use]
    pub fn full_entry(&self) -> String {
        format!("{}/{}", self.pack, self.entry)
    }

    /// Số frame của clip.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

/// Tập clip animation của V1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationSet {
    /// Các clip theo thứ tự ổn định.
    pub clips: Vec<AnimationClip>,
}

impl AnimationSet {
    /// Clip theo ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&AnimationClip> {
        self.clips.iter().find(|clip| clip.id == id)
    }

    /// Các clip phục vụ một state trình diễn.
    #[must_use]
    pub fn by_state(&self, state: PresentationState) -> Vec<&AnimationClip> {
        self.clips
            .iter()
            .filter(|clip| clip.state == state)
            .collect()
    }

    /// Kiểm tra toàn bộ tập: clip phải có frame và thời lượng phải hợp lệ.
    ///
    /// # Errors
    ///
    /// Trả [`AnimationError`] cho clip đầu tiên vi phạm.
    pub fn validate(&self) -> Result<(), AnimationError> {
        for clip in &self.clips {
            if clip.frames.is_empty() {
                return Err(AnimationError::EmptyFrames {
                    clip: clip.id.clone(),
                });
            }
            clip.timing.validate(&clip.id)?;
        }
        Ok(())
    }
}

/// Lỗi khi kiểm tra dữ liệu animation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimationError {
    /// Clip không có frame nào.
    EmptyFrames {
        /// ID clip.
        clip: String,
    },
    /// Chỉ số ảnh của frame nằm ngoài danh sách ảnh.
    ImageIndexOutOfRange {
        /// ID clip.
        clip: String,
        /// Chỉ số frame trong clip.
        frame: usize,
        /// Chỉ số ảnh đọc được.
        index: u32,
        /// Số ảnh của blob.
        count: usize,
    },
    /// Chỉ số ảnh của frame âm.
    NegativeImageIndex {
        /// ID clip.
        clip: String,
        /// Chỉ số frame trong clip.
        frame: usize,
        /// Chỉ số ảnh đọc được.
        index: i32,
    },
    /// Rect nguồn nằm ngoài ảnh.
    SourceOutOfImage {
        /// ID clip.
        clip: String,
        /// Chỉ số frame trong clip.
        frame: usize,
        /// Rect nguồn đọc được.
        source: SourceRect,
        /// Kích thước ảnh bị trỏ tới.
        image: ImageSize,
    },
    /// Rect nguồn rỗng trong khi ảnh có nội dung.
    SourceDegenerate {
        /// ID clip.
        clip: String,
        /// Chỉ số frame trong clip.
        frame: usize,
        /// Rect nguồn đọc được.
        source: SourceRect,
    },
    /// Kích thước canvas âm.
    NegativeCanvas {
        /// ID clip.
        clip: String,
        /// Chỉ số frame trong clip.
        frame: usize,
        /// Kích thước canvas đọc được.
        canvas: CanvasSize,
    },
    /// Thời lượng mang trạng thái `unknown` (không được mang giá trị).
    TimingUnknownWithValue {
        /// ID clip.
        clip: String,
    },
    /// Thời lượng có giá trị nhưng không có claim (kể cả khi nguồn là `design`).
    TimingMissingClaims {
        /// ID clip.
        clip: String,
    },
    /// Thời lượng bằng 0.
    TimingZero {
        /// ID clip.
        clip: String,
    },
}

impl std::fmt::Display for AnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFrames { clip } => write!(f, "{clip}: clip không có frame nào"),
            Self::ImageIndexOutOfRange {
                clip,
                frame,
                index,
                count,
            } => write!(
                f,
                "{clip}: frame {frame} trỏ ảnh {index} nhưng blob chỉ có {count} ảnh"
            ),
            Self::NegativeImageIndex { clip, frame, index } => {
                write!(f, "{clip}: frame {frame} có chỉ số ảnh âm ({index})")
            }
            Self::SourceOutOfImage {
                clip,
                frame,
                source,
                image,
            } => write!(
                f,
                "{clip}: frame {frame} rect ({},{},{},{}) nằm ngoài ảnh {}x{}",
                source.x0, source.y0, source.x1, source.y1, image.width, image.height
            ),
            Self::SourceDegenerate {
                clip,
                frame,
                source,
            } => write!(
                f,
                "{clip}: frame {frame} rect ({},{},{},{}) rỗng trong khi ảnh có nội dung",
                source.x0, source.y0, source.x1, source.y1
            ),
            Self::NegativeCanvas {
                clip,
                frame,
                canvas,
            } => write!(
                f,
                "{clip}: frame {frame} canvas {}x{} âm",
                canvas.width, canvas.height
            ),
            Self::TimingUnknownWithValue { clip } => write!(
                f,
                "{clip}: thời lượng mang trạng thái unknown (không được mang giá trị)"
            ),
            Self::TimingMissingClaims { clip } => {
                write!(f, "{clip}: thời lượng có giá trị nhưng không có claim")
            }
            Self::TimingZero { clip } => write!(f, "{clip}: thời lượng bằng 0"),
        }
    }
}

impl std::error::Error for AnimationError {}

/// Kiểm tra dãi frame của clip: chỉ số ảnh trong miền và rect nằm trong ảnh.
///
/// `images` là kích thước từng ảnh của blob, theo đúng thứ tự `image_index` trỏ tới.
/// Frame cuối của clip chết có ảnh 0×0 và rect rỗng: đó là dữ liệu thật của client
/// gốc (khung trống cuối chuỗi) nên được chấp nhận; rect rỗng trong ảnh có nội dung thì
/// mới là lỗi.
///
/// # Errors
///
/// Trả [`AnimationError`] cho frame đầu tiên vi phạm.
pub fn validate_frames(clip: &AnimationClip, images: &[ImageSize]) -> Result<(), AnimationError> {
    for (index, frame) in clip.frames.iter().enumerate() {
        let Some(image) = images.get(frame.image_index as usize) else {
            return Err(AnimationError::ImageIndexOutOfRange {
                clip: clip.id.clone(),
                frame: index,
                index: frame.image_index,
                count: images.len(),
            });
        };
        if frame.canvas.width < 0 || frame.canvas.height < 0 {
            return Err(AnimationError::NegativeCanvas {
                clip: clip.id.clone(),
                frame: index,
                canvas: frame.canvas,
            });
        }
        let source = frame.source;
        if source.x0 < 0
            || source.y0 < 0
            || source.x1 > i32::try_from(image.width).unwrap_or(i32::MAX)
            || source.y1 > i32::try_from(image.height).unwrap_or(i32::MAX)
            || source.x1 < source.x0
            || source.y1 < source.y0
        {
            return Err(AnimationError::SourceOutOfImage {
                clip: clip.id.clone(),
                frame: index,
                source,
                image: *image,
            });
        }
        if source.width() == 0 || source.height() == 0 {
            let empty_image = image.width == 0 || image.height == 0;
            if !empty_image {
                return Err(AnimationError::SourceDegenerate {
                    clip: clip.id.clone(),
                    frame: index,
                    source,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Claim ID giả cho test validator: thời lượng frame chưa có claim thật (R-079),
    /// nên test chỉ kiểm quy tắc "phải có claim", không khẳng định nội dung claim.
    const FAKE_CLAIM: &str = "R-000";

    fn frame(image_index: u32, source: SourceRect) -> AnimationFrame {
        AnimationFrame {
            image_index,
            source,
            canvas: CanvasSize {
                width: 56,
                height: 67,
            },
            pivot: Pivot { x: 8, y: 66 },
            offset_x: 0,
            offset_y: 40,
            flag: 0,
        }
    }

    fn clip(frames: Vec<AnimationFrame>) -> AnimationClip {
        AnimationClip {
            id: "c00.walk.down.static".to_owned(),
            state: PresentationState::Walk(Direction::Down),
            pack: "FxCharacter".to_owned(),
            entry: "Character/C00-WalkDown_ALayerStatic".to_owned(),
            layer: Some(SpriteLayer::Static),
            sprite_unk: 1,
            frames,
            timing: FrameTiming::Unknown {
                reason: "chưa có nguồn".to_owned(),
            },
            source: Source::KrStatic,
            status: Status::Confirmed,
            claims: vec!["R-077".to_owned()],
            note: None,
        }
    }

    #[test]
    fn direction_and_layer_names_match_pack_entries() {
        assert_eq!(Direction::Up.as_str(), "Up");
        assert_eq!(Direction::Down.as_str(), "Down");
        assert_eq!(Direction::Left.as_str(), "Left");
        assert_eq!(Direction::Right.as_str(), "Right");
        assert_eq!(SpriteLayer::Static.as_str(), "Static");
        assert_eq!(SpriteLayer::Adjust.as_str(), "Adjust");
    }

    #[test]
    fn accepts_frames_inside_image() {
        let images = [ImageSize {
            width: 34,
            height: 32,
        }];
        let clip = clip(vec![frame(
            0,
            SourceRect {
                x0: 0,
                y0: 0,
                x1: 34,
                y1: 32,
            },
        )]);
        assert_eq!(validate_frames(&clip, &images), Ok(()));
    }

    #[test]
    fn rejects_image_index_out_of_range() {
        let images = [ImageSize {
            width: 34,
            height: 32,
        }];
        let clip = clip(vec![frame(
            3,
            SourceRect {
                x0: 0,
                y0: 0,
                x1: 34,
                y1: 32,
            },
        )]);
        assert_eq!(
            validate_frames(&clip, &images),
            Err(AnimationError::ImageIndexOutOfRange {
                clip: "c00.walk.down.static".to_owned(),
                frame: 0,
                index: 3,
                count: 1,
            })
        );
    }

    #[test]
    fn rejects_rect_outside_image() {
        let images = [ImageSize {
            width: 34,
            height: 32,
        }];
        let clip = clip(vec![frame(
            0,
            SourceRect {
                x0: 0,
                y0: 0,
                x1: 40,
                y1: 32,
            },
        )]);
        assert!(matches!(
            validate_frames(&clip, &images),
            Err(AnimationError::SourceOutOfImage { .. })
        ));
    }

    #[test]
    fn accepts_empty_last_frame_of_empty_image() {
        // Dữ liệu thật: khung cuối của clip chết trỏ ảnh 0×0 với rect rỗng.
        let images = [
            ImageSize {
                width: 52,
                height: 109,
            },
            ImageSize {
                width: 0,
                height: 0,
            },
        ];
        let clip = clip(vec![
            frame(
                0,
                SourceRect {
                    x0: 0,
                    y0: 0,
                    x1: 52,
                    y1: 52,
                },
            ),
            frame(
                1,
                SourceRect {
                    x0: 0,
                    y0: 0,
                    x1: 0,
                    y1: 0,
                },
            ),
        ]);
        assert_eq!(validate_frames(&clip, &images), Ok(()));
    }

    #[test]
    fn rejects_empty_rect_in_image_with_content() {
        let images = [ImageSize {
            width: 52,
            height: 109,
        }];
        let clip = clip(vec![frame(
            0,
            SourceRect {
                x0: 0,
                y0: 0,
                x1: 0,
                y1: 0,
            },
        )]);
        assert!(matches!(
            validate_frames(&clip, &images),
            Err(AnimationError::SourceDegenerate { .. })
        ));
    }

    #[test]
    fn rejects_negative_canvas() {
        let images = [ImageSize {
            width: 34,
            height: 32,
        }];
        let mut clip = clip(vec![frame(
            0,
            SourceRect {
                x0: 0,
                y0: 0,
                x1: 34,
                y1: 32,
            },
        )]);
        clip.frames[0].canvas = CanvasSize {
            width: -1,
            height: -2,
        };
        assert!(matches!(
            validate_frames(&clip, &images),
            Err(AnimationError::NegativeCanvas { .. })
        ));
    }

    #[test]
    fn timing_without_source_carries_no_value() {
        let timing = FrameTiming::Unknown {
            reason: "chưa có nguồn".to_owned(),
        };
        assert_eq!(timing.ms(), None);
        assert_eq!(timing.validate("c00.walk.down.static"), Ok(()));
    }

    #[test]
    fn timing_with_value_needs_claim_even_when_source_is_design() {
        // Nguồn khác `design` thì chắc chắn phải có claim.
        let timing = FrameTiming::Known {
            ms: 100,
            source: Source::KrStatic,
            status: Status::Confirmed,
            claims: Vec::new(),
        };
        assert_eq!(
            timing.validate("c00.walk.down.static"),
            Err(AnimationError::TimingMissingClaims {
                clip: "c00.walk.down.static".to_owned()
            })
        );

        // Ngoại lệ `design` thuần của rule profile **không** áp cho thời lượng frame.
        let design_without_claim = FrameTiming::Known {
            ms: 100,
            source: Source::Design,
            status: Status::Design,
            claims: Vec::new(),
        };
        assert_eq!(
            design_without_claim.validate("c00.walk.down.static"),
            Err(AnimationError::TimingMissingClaims {
                clip: "c00.walk.down.static".to_owned()
            })
        );

        // Có claim thì mọi nguồn đều hợp lệ, kể cả `design`.
        let design_with_claim = FrameTiming::Known {
            ms: 100,
            source: Source::Design,
            status: Status::Design,
            claims: vec![FAKE_CLAIM.to_owned()],
        };
        assert_eq!(design_with_claim.validate("c00.walk.down.static"), Ok(()));
        assert_eq!(design_with_claim.ms(), Some(100));

        let confirmed_with_claim = FrameTiming::Known {
            ms: 100,
            source: Source::KrStatic,
            status: Status::Confirmed,
            claims: vec![FAKE_CLAIM.to_owned()],
        };
        assert_eq!(
            confirmed_with_claim.validate("c00.walk.down.static"),
            Ok(())
        );
    }

    /// Hồi quy: bundle nạp `known` thiếu claim phải bị từ chối, kể cả nguồn `design`.
    #[test]
    fn known_timing_from_bundle_without_claim_is_rejected() {
        let text = r#"
id = "c00.walk.down.static"
state = { walk = "down" }
pack = "FxCharacter"
entry = "Character/C00-WalkDown_ALayerStatic"
layer = "static"
sprite_unk = 1
timing = { known = { ms = 100, source = "design", status = "design" } }
source = "kr_static"
status = "confirmed"

[[frames]]
image_index = 0
source = { x0 = 0, y0 = 0, x1 = 34, y1 = 32 }
canvas = { width = 56, height = 67 }
pivot = { x = 8, y = 66 }
offset_x = 0
offset_y = 40
flag = 0
"#;
        let clip: AnimationClip = toml::from_str(text).expect("nạp clip từ TOML");
        assert_eq!(clip.timing.ms(), Some(100));
        assert_eq!(
            clip.timing.validate("probe"),
            Err(AnimationError::TimingMissingClaims {
                clip: "probe".to_owned()
            })
        );
        let set = AnimationSet { clips: vec![clip] };
        assert!(matches!(
            set.validate(),
            Err(AnimationError::TimingMissingClaims { .. })
        ));
    }

    #[test]
    fn timing_with_unknown_status_carries_no_value() {
        let timing = FrameTiming::Known {
            ms: 100,
            source: Source::KrStatic,
            status: Status::Unknown,
            claims: vec!["R-077".to_owned()],
        };
        assert_eq!(
            timing.validate("c00.walk.down.static"),
            Err(AnimationError::TimingUnknownWithValue {
                clip: "c00.walk.down.static".to_owned()
            })
        );
    }

    #[test]
    fn timing_zero_is_rejected() {
        let timing = FrameTiming::Known {
            ms: 0,
            source: Source::Design,
            status: Status::Design,
            claims: vec![FAKE_CLAIM.to_owned()],
        };
        assert_eq!(
            timing.validate("c00.walk.down.static"),
            Err(AnimationError::TimingZero {
                clip: "c00.walk.down.static".to_owned()
            })
        );
    }

    #[test]
    fn set_lookup_by_id_and_state() {
        let set = AnimationSet {
            clips: vec![clip(vec![frame(
                0,
                SourceRect {
                    x0: 0,
                    y0: 0,
                    x1: 34,
                    y1: 32,
                },
            )])],
        };
        assert!(set.get("c00.walk.down.static").is_some());
        assert!(set.get("c00.walk.up.static").is_none());
        assert_eq!(
            set.by_state(PresentationState::Walk(Direction::Down)).len(),
            1
        );
        assert_eq!(set.by_state(PresentationState::Dead).len(), 0);
        assert_eq!(set.validate(), Ok(()));
    }

    #[test]
    fn set_rejects_clip_without_frame() {
        let set = AnimationSet {
            clips: vec![clip(Vec::new())],
        };
        assert_eq!(
            set.validate(),
            Err(AnimationError::EmptyFrames {
                clip: "c00.walk.down.static".to_owned()
            })
        );
    }

    /// Schema phải nạp được từ bundle: enum ngoài (externally tagged) và các trường
    /// tuỳ chọn không bắt buộc phải có.
    #[test]
    fn clip_loads_from_toml() {
        let text = r#"
id = "c00.walk.down.static"
state = { walk = "down" }
pack = "FxCharacter"
entry = "Character/C00-WalkDown_ALayerStatic"
layer = "static"
sprite_unk = 1
timing = { unknown = { reason = "chưa có nguồn" } }
source = "kr_static"
status = "confirmed"

[[frames]]
image_index = 0
source = { x0 = 0, y0 = 0, x1 = 34, y1 = 32 }
canvas = { width = 56, height = 67 }
pivot = { x = 8, y = 66 }
offset_x = 0
offset_y = 40
flag = 0
"#;
        let clip: AnimationClip = toml::from_str(text).expect("nạp clip từ TOML");
        assert_eq!(clip.id, "c00.walk.down.static");
        assert_eq!(clip.state, PresentationState::Walk(Direction::Down));
        assert_eq!(clip.layer, Some(SpriteLayer::Static));
        assert_eq!(clip.frames.len(), 1);
        assert_eq!(clip.frames[0].source.width(), 34);
        assert_eq!(clip.claims, Vec::<String>::new());
        assert_eq!(clip.note, None);
        assert_eq!(clip.timing.ms(), None);
    }
}
