//! Loader bảng mapping content V1: `data/assets/*.toml` → dữ liệu **có nhãn bằng chứng**.
//!
//! Ba bảng mapping của V1 (animation, UI, audio) và bảng closure trước đây là khai báo
//! Rust trong `boom-assets`; nay chúng là **data** ở `data/assets/`, đúng tinh thần
//! `data/rules/*.toml` (AGENTS §3: một thông tin — một nguồn chuẩn). Đổi mapping = sửa
//! file data, không sửa code.
//!
//! Mỗi dòng mapping mang nhãn bằng chứng theo
//! [AGENTS §4](../../../../AGENTS.md#4-bằng-chứng):
//!
//! - `source`: nguồn của **dữ liệu được trỏ tới** (entry trong pack KR, hay `design` khi
//!   dự án tự quyết).
//! - `status`: mức bằng chứng — nghĩa đúng đắn của nó **ghi ở đầu mỗi file data** vì mỗi
//!   bảng trả lời một câu khác nhau (mapping state → entry, mapping event → entry, hay sự
//!   tồn tại của asset).
//! - `claims`: claim RULES_V1 của dòng; `note`/`reason`: lý do, giới hạn, ứng viên.
//!
//! Loader **từ chối** dữ liệu sai thay vì đoán (AGENTS §4):
//!
//! - ID trùng trong một bảng;
//! - dòng thiếu trường bắt buộc (entry, state, lý do…);
//! - state/hướng không hợp lệ;
//! - thời lượng `unknown` mang giá trị, `known` thiếu claim, hoặc thời lượng mang nhãn
//!   bằng chứng sai (nhãn của thời lượng được kiểm y hệt nhãn của một dòng mapping);
//! - nhãn sai (nguồn `design` mang nhãn `confirmed`, `confirmed` mà nguồn không phải
//!   client gốc, dòng mapping mang nhãn `unknown`…).

use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;

use crate::animation::{FrameTiming, PresentationState, SpriteLayer, status_name};
use crate::bundle::{UiRole, UiScreen, UiState};
use crate::rules::{Source, Status};

/// Schema hiện hành của các bảng mapping trong `data/assets/`.
pub const SCHEMA: u32 = 1;

// --------------------------------------------------------------------------------------
// Nhãn bằng chứng
// --------------------------------------------------------------------------------------

/// Kiểm nhãn bằng chứng của một dòng mapping (AGENTS §4).
///
/// Quy tắc:
///
/// 1. `design` là quyết định của dự án ⇒ nhãn phải là `design`, không phải `confirmed`.
/// 2. `confirmed` cần bằng chứng đọc được từ client gốc (KR/VN) — nguồn `web`/`oss`/
///    `emu_choice` chỉ được là `candidate` hoặc `design` cho tới khi được đối chiếu.
/// 3. `unknown` không mang giá trị nên dòng mapping không được mang nhãn này.
/// 4. Claim phải có dạng `R-<số>`; dòng bắt buộc có claim thì không được rỗng.
///
/// # Errors
///
/// Trả [`AssetError`] khi nhãn sai.
fn validate_evidence(
    kind: &'static str,
    id: &str,
    source: Source,
    status: Status,
    claims: &[String],
    need_claims: bool,
) -> Result<(), AssetError> {
    match (source, status) {
        (Source::Design, Status::Confirmed | Status::Candidate) => {
            return Err(AssetError::BadLabel {
                kind,
                id: id.to_owned(),
                message: "nguồn `design` không được mang nhãn `confirmed`/`candidate`".to_owned(),
            });
        }
        (Source::Web | Source::Oss | Source::EmuChoice, status)
            if !matches!(status, Status::Candidate | Status::Design) =>
        {
            return Err(AssetError::BadLabel {
                kind,
                id: id.to_owned(),
                message: format!(
                    "nguồn `{}` chỉ được là `candidate` hoặc `design`",
                    source_name(source)
                ),
            });
        }
        (_, Status::Confirmed)
            if !matches!(
                source,
                Source::KrStatic | Source::KrRuntime | Source::VnStatic | Source::VnRuntime
            ) =>
        {
            return Err(AssetError::BadLabel {
                kind,
                id: id.to_owned(),
                message: format!(
                    "nhãn `confirmed` cần nguồn đọc được từ client gốc (kr_*/vn_*), đang là `{}`",
                    source_name(source)
                ),
            });
        }
        (_, Status::Unknown) => {
            return Err(AssetError::BadLabel {
                kind,
                id: id.to_owned(),
                message:
                    "dòng mapping không được mang nhãn `unknown` (chưa xác định thì không ghi dòng)"
                        .to_owned(),
            });
        }
        _ => {}
    }
    if need_claims && claims.is_empty() {
        return Err(AssetError::MissingClaims {
            kind,
            id: id.to_owned(),
        });
    }
    for claim in claims {
        if !is_claim_id(claim) {
            return Err(AssetError::BadClaim {
                kind,
                id: id.to_owned(),
                claim: claim.clone(),
            });
        }
    }
    Ok(())
}

/// Claim RULES_V1 có dạng `R-<số>` (ít nhất hai chữ số).
fn is_claim_id(claim: &str) -> bool {
    let Some(rest) = claim.strip_prefix("R-") else {
        return false;
    };
    rest.len() >= 2 && rest.bytes().all(|byte| byte.is_ascii_digit())
}

/// Tên nguồn bằng chứng (để in ra log/lỗi).
#[must_use]
pub fn source_name(source: Source) -> &'static str {
    match source {
        Source::KrStatic => "kr_static",
        Source::KrRuntime => "kr_runtime",
        Source::VnStatic => "vn_static",
        Source::VnRuntime => "vn_runtime",
        Source::Web => "web",
        Source::Oss => "oss",
        Source::EmuChoice => "emu_choice",
        Source::Design => "design",
    }
}

/// In nhãn trạng thái bằng chứng — dùng khi CLI in bảng mapping.
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(status_name(*self))
    }
}

// --------------------------------------------------------------------------------------
// Thời lượng frame
// --------------------------------------------------------------------------------------

/// Thời lượng frame khi **parse**: nhánh `unknown` không được mang giá trị, nhánh `known`
/// bắt buộc có claim (kể cả khi nguồn là `design` — AGENTS §4).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum TimingSpec {
    /// Có giá trị kèm nguồn và claim.
    Known {
        /// Thời lượng một frame (ms).
        ms: u32,
        /// Nguồn bằng chứng.
        source: Source,
        /// Trạng thái bằng chứng.
        status: Status,
        /// Claim RULES_V1; bắt buộc có ít nhất một claim.
        #[serde(default)]
        claims: Vec<String>,
    },
    /// Chưa có nguồn: chỉ có lý do, **không** mang giá trị.
    Unknown {
        /// Vì sao chưa có nguồn.
        reason: String,
    },
}

impl TimingSpec {
    /// Đổi sang [`FrameTiming`] của schema animation, kèm kiểm tra quy tắc.
    ///
    /// Thời lượng là **giá trị có bằng chứng** nên nhãn của nó được kiểm y hệt nhãn của một
    /// dòng mapping (AGENTS §4): `design` không được mang nhãn `confirmed`/`candidate`,
    /// `confirmed` cần nguồn đọc được từ client gốc, và `unknown` không được mang giá trị
    /// (ngược lại `FrameTiming::validate` cũng từ chối). Thời lượng `known` bắt buộc có
    /// claim — kể cả khi nguồn là `design`.
    ///
    /// `kind`/`id` là của dòng mang thời lượng: `clip` + ID clip cho thời lượng riêng,
    /// `clip` + `default_timing` cho thời lượng mặc định của bảng.
    fn into_timing(self, kind: &'static str, id: &str) -> Result<FrameTiming, AssetError> {
        match self {
            Self::Known {
                ms,
                source,
                status,
                claims,
            } => {
                // Nhãn của thời lượng: dùng chung validator với dòng mapping. `need_claims`
                // để false vì phần thiếu claim báo bằng `BadTiming` (thông điệp riêng của
                // thời lượng) bên dưới.
                validate_evidence(kind, id, source, status, &claims, false)?;
                if ms == 0 {
                    return Err(AssetError::BadTiming {
                        kind,
                        id: id.to_owned(),
                        message: "thời lượng `known` phải khác 0".to_owned(),
                    });
                }
                if claims.is_empty() {
                    return Err(AssetError::BadTiming {
                        kind,
                        id: id.to_owned(),
                        message: "thời lượng `known` phải có ít nhất một claim".to_owned(),
                    });
                }
                Ok(FrameTiming::Known {
                    ms,
                    source,
                    status,
                    claims,
                })
            }
            Self::Unknown { reason } => {
                if reason.trim().is_empty() {
                    return Err(AssetError::BadTiming {
                        kind,
                        id: id.to_owned(),
                        message: "thời lượng `unknown` phải ghi lý do".to_owned(),
                    });
                }
                Ok(FrameTiming::Unknown { reason })
            }
        }
    }
}

// --------------------------------------------------------------------------------------
// Bảng animation
// --------------------------------------------------------------------------------------

/// Một dòng của bảng clip animation: state trình diễn → entry pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationMapping {
    /// ID ổn định của clip, ví dụ `c00.walk.down.static`.
    pub id: String,
    /// State trình diễn mà clip phục vụ.
    pub state: PresentationState,
    /// Tên pack chứa entry.
    pub pack: String,
    /// Đường dẫn entry trong pack.
    pub entry: String,
    /// Lớp sprite; `None` cho clip không chia lớp.
    pub layer: Option<SpriteLayer>,
    /// Thời lượng một frame.
    pub timing: FrameTiming,
    /// Nguồn bằng chứng của entry được trỏ tới (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng của mapping (AGENTS §4).
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn, ứng viên.
    pub note: Option<String>,
    /// Khối bảng animation ODT của client gốc chứa chuỗi phát, ví dụ `Character/C00_A`.
    pub odt: Option<String>,
    /// State cần lấy chuỗi phát trong khối [`AnimationMapping::odt`], ví dụ `Walk Left`.
    pub odt_states: Vec<String>,
}

/// Dòng bảng animation khi parse (thời lượng chưa resolve về [`FrameTiming`]).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnimationMapping {
    id: String,
    state: String,
    #[serde(default)]
    pack: Option<String>,
    #[serde(default)]
    entry: Option<String>,
    #[serde(default)]
    layer: Option<SpriteLayer>,
    #[serde(default)]
    timing: Option<TimingSpec>,
    source: Source,
    status: Status,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    odt: Option<String>,
    #[serde(default)]
    odt_states: Vec<String>,
}

/// Bảng clip animation của V1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationTable {
    /// Schema của bảng.
    pub schema: u32,
    /// Các clip theo thứ tự ổn định (thứ tự này quyết định thứ tự atlas của bundle).
    pub clips: Vec<AnimationMapping>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnimationTable {
    schema: u32,
    #[serde(default)]
    default_timing: Option<TimingSpec>,
    #[serde(default)]
    clips: Vec<RawAnimationMapping>,
}

impl AnimationTable {
    /// Nạp bảng clip từ TOML.
    ///
    /// # Errors
    ///
    /// Trả [`AssetError`] khi schema sai, ID trùng, dòng thiếu trường, state/hướng
    /// không hợp lệ, thời lượng sai quy tắc hoặc nhãn bằng chứng sai.
    pub fn from_toml(text: &str) -> Result<Self, AssetError> {
        let raw: RawAnimationTable = toml::from_str(text).map_err(|e| AssetError::Parse {
            file: "data/assets/animation-v1.toml",
            message: e.to_string(),
        })?;
        if raw.schema != SCHEMA {
            return Err(AssetError::Schema {
                file: "data/assets/animation-v1.toml",
                found: raw.schema,
            });
        }
        let default_timing = raw
            .default_timing
            .map(|spec| spec.into_timing("clip", "default_timing"))
            .transpose()?;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut clips = Vec::with_capacity(raw.clips.len());
        for row in raw.clips {
            if row.id.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "clip",
                    id: "(không có)".to_owned(),
                    field: "id",
                });
            }
            if !seen.insert(row.id.clone()) {
                return Err(AssetError::DuplicateId {
                    kind: "clip",
                    id: row.id.clone(),
                });
            }
            let pack = require_text(&row.pack, "clip", &row.id, "pack")?;
            let entry = require_text(&row.entry, "clip", &row.id, "entry")?;
            let state = parse_state(&row.state)?;
            validate_evidence("clip", &row.id, row.source, row.status, &row.claims, true)?;
            let timing = match row.timing {
                Some(spec) => spec.into_timing("clip", &row.id)?,
                None => default_timing
                    .clone()
                    .ok_or_else(|| AssetError::MissingField {
                        kind: "clip",
                        id: row.id.clone(),
                        field: "timing",
                    })?,
            };
            // Chuỗi ODT: khối và danh sách state phải đi cùng nhau (ADR 0014 mục 3).
            if row.odt.is_some() != !row.odt_states.is_empty() {
                return Err(AssetError::MissingField {
                    kind: "clip",
                    id: row.id.clone(),
                    field: if row.odt.is_some() {
                        "odt_states"
                    } else {
                        "odt"
                    },
                });
            }
            clips.push(AnimationMapping {
                id: row.id,
                state,
                pack,
                entry,
                layer: row.layer,
                timing,
                source: row.source,
                status: row.status,
                claims: row.claims,
                note: row.note,
                odt: row.odt,
                odt_states: row.odt_states,
            });
        }
        Ok(Self {
            schema: raw.schema,
            clips,
        })
    }
}

/// Trường bắt buộc phải có và không rỗng.
fn require_text(
    value: &Option<String>,
    kind: &'static str,
    id: &str,
    field: &'static str,
) -> Result<String, AssetError> {
    match value {
        Some(text) if !text.trim().is_empty() => Ok(text.clone()),
        _ => Err(AssetError::MissingField {
            kind,
            id: id.to_owned(),
            field,
        }),
    }
}

/// Đổi khoá state sang [`PresentationState`]; sai thì báo lỗi kèm khoá.
fn parse_state(key: &str) -> Result<PresentationState, AssetError> {
    use crate::animation::FlameKind;
    use PresentationState::{BombFuse, Dead, Explosion, Flame, Idle, Item, Start, Trapped, Walk};

    if let Some(direction) = key.strip_prefix("walk_") {
        return Ok(Walk(parse_direction(direction)?));
    }
    if let Some(direction) = key.strip_prefix("flame_motion_blur_") {
        return Ok(Flame(FlameKind::MotionBlur(parse_direction(direction)?)));
    }
    match key {
        "idle" => Ok(Idle),
        "start" => Ok(Start),
        "trapped" => Ok(Trapped),
        "dead" => Ok(Dead),
        "bomb_fuse" => Ok(BombFuse),
        "flame_main" => Ok(Flame(FlameKind::Main)),
        "explosion" => Ok(Explosion),
        "item" => Ok(Item),
        other => Err(AssetError::InvalidState {
            value: other.to_owned(),
        }),
    }
}

/// Đổi tên hướng sang [`Direction`]; sai thì báo lỗi kèm tên.
fn parse_direction(name: &str) -> Result<crate::animation::Direction, AssetError> {
    use crate::animation::Direction;
    match name {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        "left" => Ok(Direction::Left),
        "right" => Ok(Direction::Right),
        other => Err(AssetError::InvalidDirection {
            value: other.to_owned(),
        }),
    }
}

// --------------------------------------------------------------------------------------
// Bảng UI
// --------------------------------------------------------------------------------------

/// Một dòng của bảng asset UI: phần tử của màn hình → entry pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiMapping {
    /// ID ổn định của phần tử UI, ví dụ `lobby.create`.
    pub id: String,
    /// Màn hình của luồng V1 dùng phần tử này.
    pub screen: UiScreen,
    /// Vai trò của phần tử.
    pub role: UiRole,
    /// Entry pack của asset KR; `None` khi V1 không dùng asset nào cho phần tử này.
    pub entry: Option<String>,
    /// Trạng thái dùng (không phải nhãn bằng chứng).
    pub state: UiState,
    /// Lý do/ghi chú — bắt buộc, không rỗng.
    pub reason: String,
    /// Nguồn bằng chứng của asset được trỏ tới (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng của asset (AGENTS §4).
    pub status: Status,
    /// Claim RULES_V1 liên quan (UI không phải luật chơi nên thường rỗng).
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn, ứng viên.
    pub note: Option<String>,
}

/// Dòng bảng UI khi parse.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUiMapping {
    id: String,
    screen: UiScreen,
    role: UiRole,
    #[serde(default)]
    entry: Option<String>,
    state: UiState,
    source: Source,
    status: Status,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    note: Option<String>,
    reason: String,
}

/// Bảng asset UI của luồng V1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiTable {
    /// Schema của bảng.
    pub schema: u32,
    /// Các phần tử theo thứ tự ổn định.
    pub elements: Vec<UiMapping>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUiTable {
    schema: u32,
    #[serde(default)]
    elements: Vec<RawUiMapping>,
}

impl UiTable {
    /// Nạp bảng UI từ TOML.
    ///
    /// # Errors
    ///
    /// Trả [`AssetError`] khi schema sai, ID trùng, phần tử thiếu trường bắt buộc, trạng
    /// thái dùng không khớp entry, hoặc nhãn bằng chứng sai.
    pub fn from_toml(text: &str) -> Result<Self, AssetError> {
        let raw: RawUiTable = toml::from_str(text).map_err(|e| AssetError::Parse {
            file: "data/assets/ui-v1.toml",
            message: e.to_string(),
        })?;
        if raw.schema != SCHEMA {
            return Err(AssetError::Schema {
                file: "data/assets/ui-v1.toml",
                found: raw.schema,
            });
        }
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut elements = Vec::with_capacity(raw.elements.len());
        for row in raw.elements {
            if row.id.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "ui",
                    id: "(không có)".to_owned(),
                    field: "id",
                });
            }
            if !seen.insert(row.id.clone()) {
                return Err(AssetError::DuplicateId {
                    kind: "ui",
                    id: row.id.clone(),
                });
            }
            if row.reason.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "ui",
                    id: row.id.clone(),
                    field: "reason",
                });
            }
            // Phần tử dùng asset thì phải có entry; phần tử không dùng asset thì `excluded`.
            if row.state != UiState::Excluded && row.entry.is_none() {
                return Err(AssetError::MissingField {
                    kind: "ui",
                    id: row.id.clone(),
                    field: "entry",
                });
            }
            if row
                .entry
                .as_ref()
                .is_some_and(|entry| entry.trim().is_empty())
            {
                return Err(AssetError::MissingField {
                    kind: "ui",
                    id: row.id.clone(),
                    field: "entry",
                });
            }
            validate_evidence("ui", &row.id, row.source, row.status, &row.claims, false)?;
            elements.push(UiMapping {
                id: row.id,
                screen: row.screen,
                role: row.role,
                entry: row.entry,
                state: row.state,
                reason: row.reason,
                source: row.source,
                status: row.status,
                claims: row.claims,
                note: row.note,
            });
        }
        Ok(Self {
            schema: raw.schema,
            elements,
        })
    }
}

// --------------------------------------------------------------------------------------
// Bảng audio
// --------------------------------------------------------------------------------------

/// Một dòng của bảng audio: event → entry trong pack `FxSound`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioMapping {
    /// ID cue theo AUDIO_V1, ví dụ `bgm_loop`.
    pub event: String,
    /// Entry nguồn trong pack `FxSound`.
    pub entry: String,
    /// Nguồn bằng chứng của entry được trỏ tới (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng của mapping event → entry (AGENTS §4).
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn, ứng viên.
    pub note: Option<String>,
}

/// Dòng bảng audio khi parse.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAudioMapping {
    event: String,
    #[serde(default)]
    entry: Option<String>,
    source: Source,
    status: Status,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

/// Bảng mapping âm thanh V1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioTable {
    /// Schema của bảng.
    pub schema: u32,
    /// Các cue theo thứ tự ổn định.
    pub cues: Vec<AudioMapping>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAudioTable {
    schema: u32,
    #[serde(default)]
    cues: Vec<RawAudioMapping>,
}

impl AudioTable {
    /// Nạp bảng audio từ TOML.
    ///
    /// # Errors
    ///
    /// Trả [`AssetError`] khi schema sai, event trùng, dòng thiếu entry, entry không thuộc
    /// `FxSound`, hoặc nhãn bằng chứng sai.
    pub fn from_toml(text: &str) -> Result<Self, AssetError> {
        let raw: RawAudioTable = toml::from_str(text).map_err(|e| AssetError::Parse {
            file: "data/assets/audio-v1.toml",
            message: e.to_string(),
        })?;
        if raw.schema != SCHEMA {
            return Err(AssetError::Schema {
                file: "data/assets/audio-v1.toml",
                found: raw.schema,
            });
        }
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut cues = Vec::with_capacity(raw.cues.len());
        for row in raw.cues {
            if row.event.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "cue",
                    id: "(không có)".to_owned(),
                    field: "event",
                });
            }
            if !seen.insert(row.event.clone()) {
                return Err(AssetError::DuplicateId {
                    kind: "cue",
                    id: row.event.clone(),
                });
            }
            let entry = require_text(&row.entry, "cue", &row.event, "entry")?;
            // Pack âm thanh của client gốc chỉ có hai thư mục entry.
            if !(entry.starts_with("MP3/") || entry.starts_with("Wave/")) {
                return Err(AssetError::BadEntry {
                    kind: "cue",
                    id: row.event.clone(),
                    entry: entry.clone(),
                });
            }
            validate_evidence("cue", &row.event, row.source, row.status, &row.claims, true)?;
            cues.push(AudioMapping {
                event: row.event,
                entry,
                source: row.source,
                status: row.status,
                claims: row.claims,
                note: row.note,
            });
        }
        Ok(Self {
            schema: raw.schema,
            cues,
        })
    }
}

// --------------------------------------------------------------------------------------
// Bảng closure
// --------------------------------------------------------------------------------------

/// Vai trò của một entry trong closure của một trận V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureRole {
    /// Dữ liệu map `.map` của trận.
    Map,
    /// Theme của map (ảnh chọn map trong lobby).
    Theme,
    /// Preview minimap của map.
    Preview,
    /// Nhạc nền của map.
    Music,
    /// Sprite nền của ô (ID tile trong map).
    Tile,
    /// Sprite vật cản của ô (ID obj trong map).
    Object,
    /// Sprite đi của nhân vật (4 hướng).
    CharacterWalk,
    /// Sprite đứng yên của nhân vật.
    CharacterIdle,
    /// Sprite bị nhốt của nhân vật.
    CharacterTrapped,
    /// Sprite chết của nhân vật.
    CharacterDead,
    /// Hình quả bom đặt trên ô.
    BombArt,
    /// Hình lửa/nước lan trên ô.
    FlameArt,
    /// Hình vụ nổ.
    ExplosionArt,
    /// Bong bóng của người bị nhốt.
    TrappedArt,
    /// Icon item rơi (Bomb/Fire/Roller).
    ItemIcon,
    /// Bảng chỉ số nhân vật.
    CharacterAttr,
    /// Bảng chỉ số item.
    ItemAttr,
}

impl ClosureRole {
    /// Tên vai trò dùng trong manifest.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Theme => "theme",
            Self::Preview => "preview",
            Self::Music => "music",
            Self::Tile => "tile",
            Self::Object => "object",
            Self::CharacterWalk => "character_walk",
            Self::CharacterIdle => "character_idle",
            Self::CharacterTrapped => "character_trapped",
            Self::CharacterDead => "character_dead",
            Self::BombArt => "bomb_art",
            Self::FlameArt => "flame_art",
            Self::ExplosionArt => "explosion_art",
            Self::TrappedArt => "trapped_art",
            Self::ItemIcon => "item_icon",
            Self::CharacterAttr => "character_attr",
            Self::ItemAttr => "item_attr",
        }
    }
}

/// Một dòng của bảng closure: entry cố định của một trận V1 + vai trò + nhãn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureEntryMapping {
    /// Đường dẫn entry đầy đủ, ví dụ `fx/Item/Bomb`.
    pub entry: String,
    /// Vai trò của entry trong closure.
    pub role: ClosureRole,
    /// Nguồn bằng chứng của entry được trỏ tới (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng của vai trò (AGENTS §4).
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn.
    pub note: Option<String>,
}

/// Một họ entry sprite nhân vật mà V1 khai trong closure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterFamilyMapping {
    /// Họ entry — **tiền tố** entry trong ODT (`Character/<Id>-<họ>`); entry pack đầy đủ là
    /// `Character/<Id>-<họ>Layer<lớp>` vì client ghep thêm lớp `Static`/`Adjust` (R-094).
    pub family: String,
    /// Vai trò của entry trong closure.
    pub role: ClosureRole,
    /// V1 **dùng** họ này ⇒ closure kiểm nó có trong bảng state của client (ODT).
    pub used_by_v1: bool,
    /// Nguồn bằng chứng của entry được trỏ tới (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng của vai trò (AGENTS §4).
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn.
    pub note: Option<String>,
}

/// Mapping state của V1 → state của client (ODT) → họ entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterStateMapping {
    /// State trình diễn của V1, ví dụ `idle`.
    pub v1_state: String,
    /// State của client trong bảng ODT, ví dụ `Blink`.
    pub client_state: String,
    /// Họ entry mà V1 chọn cho state đó.
    pub family: String,
    /// Nguồn bằng chứng của mapping (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng của mapping (AGENTS §4).
    pub status: Status,
    /// Claim RULES_V1 liên quan.
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn.
    pub note: Option<String>,
}

/// Bảng closure của một trận V1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureTable {
    /// Schema của bảng.
    pub schema: u32,
    /// Entry cố định của một trận V1, theo thứ tự ổn định.
    pub entries: Vec<ClosureEntryMapping>,
    /// Họ entry sprite nhân vật.
    pub character_families: Vec<CharacterFamilyMapping>,
    /// Mapping state V1 → state của client.
    pub character_states: Vec<CharacterStateMapping>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClosureEntryMapping {
    entry: String,
    role: ClosureRole,
    source: Source,
    status: Status,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCharacterFamilyMapping {
    family: String,
    role: ClosureRole,
    #[serde(default)]
    used_by_v1: bool,
    source: Source,
    status: Status,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCharacterStateMapping {
    v1_state: String,
    client_state: String,
    family: String,
    source: Source,
    status: Status,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClosureTable {
    schema: u32,
    #[serde(default)]
    entries: Vec<RawClosureEntryMapping>,
    #[serde(default)]
    character_families: Vec<RawCharacterFamilyMapping>,
    #[serde(default)]
    character_states: Vec<RawCharacterStateMapping>,
}

impl ClosureTable {
    /// Nạp bảng closure từ TOML.
    ///
    /// # Errors
    ///
    /// Trả [`AssetError`] khi schema sai, ID trùng, dòng thiếu trường bắt buộc, hoặc nhãn
    /// bằng chứng sai.
    pub fn from_toml(text: &str) -> Result<Self, AssetError> {
        let raw: RawClosureTable = toml::from_str(text).map_err(|e| AssetError::Parse {
            file: "data/assets/closure-v1.toml",
            message: e.to_string(),
        })?;
        if raw.schema != SCHEMA {
            return Err(AssetError::Schema {
                file: "data/assets/closure-v1.toml",
                found: raw.schema,
            });
        }
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut entries = Vec::with_capacity(raw.entries.len());
        for row in raw.entries {
            if row.entry.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "closure",
                    id: "(không có)".to_owned(),
                    field: "entry",
                });
            }
            if !seen.insert(row.entry.clone()) {
                return Err(AssetError::DuplicateId {
                    kind: "closure",
                    id: row.entry.clone(),
                });
            }
            validate_evidence(
                "closure",
                &row.entry,
                row.source,
                row.status,
                &row.claims,
                true,
            )?;
            entries.push(ClosureEntryMapping {
                entry: row.entry,
                role: row.role,
                source: row.source,
                status: row.status,
                claims: row.claims,
                note: row.note,
            });
        }

        let mut seen_families: BTreeSet<String> = BTreeSet::new();
        let mut character_families = Vec::with_capacity(raw.character_families.len());
        for row in raw.character_families {
            if row.family.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "character_family",
                    id: "(không có)".to_owned(),
                    field: "family",
                });
            }
            if !seen_families.insert(row.family.clone()) {
                return Err(AssetError::DuplicateId {
                    kind: "character_family",
                    id: row.family.clone(),
                });
            }
            validate_evidence(
                "character_family",
                &row.family,
                row.source,
                row.status,
                &row.claims,
                true,
            )?;
            character_families.push(CharacterFamilyMapping {
                family: row.family,
                role: row.role,
                used_by_v1: row.used_by_v1,
                source: row.source,
                status: row.status,
                claims: row.claims,
                note: row.note,
            });
        }

        let mut seen_states: BTreeSet<String> = BTreeSet::new();
        let mut character_states = Vec::with_capacity(raw.character_states.len());
        for row in raw.character_states {
            if row.v1_state.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "character_state",
                    id: "(không có)".to_owned(),
                    field: "v1_state",
                });
            }
            if !seen_states.insert(row.v1_state.clone()) {
                return Err(AssetError::DuplicateId {
                    kind: "character_state",
                    id: row.v1_state.clone(),
                });
            }
            if row.client_state.trim().is_empty() || row.family.trim().is_empty() {
                return Err(AssetError::MissingField {
                    kind: "character_state",
                    id: row.v1_state.clone(),
                    field: "client_state/family",
                });
            }
            validate_evidence(
                "character_state",
                &row.v1_state,
                row.source,
                row.status,
                &row.claims,
                true,
            )?;
            character_states.push(CharacterStateMapping {
                v1_state: row.v1_state,
                client_state: row.client_state,
                family: row.family,
                source: row.source,
                status: row.status,
                claims: row.claims,
                note: row.note,
            });
        }

        Ok(Self {
            schema: raw.schema,
            entries,
            character_families,
            character_states,
        })
    }
}

// --------------------------------------------------------------------------------------
// Tổng bảng
// --------------------------------------------------------------------------------------

/// Bốn bảng mapping content của V1, nạp từ `data/assets/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetTables {
    /// Bảng clip animation.
    pub animation: AnimationTable,
    /// Bảng asset UI.
    pub ui: UiTable,
    /// Bảng mapping âm thanh.
    pub audio: AudioTable,
    /// Bảng closure của một trận V1.
    pub closure: ClosureTable,
}

impl AssetTables {
    /// Nạp bốn bảng từ nội dung TOML.
    ///
    /// # Errors
    ///
    /// Trả lỗi của bảng đầu tiên không hợp lệ.
    pub fn from_toml(
        animation: &str,
        ui: &str,
        audio: &str,
        closure: &str,
    ) -> Result<Self, AssetError> {
        Ok(Self {
            animation: AnimationTable::from_toml(animation)?,
            ui: UiTable::from_toml(ui)?,
            audio: AudioTable::from_toml(audio)?,
            closure: ClosureTable::from_toml(closure)?,
        })
    }
}

/// Nội dung bảng clip animation V1 (nhúng từ `data/assets/`).
pub const ANIMATION_V1: &str = include_str!("../../../data/assets/animation-v1.toml");

/// Nội dung bảng asset UI V1 (nhúng từ `data/assets/`).
pub const UI_V1: &str = include_str!("../../../data/assets/ui-v1.toml");

/// Nội dung bảng mapping âm thanh V1 (nhúng từ `data/assets/`).
pub const AUDIO_V1: &str = include_str!("../../../data/assets/audio-v1.toml");

/// Nội dung bảng closure V1 (nhúng từ `data/assets/`).
pub const CLOSURE_V1: &str = include_str!("../../../data/assets/closure-v1.toml");

impl AssetTables {
    /// Bốn bảng V1 nhúng trong crate.
    ///
    /// # Errors
    ///
    /// Trả [`AssetError`] nếu dữ liệu nhúng sai quy tắc (test trong crate chặn việc này).
    pub fn v1() -> Result<Self, AssetError> {
        Self::from_toml(ANIMATION_V1, UI_V1, AUDIO_V1, CLOSURE_V1)
    }
}

// --------------------------------------------------------------------------------------
// Lỗi
// --------------------------------------------------------------------------------------

/// Lỗi khi nạp bảng mapping content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    /// File không parse được.
    Parse {
        /// File nguồn.
        file: &'static str,
        /// Thông điệp lỗi gốc.
        message: String,
    },
    /// Schema của file không đúng.
    Schema {
        /// File nguồn.
        file: &'static str,
        /// Schema đọc được.
        found: u32,
    },
    /// ID trùng trong một bảng.
    DuplicateId {
        /// Loại dòng (`clip`, `ui`, `cue`, …).
        kind: &'static str,
        /// ID bị trùng.
        id: String,
    },
    /// Dòng thiếu trường bắt buộc.
    MissingField {
        /// Loại dòng.
        kind: &'static str,
        /// ID của dòng.
        id: String,
        /// Tên trường thiếu.
        field: &'static str,
    },
    /// State trình diễn không hợp lệ.
    InvalidState {
        /// Khoá state đọc được.
        value: String,
    },
    /// Hướng không hợp lệ.
    InvalidDirection {
        /// Tên hướng đọc được.
        value: String,
    },
    /// Thời lượng frame vi phạm quy tắc.
    BadTiming {
        /// Loại dòng.
        kind: &'static str,
        /// ID của dòng.
        id: String,
        /// Lý do.
        message: String,
    },
    /// Nhãn bằng chứng sai.
    BadLabel {
        /// Loại dòng.
        kind: &'static str,
        /// ID của dòng.
        id: String,
        /// Lý do.
        message: String,
    },
    /// Dòng bắt buộc có claim nhưng không có.
    MissingClaims {
        /// Loại dòng.
        kind: &'static str,
        /// ID của dòng.
        id: String,
    },
    /// Claim không có dạng `R-<số>`.
    BadClaim {
        /// Loại dòng.
        kind: &'static str,
        /// ID của dòng.
        id: String,
        /// Claim sai.
        claim: String,
    },
    /// Entry không thuộc pack được phép.
    BadEntry {
        /// Loại dòng.
        kind: &'static str,
        /// ID của dòng.
        id: String,
        /// Entry sai.
        entry: String,
    },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { file, message } => write!(f, "{file}: không đọc được: {message}"),
            Self::Schema { file, found } => {
                write!(f, "{file}: schema {found} không hỗ trợ (cần {SCHEMA})")
            }
            Self::DuplicateId { kind, id } => write!(f, "{kind}: ID {id} bị trùng"),
            Self::MissingField { kind, id, field } => {
                write!(f, "{kind} {id}: thiếu trường {field}")
            }
            Self::InvalidState { value } => write!(f, "state trình diễn không hợp lệ: {value}"),
            Self::InvalidDirection { value } => write!(f, "hướng không hợp lệ: {value}"),
            Self::BadTiming { kind, id, message } => write!(f, "{kind} {id}: {message}"),
            Self::BadLabel { kind, id, message } => write!(f, "{kind} {id}: {message}"),
            Self::MissingClaims { kind, id } => {
                write!(f, "{kind} {id}: dòng bắt buộc phải có claim RULES_V1")
            }
            Self::BadClaim { kind, id, claim } => {
                write!(f, "{kind} {id}: claim {claim:?} không có dạng `R-<số>`")
            }
            Self::BadEntry { kind, id, entry } => {
                write!(f, "{kind} {id}: entry {entry:?} không thuộc pack âm thanh")
            }
        }
    }
}

impl std::error::Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bảng clip tối thiểu hợp lệ (dùng làm gốc cho các ca lỗi).
    const CLIP: &str = r#"
schema = 1
default_timing = { unknown = { reason = "chưa có nguồn thời lượng" } }

[[clips]]
id = "c00.walk.up.static"
state = "walk_up"
pack = "FxCharacter"
entry = "Character/C00-WalkUp_ALayerStatic"
layer = "static"
source = "kr_static"
status = "confirmed"
claims = ["R-077"]
"#;

    /// Bảng UI tối thiểu hợp lệ.
    const UI: &str = r#"
schema = 1

[[elements]]
id = "lobby.create"
screen = "lobby"
role = "button"
entry = "FxStage/Lobby/LB_Bt_T_Create-NM"
state = "needs_review_text"
source = "kr_static"
status = "confirmed"
claims = []
reason = "ảnh nút có chữ Hàn"
"#;

    /// Bảng audio tối thiểu hợp lệ.
    const AUDIO: &str = r#"
schema = 1

[[cues]]
event = "bomb_placed"
entry = "Wave/Cha_BombIgnite"
source = "kr_static"
status = "design"
claims = ["R-082"]
"#;

    /// Bảng closure tối thiểu hợp lệ.
    const CLOSURE: &str = r#"
schema = 1

[[entries]]
entry = "fx/Item/Bomb"
role = "item_icon"
source = "kr_static"
status = "confirmed"
claims = ["R-077"]
"#;

    /// Dữ liệu trong repo phải nạp được và giữ đúng số dòng đã đo.
    #[test]
    fn embedded_tables_load_with_the_recorded_shape() {
        let tables = AssetTables::v1().expect("nạp bảng trong data/assets/");
        assert_eq!(tables.animation.clips.len(), 48, "bảng clip V1");
        assert_eq!(tables.ui.elements.len(), 23, "bảng UI V1");
        assert_eq!(tables.audio.cues.len(), 15, "bảng audio V1");
        assert_eq!(
            tables.closure.entries.len(),
            11,
            "entry cố định của closure"
        );
        assert_eq!(
            tables.closure.character_families.len(),
            8,
            "họ entry nhân vật"
        );
        assert_eq!(
            tables.closure.character_states.len(),
            2,
            "mapping state V1 → client"
        );

        // ID trong mỗi bảng là duy nhất.
        let mut ids: Vec<&str> = tables
            .animation
            .clips
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "ID clip phải là duy nhất");
    }

    /// Không clip nào được mang thời lượng đoán mò: chưa có nguồn thì là `unknown`.
    #[test]
    fn no_clip_carries_a_guessed_frame_duration() {
        let tables = AssetTables::v1().expect("nạp bảng trong data/assets/");
        for clip in &tables.animation.clips {
            assert_eq!(
                clip.timing.ms(),
                None,
                "clip {} không được mang thời lượng khi chưa có nguồn",
                clip.id
            );
        }
    }

    /// Nhãn bằng chứng của clip phải giữ nguyên như trước khi chuyển sang data.
    #[test]
    fn clip_evidence_labels_are_unchanged() {
        let tables = AssetTables::v1().expect("nạp bảng trong data/assets/");
        let status_of = |id: &str| {
            tables
                .animation
                .clips
                .iter()
                .find(|clip| clip.id == id)
                .map(|clip| clip.status)
        };
        assert_eq!(status_of("c00.walk.down.static"), Some(Status::Confirmed));
        assert_eq!(status_of("c00.idle.blink.static"), Some(Status::Candidate));
        assert_eq!(status_of("explosion.main"), Some(Status::Confirmed));
        assert_eq!(status_of("flame.main"), Some(Status::Candidate));
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let text = format!(
            "{CLIP}\n[[clips]]\nid = \"c00.walk.up.static\"\nstate = \"idle\"\npack = \"FxCharacter\"\nentry = \"Character/C00-Blink_ALayerStatic\"\nsource = \"kr_static\"\nstatus = \"confirmed\"\nclaims = [\"R-077\"]\n"
        );
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::DuplicateId { kind: "clip", .. })
        ));

        let text = format!(
            "{UI}\n[[elements]]\nid = \"lobby.create\"\nscreen = \"lobby\"\nrole = \"button\"\nstate = \"excluded\"\nsource = \"design\"\nstatus = \"design\"\nclaims = []\nreason = \"lặp\"\n"
        );
        assert!(matches!(
            UiTable::from_toml(&text),
            Err(AssetError::DuplicateId { kind: "ui", .. })
        ));

        let text = format!(
            "{AUDIO}\n[[cues]]\nevent = \"bomb_placed\"\nentry = \"Wave/Cha_Die\"\nsource = \"kr_static\"\nstatus = \"design\"\nclaims = [\"R-082\"]\n"
        );
        assert!(matches!(
            AudioTable::from_toml(&text),
            Err(AssetError::DuplicateId { kind: "cue", .. })
        ));
    }

    #[test]
    fn missing_entry_is_rejected() {
        let text = CLIP.replace("entry = \"Character/C00-WalkUp_ALayerStatic\"", "");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::MissingField {
                kind: "clip",
                field: "entry",
                ..
            })
        ));

        let text = AUDIO.replace("entry = \"Wave/Cha_BombIgnite\"", "");
        assert!(matches!(
            AudioTable::from_toml(&text),
            Err(AssetError::MissingField {
                kind: "cue",
                field: "entry",
                ..
            })
        ));

        // Phần tử UI dùng asset thì phải có entry.
        let text = UI.replace("entry = \"FxStage/Lobby/LB_Bt_T_Create-NM\"", "");
        assert!(matches!(
            UiTable::from_toml(&text),
            Err(AssetError::MissingField {
                kind: "ui",
                field: "entry",
                ..
            })
        ));
    }

    #[test]
    fn invalid_state_or_direction_is_rejected() {
        let text = CLIP.replace("state = \"walk_up\"", "state = \"flying\"");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::InvalidState { .. })
        ));

        let text = CLIP.replace("state = \"walk_up\"", "state = \"walk_sideways\"");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::InvalidDirection { .. })
        ));

        let text = CLIP.replace(
            "state = \"walk_up\"",
            "state = \"flame_motion_blur_middle\"",
        );
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::InvalidDirection { .. })
        ));
    }

    /// `unknown` không được mang giá trị; `known` bắt buộc có claim.
    #[test]
    fn timing_unknown_must_not_carry_a_value() {
        let text = CLIP.replace(
            "default_timing = { unknown = { reason = \"chưa có nguồn thời lượng\" } }",
            "default_timing = { unknown = { reason = \"chưa có nguồn thời lượng\", ms = 100 } }",
        );
        let error = AnimationTable::from_toml(&text).expect_err("phải từ chối");
        assert!(matches!(error, AssetError::Parse { .. }), "lỗi lạ: {error}");
    }

    #[test]
    fn timing_known_needs_claims_and_a_positive_duration() {
        let text = CLIP.replace(
            "claims = [\"R-077\"]",
            "claims = [\"R-077\"]\ntiming = { known = { ms = 100, source = \"design\", status = \"design\" } }",
        );
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadTiming { .. })
        ));

        let text = CLIP.replace(
            "claims = [\"R-077\"]",
            "claims = [\"R-077\"]\ntiming = { known = { ms = 0, source = \"design\", status = \"design\", claims = [\"R-085\"] } }",
        );
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadTiming { .. })
        ));
    }

    /// Nhãn bằng chứng của **thời lượng** cũng phải đúng: `design` không được mang nhãn
    /// `confirmed`, `confirmed` cần nguồn đọc được từ client gốc, `unknown` không được
    /// mang giá trị (AGENTS §4 — review Codex #139).
    #[test]
    fn timing_known_label_is_checked() {
        // Nhãn sai trong thời lượng riêng của clip.
        for (source, status) in [
            ("design", "confirmed"),
            ("web", "confirmed"),
            ("kr_static", "unknown"),
        ] {
            let text = CLIP.replace(
                "claims = [\"R-077\"]",
                &format!(
                    "claims = [\"R-077\"]\ntiming = {{ known = {{ ms = 100, source = \"{source}\", \
                     status = \"{status}\", claims = [\"R-085\"] }} }}"
                ),
            );
            assert!(
                matches!(
                    AnimationTable::from_toml(&text),
                    Err(AssetError::BadLabel { kind: "clip", .. })
                ),
                "thời lượng mang nhãn {source}/{status} phải bị từ chối"
            );
        }

        // Nhãn sai trong `default_timing` cũng bị từ chối, và báo đúng dòng.
        let text = CLIP.replace(
            "default_timing = { unknown = { reason = \"chưa có nguồn thời lượng\" } }",
            "default_timing = { known = { ms = 100, source = \"web\", status = \"confirmed\", \
             claims = [\"R-085\"] } }",
        );
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadLabel {
                kind: "clip",
                ref id,
                ..
            }) if id == "default_timing"
        ));

        // Nhãn đúng thì thời lượng được nhận: nguồn client gốc + `confirmed`, và `design`
        // thì phải mang nhãn `design` (design thuần không được nâng thành `confirmed`).
        let text = CLIP.replace(
            "claims = [\"R-077\"]",
            "claims = [\"R-077\"]\ntiming = { known = { ms = 100, source = \"kr_static\", \
             status = \"confirmed\", claims = [\"R-085\"] } }",
        );
        let table = AnimationTable::from_toml(&text).expect("thời lượng nhãn đúng phải nạp được");
        let clip = &table.clips[0];
        assert_eq!(
            clip.timing.ms(),
            Some(100),
            "thời lượng hợp lệ phải mang giá trị"
        );

        let text = CLIP.replace(
            "claims = [\"R-077\"]",
            "claims = [\"R-077\"]\ntiming = { known = { ms = 100, source = \"design\", \
             status = \"design\", claims = [\"R-085\"] } }",
        );
        assert!(
            AnimationTable::from_toml(&text).is_ok(),
            "thời lượng design/design kèm claim phải hợp lệ"
        );
    }

    /// Nhãn bằng chứng sai bị từ chối (nhãn `confirmed` cần nguồn đọc được từ client gốc).
    #[test]
    fn wrong_evidence_label_is_rejected() {
        let text = CLIP.replace("source = \"kr_static\"", "source = \"design\"");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadLabel { .. })
        ));

        let text = CLIP.replace("source = \"kr_static\"", "source = \"web\"");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadLabel { .. })
        ));

        let text = CLIP.replace("status = \"confirmed\"", "status = \"unknown\"");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadLabel { .. })
        ));
    }

    /// Claim phải có dạng `R-<số>`; dòng bắt buộc có claim thì không được rỗng.
    #[test]
    fn claim_id_is_checked() {
        let text = CLIP.replace("claims = [\"R-077\"]", "claims = [\"R77\"]");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::BadClaim { .. })
        ));

        let text = CLIP.replace("claims = [\"R-077\"]", "claims = []");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::MissingClaims { .. })
        ));
    }

    /// Khoá lạ trong một dòng bị từ chối (gõ sai tên trường không được lọt).
    #[test]
    fn unknown_field_in_a_row_is_rejected() {
        let text = CLIP.replace(
            "status = \"confirmed\"",
            "status = \"confirmed\"\nsourse = \"kr_static\"",
        );
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::Parse { .. })
        ));
    }

    #[test]
    fn wrong_schema_is_rejected() {
        let text = CLIP.replace("schema = 1", "schema = 2");
        assert!(matches!(
            AnimationTable::from_toml(&text),
            Err(AssetError::Schema { found: 2, .. })
        ));
    }

    /// Entry âm thanh phải thuộc pack `FxSound` (`MP3/` hoặc `Wave/`).
    #[test]
    fn audio_entry_outside_the_sound_pack_is_rejected() {
        let text = AUDIO.replace("Wave/Cha_BombIgnite", "Sprite/C00");
        assert!(matches!(
            AudioTable::from_toml(&text),
            Err(AssetError::BadEntry { .. })
        ));
    }

    /// Bảng closure: entry trùng, họ trùng và dòng thiếu claim đều bị từ chối.
    #[test]
    fn closure_table_is_checked() {
        let text = format!(
            "{CLOSURE}\n[[entries]]\nentry = \"fx/Item/Bomb\"\nrole = \"item_icon\"\nsource = \"kr_static\"\nstatus = \"confirmed\"\nclaims = [\"R-077\"]\n"
        );
        assert!(matches!(
            ClosureTable::from_toml(&text),
            Err(AssetError::DuplicateId {
                kind: "closure",
                ..
            })
        ));

        let text = CLOSURE.replace("claims = [\"R-077\"]", "claims = []");
        assert!(matches!(
            ClosureTable::from_toml(&text),
            Err(AssetError::MissingClaims { .. })
        ));

        // Vai trò lạ bị từ chối khi parse.
        let text = CLOSURE.replace("role = \"item_icon\"", "role = \"khong_co\"");
        assert!(matches!(
            ClosureTable::from_toml(&text),
            Err(AssetError::Parse { .. })
        ));
    }
}
