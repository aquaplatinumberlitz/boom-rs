//! Bundle nội dung: ảnh (atlas) + metadata ô của map + clip animation + bảng UI,
//! đóng gói cho client.
//!
//! Bundle là **output cục bộ** của `boom-assets bundle` (không commit vào repo). Nó gom
//! ảnh đã giải mã từ pack KR thành các trang atlas PNG, kèm metadata ánh xạ ô của map →
//! sprite/frame, kích thước, canvas và pivot, **clip animation** của V1 (schema 2),
//! **bảng UI** của luồng V1 (schema 3) và **audio thô** (schema 4):
//! màn hình, vai trò, trạng thái dùng và ảnh. Client đọc bundle bằng kiểu ở đây, **không
//! cần bộ giải mã pack** (ARCHITECTURE §7).
//!
//! ## Phạm vi
//!
//! - Bundle chỉ mang **nội dung**: ảnh và ánh xạ id. Phân loại ô/va chạm vẫn nằm ở bảng
//!   [`crate::map::MapProfile`] (`data/maps/*.toml`) — một thông tin, một nguồn chuẩn
//!   (AGENTS §3). Bundle không nhân bản loại ô.
//! - Clip animation **không mang ảnh riêng**: ảnh của clip nằm trong sprite mà clip trỏ
//!   tới, nên mỗi entry chỉ có một bản pixel trong atlas. Bảng UI cũng vậy: ảnh của một
//!   phần tử UI nằm trong sprite mà phần tử đó trỏ tới.
//! - Thời lượng frame giữ đúng trạng thái bằng chứng: `unknown` không mang số; khi V1
//!   chốt được nhịp, giá trị `design` đi kèm claim và nằm ở profile rules — không
//!   hard-code trong bundle lẫn trong code.
//! - Bảng UI **không có trạng thái "đã duyệt"**: một ảnh có chữ chỉ được hiển thị khi
//!   người duyệt ảnh ký quyết định (quy trình [IMAGE_REVIEW]); tới lúc đó client vẽ
//!   text từ catalog. Xem [`UiState`].
//! - Định dạng bundle là quyết định **design** của dự án (ARCHITECTURE §8 "Định dạng
//!   bundle content"), không phải định dạng của client gốc. Mô tả ở
//!   `docs/specification/assets/BUNDLES.md`.
//!
//! ## Định danh nội dung
//!
//! Trường `content_hash` là SHA-256 của encoding chuẩn ([`Bundle::canonical_bytes`]):
//! số, chuỗi và danh sách theo thứ tự cố định, **không** phụ thuộc thứ tự key hay
//! cách ghi file. Encoding **có** kèm SHA-256 của từng file atlas, nên `content_hash`
//! ràng buộc đúng byte mà reader nạp: thay ảnh trong atlas (kể cả khi giữ nguyên mọi
//! `pixel_sha256`) thì `content_hash` phải đổi và `load()` phải từ chối bundle cũ.
//! Đánh đổi của thiết kế này: đổi bộ mã PNG cũng đổi `content_hash` — đổi lại reader
//! kiểm được nội dung hiển thị mà **không** cần giải ảnh.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::animation::{FrameTiming, PresentationState, SpriteLayer, source_name, status_name};
use crate::map::{CellIds, MapGrid};
use crate::rules::{Source, Status};

/// Schema hiện hành của bundle nội dung (có clip animation, bảng UI và audio V1).
pub const SCHEMA: u32 = 4;

/// Schema bundle có clip animation và bảng UI nhưng **không** có audio.
pub const SCHEMA_WITH_UI: u32 = 3;

/// Schema bundle có clip animation nhưng **không** có bảng UI.
///
/// Bundle schema này bị reader từ chối: client V1 cần biết phần tử UI nào dùng được,
/// phần tử nào phải vẽ thay bằng text — đọc bundle không có bảng UI sẽ im lặng bỏ qua
/// một phần nội dung. Dựng lại bằng `boom-assets bundle` là đủ.
pub const SCHEMA_WITH_CLIPS: u32 = 2;

/// Schema bundle chỉ có atlas + ô của map, **không** có clip animation.
///
/// Bundle schema này bị reader từ chối: client V1 cần clip để phát animation, và
/// đọc bundle không có clip sẽ im lặng bỏ qua một phần nội dung. Dựng lại bằng
/// `boom-assets bundle` là đủ.
pub const SCHEMA_SPRITE_ONLY: u32 = 1;

/// Tên file metadata trong thư mục bundle.
pub const METADATA_FILE: &str = "bundle.toml";

/// Thư mục con chứa các trang atlas PNG, tính từ thư mục bundle.
pub const ATLAS_DIR: &str = "atlas";
/// Thư mục con chứa payload âm thanh thô (ogg/wav).
pub const AUDIO_DIR: &str = "audio";

/// Vai trò sprite: nền của ô (ID tile trong file map).
pub const ROLE_TILE: &str = "tile";

/// Vai trò sprite: vật cản của ô (ID obj trong file map).
pub const ROLE_OBJECT: &str = "object";

/// Vai trò sprite: ảnh của một phần tử UI của luồng V1.
pub const ROLE_UI: &str = "ui";

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
    /// Nằm trong `content_hash` (xem [`Bundle::compute_content_hash`]): hash phải ràng
    /// buộc đúng byte file mà reader nạp, nếu không thì thay ảnh trong atlas vẫn giữ
    /// nguyên `content_hash` dù nội dung hiển thị đã khác.
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

/// Một clip animation trong bundle: dãi frame, canvas, pivot và thời lượng.
///
/// Clip **không mang ảnh riêng**: ảnh của clip nằm trong sprite mà [`Clip::sprite`]
/// trỏ tới (bảng `[[sprites]]`), nên mỗi entry chỉ có một bản pixel trong atlas.
///
/// Thời lượng frame giữ đúng trạng thái bằng chứng của R-079: [`FrameTiming::Unknown`]
/// không mang số; khi V1 chốt được nhịp, giá trị `design` đi kèm claim và nằm ở
/// profile rules — **không** hard-code trong bundle lẫn trong code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    /// ID ổn định của clip, ví dụ `c00.walk.down.static`.
    pub id: String,
    /// State trình diễn mà clip phục vụ.
    pub state: PresentationState,
    /// Tên pack chứa entry, ví dụ `FxCharacter`.
    pub pack: String,
    /// Đường dẫn entry trong pack, ví dụ `Character/C00-WalkDown_ALayerStatic`.
    pub entry: String,
    /// Lớp sprite; vắng nếu clip không chia lớp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<SpriteLayer>,
    /// Dword thứ tư của header sprite — nghĩa chưa chốt, giữ nguyên số.
    pub sprite_unk: u32,
    /// ID của sprite trong bảng `[[sprites]]` chứa ảnh của clip.
    pub sprite: String,
    /// Nguồn bằng chứng (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng (AGENTS §4).
    pub status: EvidenceStatus,
    /// Claim RULES_V1 liên quan; phải có ít nhất một claim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<String>,
    /// Ghi chú: lý do, giới hạn, ứng viên.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Thời lượng một frame.
    pub timing: FrameTiming,
    /// Dãi frame của clip; chỉ số ảnh trỏ vào ảnh của [`Clip::sprite`].
    pub frames: Vec<Frame>,
    /// Khối bảng animation trong ODT của client gốc mà [`Clip::sequences`] lấy ra,
    /// ví dụ `Character/C00_A`; vắng khi clip chưa nối ODT (ADR 0014).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odt: Option<String>,
    /// Chuỗi phát theo bảng ODT: mỗi state một chuỗi (frame, thời lượng theo đơn vị ODT).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequences: Vec<ClipSequence>,
}

/// Một chuỗi phát của clip, lấy nguyên từ bảng animation ODT của client gốc.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipSequence {
    /// Tên state trong ODT (phần sau số thứ tự của nhãn), ví dụ `Walk Left`, `Lock`.
    pub state: String,
    /// Các bước theo thứ tự phát.
    pub steps: Vec<SequenceStep>,
}

/// Một bước của chuỗi phát.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceStep {
    /// Chỉ số frame trong [`Clip::frames`].
    pub frame: u32,
    /// Thời lượng theo đơn vị ODT (giá trị thô; `0xFFFFFFFF`/`32767` gặp ở state giữ).
    pub units: u32,
}

impl Clip {
    /// Chuỗi phát của state `state` (so khớp đúng tên).
    #[must_use]
    pub fn sequence(&self, state: &str) -> Option<&ClipSequence> {
        self.sequences
            .iter()
            .find(|sequence| sequence.state == state)
    }
}

/// Màn hình của luồng V1 (đăng nhập → sảnh → phòng → trận → kết quả).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiScreen {
    /// Màn đăng nhập.
    Login,
    /// Sảnh danh sách phòng.
    Lobby,
    /// Phòng chờ.
    Room,
    /// Trong trận.
    Match,
    /// Màn kết quả.
    Result,
}

impl UiScreen {
    /// Khoá ổn định của màn hình — dùng cho mã hoá chuẩn (bundle `content_hash`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Lobby => "lobby",
            Self::Room => "room",
            Self::Match => "match",
            Self::Result => "result",
        }
    }
}

/// Vai trò của một phần tử UI trong luồng V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiRole {
    /// Nền của màn hình.
    Background,
    /// Nút bấm (điều hướng hoặc hành động).
    Button,
    /// Hộp thoại.
    Dialog,
    /// Hình kết quả lượt (thắng/thua/hòa).
    ResultArt,
    /// Nhãn hoặc ô thông tin.
    Info,
}

impl UiRole {
    /// Khoá ổn định của vai trò — dùng cho mã hoá chuẩn (bundle `content_hash`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Button => "button",
            Self::Dialog => "dialog",
            Self::ResultArt => "result_art",
            Self::Info => "info",
        }
    }
}

/// Trạng thái dùng của một phần tử UI.
///
/// Bundle **không có** trạng thái "đã duyệt": một ảnh có chữ chỉ được hiển thị khi
/// người duyệt ảnh ký quyết định trong quy trình
/// [IMAGE_REVIEW](https://github.com/aquaplatinumberlitz/boom-rs/blob/main/docs/specification/localization/IMAGE_REVIEW.md);
/// tới lúc đó client vẽ text từ catalog theo
/// [UI_STRINGS_V1](https://github.com/aquaplatinumberlitz/boom-rs/blob/main/docs/specification/localization/UI_STRINGS_V1.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiState {
    /// Có trong closure V1 và **không có chữ** cần duyệt: hiển thị được ngay.
    Selected,
    /// Có trong closure V1 nhưng **có chữ**: chỉ hiển thị khi ảnh được duyệt; chưa duyệt
    /// thì client vẽ text từ catalog.
    NeedsReviewText,
    /// Không dùng asset này: ghi rõ lý do và cách thay thế (vẽ text hoặc màu).
    Excluded,
}

impl UiState {
    /// Khoá ổn định của trạng thái — dùng cho mã hoá chuẩn (bundle `content_hash`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::NeedsReviewText => "needs_review_text",
            Self::Excluded => "excluded",
        }
    }
}

/// Một phần tử UI của luồng V1 trong bundle: màn hình, vai trò, trạng thái dùng và ảnh.
///
/// Phần tử **không mang ảnh riêng**: ảnh nằm trong sprite mà [`UiElement::sprite`] trỏ
/// tới (bảng `[[sprites]]`), nên mỗi entry chỉ có một bản pixel trong atlas — cùng nguyên
/// tắc với clip animation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiElement {
    /// ID ổn định của phần tử UI, ví dụ `lobby.create`.
    pub id: String,
    /// Màn hình của luồng V1 dùng phần tử này.
    pub screen: UiScreen,
    /// Vai trò của phần tử.
    pub role: UiRole,
    /// Entry pack của asset KR; vắng khi V1 không dùng asset nào cho phần tử này.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// Trạng thái dùng.
    pub state: UiState,
    /// Lý do hoặc ghi chú; **bắt buộc** (không rỗng) cho mọi phần tử — mọi quyết định
    /// dùng phải truy được, và phần tử chưa duyệt ảnh phải ghi rõ client vẽ text thay.
    pub reason: String,
    /// ID của sprite trong bảng `[[sprites]]` chứa ảnh; vắng khi `excluded`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<String>,
    /// Nguồn bằng chứng (AGENTS §4).
    pub source: Source,
    /// Trạng thái bằng chứng (AGENTS §4).
    pub status: EvidenceStatus,
}

impl UiElement {
    /// Phần tử này hiển thị được ngay, không cần duyệt ảnh.
    #[must_use]
    pub fn is_displayable(&self) -> bool {
        matches!(self.state, UiState::Selected)
    }

    /// Phần tử này phải vẽ thay bằng text từ catalog (chưa duyệt ảnh, hoặc không dùng
    /// asset nào).
    #[must_use]
    pub fn needs_text_fallback(&self) -> bool {
        !self.is_displayable()
    }
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

/// Một entry âm thanh V1 trong bundle (payload thô, không decode).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioAsset {
    /// ID cue ổn định theo `AUDIO_V1` (ví dụ `bgm_loop`, `bomb_exploded`).
    pub id: String,
    /// Entry nguồn trong pack `FxSound`.
    pub entry: String,
    /// `bgm` hoặc `sfx`.
    pub kind: String,
    /// Định dạng payload thô: `ogg`/`wav`; `mp3`/`unknown` bị builder bỏ qua có log.
    pub format: String,
    /// Đường dẫn tương đối trong bundle, ví dụ `audio/bomb_exploded.ogg`.
    pub file: String,
    /// Kích thước payload.
    pub bytes: u32,
    /// SHA-256 payload thô; nằm trong `content_hash`.
    pub sha256: String,
    /// Nhãn bằng chứng của mapping (không phải định dạng payload).
    pub status: EvidenceStatus,
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
    /// Định danh toàn bộ bundle: SHA-256 của [`Bundle::canonical_bytes`] (hex) — gồm
    /// metadata, SHA-256 file atlas và SHA-256 payload audio.
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
    /// Các clip animation của V1 (schema 2).
    #[serde(default)]
    pub clips: Vec<Clip>,
    /// Các phần tử UI của luồng V1 (schema 3).
    #[serde(default)]
    pub ui: Vec<UiElement>,
    /// Các entry âm thanh V1 (schema 4).
    #[serde(default)]
    pub audio: Vec<AudioAsset>,
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
             # Sinh bởi `boom-assets bundle`; content_hash băm metadata và byte file atlas.\n\
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
    /// little-endian. `content_hash` không nằm trong encoding này (vòng lặp), nhưng
    /// **SHA-256 của từng file atlas** thì có: hash phải ràng buộc đúng byte file mà
    /// reader nạp, nếu không thì thay ảnh trong atlas vẫn giữ nguyên `content_hash`.
    ///
    /// Trường `Option<u32>` được ghi bằng cờ hiện diện (0 = không có, 1 = có) rồi mới
    /// tới giá trị, để **không mất dữ liệu**: mọi giá trị `u32` — kể cả `u32::MAX` —
    /// đều biểu diễn được và khác nhau với `None`.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Canonical::new();
        out.bytes(if self.schema >= SCHEMA {
            b"boom-bundle-v4\0"
        } else {
            b"boom-bundle-v3\0"
        });
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
            // SHA-256 của file atlas nằm trong encoding: `content_hash` phải ràng buộc
            // đúng byte file mà reader nạp (xem [`Bundle::compute_content_hash`]).
            out.str(&page.sha256);
        }
        out.u32(self.sprites.len() as u32);
        for sprite in &self.sprites {
            out.str(&sprite.id);
            out.str(&sprite.entry);
            out.str(&sprite.role);
            out.str(sprite.status.as_str());
            out.str(&sprite.source_sha256);
            out.opt_u32(sprite.map_id);
            out.str(sprite.object_name.as_deref().unwrap_or(""));
            out.u32(sprite.images.len() as u32);
            for image in &sprite.images {
                out.u32(image.index);
                out.opt_u32(image.page);
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
            out.opt_u32(cell.tile);
            out.opt_u32(cell.object);
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
        out.u32(self.clips.len() as u32);
        for clip in &self.clips {
            out.str(&clip.id);
            out.str(&clip.state.key());
            out.str(&clip.pack);
            out.str(&clip.entry);
            out.opt_str(clip.layer.map(SpriteLayer::key));
            out.u32(clip.sprite_unk);
            out.str(&clip.sprite);
            out.str(source_name(clip.source));
            out.str(clip.status.as_str());
            out.u32(clip.claims.len() as u32);
            for claim in &clip.claims {
                out.str(claim);
            }
            out.str(clip.note.as_deref().unwrap_or(""));
            match &clip.timing {
                FrameTiming::Unknown { reason } => {
                    out.str("unknown");
                    out.str(reason);
                }
                FrameTiming::Known {
                    ms,
                    source,
                    status,
                    claims,
                } => {
                    out.str("known");
                    out.u32(*ms);
                    out.str(source_name(*source));
                    out.str(status_name(*status));
                    out.u32(claims.len() as u32);
                    for claim in claims {
                        out.str(claim);
                    }
                }
            }
            out.u32(clip.frames.len() as u32);
            for frame in &clip.frames {
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
            // Chuỗi ODT chỉ ghi khi có, sau một thẻ chứa `\0` (ID clip không thể chứa) nên
            // bundle cũ không có chuỗi giữ nguyên `content_hash`.
            if clip.odt.is_some() || !clip.sequences.is_empty() {
                out.str("\0odt");
                out.opt_str(clip.odt.as_deref());
                out.u32(clip.sequences.len() as u32);
                for sequence in &clip.sequences {
                    out.str(&sequence.state);
                    out.u32(sequence.steps.len() as u32);
                    for step in &sequence.steps {
                        out.u32(step.frame);
                        out.u32(step.units);
                    }
                }
            }
        }
        out.u32(self.ui.len() as u32);
        for element in &self.ui {
            out.str(&element.id);
            out.str(element.screen.key());
            out.str(element.role.key());
            out.str(element.state.key());
            out.opt_str(element.entry.as_deref());
            out.str(&element.reason);
            out.opt_str(element.sprite.as_deref());
            out.str(source_name(element.source));
            out.str(element.status.as_str());
        }
        if self.schema >= SCHEMA {
            out.u32(self.audio.len() as u32);
            for audio in &self.audio {
                out.str(&audio.id);
                out.str(&audio.entry);
                out.str(&audio.kind);
                out.str(&audio.format);
                out.str(&audio.file);
                out.u32(audio.bytes);
                out.str(&audio.sha256);
                out.str(audio.status.as_str());
            }
        }
        out.finish()
    }

    /// `content_hash` tính từ nội dung hiện tại: metadata, hash file atlas và hash payload audio.
    ///
    /// Đổi bất kỳ phần nào của bundle — kể cả pixel trong atlas — thì hash phải đổi.
    #[must_use]
    pub fn compute_content_hash(&self) -> String {
        sha256_hex(&self.canonical_bytes())
    }

    /// Lưới ô của map theo đúng thứ tự file nguồn.
    ///
    /// Dùng để phân loại ô bằng [`crate::map::MapProfile`] rồi kiểm spawn
    /// ([`crate::map::Terrain::check_spawns`]); bundle không nhân bản loại ô (AGENTS §3).
    #[must_use]
    pub fn map_grid(&self) -> MapGrid {
        MapGrid {
            width: self.width,
            height: self.height,
            cells: self
                .cells
                .iter()
                .map(|cell| CellIds {
                    tile: cell.tile,
                    obj: cell.object,
                })
                .collect(),
            spawns: self.spawns.clone(),
        }
    }

    /// Kiểm mọi tham chiếu trong metadata.
    ///
    /// Kiểm: lưới ô đủ và không trùng; mọi spawn nằm trong lưới và không trùng nhau;
    /// mỗi ảnh nằm trong trang atlas và nằm trong trang; mỗi frame trỏ tới ảnh có thật,
    /// hình chữ nhật nguồn nằm trong ảnh, canvas không nhỏ hơn hình nguồn, pivot nằm
    /// trong canvas; mỗi ô có sprite nền (hoặc nằm trong `unknown` kèm lý do) và sprite
    /// vật cản nếu có; mọi `unknown` đều có lý do.
    ///
    /// Với clip (schema 2): ID clip là duy nhất, clip phải có claim, clip phải trỏ
    /// tới một sprite có thật, mỗi frame của clip phải trỏ tới ảnh có thật của sprite
    /// đó, rect nằm trong ảnh, canvas không nhỏ hơn hình nguồn, pivot nằm trong canvas,
    /// và thời lượng phải hợp lệ (`unknown` không mang số; `known` phải có claim).
    ///
    /// Với bảng UI (schema 3): ID phần tử là duy nhất; phần tử `selected` hoặc
    /// `needs_review_text` phải có entry và trỏ tới một sprite có thật; phần tử
    /// `excluded` không được có sprite và **phải** có lý do; phần tử
    /// `needs_review_text` cũng phải có lý do (vì còn phải vẽ text thay khi ảnh chưa
    /// được duyệt). Xem [`validate_ui`].
    ///
    /// Phân loại spawn (va chạm với ô, ý nghĩa của vị trí spawn) vẫn thuộc bảng phân
    /// loại map, không nhân bản vào bundle (AGENTS §3 — một thông tin, một nguồn).
    pub fn validate(&self) -> Result<(), BundleError> {
        if !matches!(self.schema, SCHEMA_WITH_UI | SCHEMA) {
            return Err(BundleError::Schema { found: self.schema });
        }
        if self.schema == SCHEMA_WITH_UI && !self.audio.is_empty() {
            return Err(BundleError::Reference(
                "schema 3 không được có bảng audio".to_owned(),
            ));
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

        // Spawn phải nằm trong lưới và không trùng nhau.
        let mut seen_spawns: Vec<(u16, u16)> = Vec::with_capacity(self.spawns.len());
        for (x, y) in &self.spawns {
            if *x >= self.width || *y >= self.height {
                return Err(BundleError::Reference(format!(
                    "spawn ({x}, {y}) nằm ngoài lưới {}x{}",
                    self.width, self.height
                )));
            }
            if seen_spawns.contains(&(*x, *y)) {
                return Err(BundleError::Reference(format!("spawn ({x}, {y}) bị lặp")));
            }
            seen_spawns.push((*x, *y));
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

        self.validate_clips()?;

        self.validate_ui()?;
        self.validate_audio()?;

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

    /// Kiểm mọi clip của bundle (schema 2).
    ///
    /// Mỗi clip phải có ID duy nhất và ít nhất một claim; phải trỏ tới một sprite có
    /// thật; mỗi frame phải trỏ tới ảnh có thật của sprite đó (ảnh giữ chỗ 0×0 của
    /// client gốc cũng được phép — khung cuối clip chết trỏ về ảnh đó), rect nằm trong
    /// ảnh, canvas không nhỏ hơn hình nguồn và pivot nằm trong canvas. Thời lượng phải
    /// hợp lệ theo [`FrameTiming::validate`]: `unknown` không mang số, `known` phải có
    /// claim (kể cả khi nguồn là `design`).
    ///
    /// # Errors
    ///
    /// Trả [`BundleError::Reference`] cho clip đầu tiên vi phạm.
    fn validate_clips(&self) -> Result<(), BundleError> {
        let mut seen_clips: Vec<&str> = Vec::with_capacity(self.clips.len());
        for clip in &self.clips {
            if clip.id.is_empty() {
                return Err(BundleError::Reference("clip thiếu id".to_owned()));
            }
            if seen_clips.contains(&clip.id.as_str()) {
                return Err(BundleError::Reference(format!("clip {} bị lặp", clip.id)));
            }
            seen_clips.push(clip.id.as_str());
            if clip.claims.is_empty() {
                return Err(BundleError::Reference(format!(
                    "clip {} phải có claim (không có claim thì không truy được nguồn)",
                    clip.id
                )));
            }
            let expected_sprite = format!("{}/{}", clip.pack, clip.entry);
            if clip.sprite != expected_sprite {
                return Err(BundleError::Reference(format!(
                    "clip {} trỏ sprite {} nhưng entry là {expected_sprite}",
                    clip.id, clip.sprite
                )));
            }
            let Some(sprite) = self.sprites.iter().find(|s| s.id == clip.sprite) else {
                return Err(BundleError::Reference(format!(
                    "clip {} trỏ tới sprite {} không có trong bundle",
                    clip.id, clip.sprite
                )));
            };
            if clip.frames.is_empty() {
                return Err(BundleError::Reference(format!(
                    "clip {} không có frame nào",
                    clip.id
                )));
            }
            for sequence in &clip.sequences {
                if sequence.steps.is_empty() {
                    return Err(BundleError::Reference(format!(
                        "clip {}: chuỗi ODT {} không có bước nào",
                        clip.id, sequence.state
                    )));
                }
                if let Some(step) = sequence
                    .steps
                    .iter()
                    .find(|step| step.frame as usize >= clip.frames.len())
                {
                    return Err(BundleError::Reference(format!(
                        "clip {}: chuỗi ODT {} trỏ frame {} nhưng clip có {} frame",
                        clip.id,
                        sequence.state,
                        step.frame,
                        clip.frames.len()
                    )));
                }
            }
            for (position, frame) in clip.frames.iter().enumerate() {
                let Some(image) = sprite.images.get(frame.image as usize) else {
                    return Err(BundleError::Reference(format!(
                        "clip {}: frame {position} trỏ tới ảnh {} không có (sprite {} có {} ảnh)",
                        clip.id,
                        frame.image,
                        sprite.id,
                        sprite.images.len()
                    )));
                };
                let (Ok(x0), Ok(y0), Ok(x1), Ok(y1)) = (
                    u32::try_from(frame.x0),
                    u32::try_from(frame.y0),
                    u32::try_from(frame.x1),
                    u32::try_from(frame.y1),
                ) else {
                    return Err(BundleError::Reference(format!(
                        "clip {}: frame {position} có toạ độ nguồn âm",
                        clip.id
                    )));
                };
                if x1 < x0 || y1 < y0 || x1 > image.width || y1 > image.height {
                    return Err(BundleError::Reference(format!(
                        "clip {}: frame {position} nguồn ({x0},{y0})-({x1},{y1}) không nằm trong ảnh {}x{}",
                        clip.id, image.width, image.height
                    )));
                }
                if frame.canvas_width < i32::try_from(x1 - x0).unwrap_or(i32::MAX)
                    || frame.canvas_height < i32::try_from(y1 - y0).unwrap_or(i32::MAX)
                {
                    return Err(BundleError::Reference(format!(
                        "clip {}: frame {position} có canvas {}x{} nhỏ hơn hình nguồn",
                        clip.id, frame.canvas_width, frame.canvas_height
                    )));
                }
                if frame.pivot_x < 0
                    || frame.pivot_y < 0
                    || frame.pivot_x > frame.canvas_width
                    || frame.pivot_y > frame.canvas_height
                {
                    return Err(BundleError::Reference(format!(
                        "clip {}: frame {position} có pivot ({},{}) nằm ngoài canvas {}x{}",
                        clip.id,
                        frame.pivot_x,
                        frame.pivot_y,
                        frame.canvas_width,
                        frame.canvas_height
                    )));
                }
            }
            clip.timing
                .validate(&clip.id)
                .map_err(|e| BundleError::Reference(format!("clip {}: {e}", clip.id)))?;
        }
        Ok(())
    }

    /// Kiểm bảng audio schema 4: ID/file duy nhất, hash/định dạng hợp lệ và file nằm trong
    /// `audio/`; schema 3 phải không có bảng này.
    fn validate_audio(&self) -> Result<(), BundleError> {
        if self.schema < SCHEMA && !self.audio.is_empty() {
            return Err(BundleError::Reference(
                "schema cũ không được có bảng audio".to_owned(),
            ));
        }
        let mut seen_ids: Vec<&str> = Vec::with_capacity(self.audio.len());
        let mut seen_files: Vec<&str> = Vec::with_capacity(self.audio.len());
        for audio in &self.audio {
            if audio.id.is_empty() || audio.entry.is_empty() {
                return Err(BundleError::Reference("audio thiếu id/entry".to_owned()));
            }
            if seen_ids.contains(&audio.id.as_str()) {
                return Err(BundleError::Reference(format!(
                    "audio id {} bị lặp",
                    audio.id
                )));
            }
            seen_ids.push(&audio.id);
            if !matches!(audio.kind.as_str(), "bgm" | "sfx") {
                return Err(BundleError::Reference(format!(
                    "audio {} có kind {} không hỗ trợ",
                    audio.id, audio.kind
                )));
            }
            if !matches!(audio.format.as_str(), "ogg" | "wav") {
                return Err(BundleError::Reference(format!(
                    "audio {} có format {} không hỗ trợ",
                    audio.id, audio.format
                )));
            }
            if !audio.file.starts_with("audio/")
                || audio.file.contains("..")
                || audio.file.contains('\\')
            {
                return Err(BundleError::Reference(format!(
                    "audio {} có file {} ngoài thư mục audio",
                    audio.id, audio.file
                )));
            }
            if seen_files.contains(&audio.file.as_str()) {
                return Err(BundleError::Reference(format!(
                    "audio file {} bị lặp",
                    audio.file
                )));
            }
            seen_files.push(&audio.file);
            if audio.bytes == 0 || !is_sha256_hex(&audio.sha256) {
                return Err(BundleError::Reference(format!(
                    "audio {} thiếu bytes/hash hợp lệ",
                    audio.id
                )));
            }
        }
        Ok(())
    }

    /// Kiểm bảng UI của bundle (schema 3).
    ///
    /// Mỗi phần tử phải có ID duy nhất và không rỗng. Phần tử `selected` hoặc
    /// `needs_review_text` phải có entry và trỏ tới một sprite có thật trong `[[sprites]]`
    /// (ảnh của phần tử nằm ở đó, không nhân bản). Phần tử `excluded` **không** được có
    /// sprite và phải có lý do — không có lý do thì client không biết thay bằng gì. Mọi
    /// phần tử đều phải có lý do: mọi quyết định dùng phải truy được.
    ///
    /// # Errors
    ///
    /// Trả [`BundleError::Reference`] cho phần tử đầu tiên vi phạm.
    fn validate_ui(&self) -> Result<(), BundleError> {
        let mut seen: Vec<&str> = Vec::with_capacity(self.ui.len());
        for element in &self.ui {
            if element.id.is_empty() {
                return Err(BundleError::Reference("phần tử UI thiếu id".to_owned()));
            }
            if seen.contains(&element.id.as_str()) {
                return Err(BundleError::Reference(format!(
                    "phần tử UI {} bị lặp",
                    element.id
                )));
            }
            seen.push(element.id.as_str());
            if element.reason.is_empty() {
                return Err(BundleError::Reference(format!(
                    "phần tử UI {} phải có lý do",
                    element.id
                )));
            }
            match element.state {
                UiState::Selected | UiState::NeedsReviewText => {
                    if element.entry.is_none() {
                        return Err(BundleError::Reference(format!(
                            "phần tử UI {} phải có entry pack",
                            element.id
                        )));
                    }
                    let Some(sprite) = element.sprite.as_deref() else {
                        return Err(BundleError::Reference(format!(
                            "phần tử UI {} phải trỏ tới sprite chứa ảnh",
                            element.id
                        )));
                    };
                    if !self.sprites.iter().any(|item| item.id == sprite) {
                        return Err(BundleError::Reference(format!(
                            "phần tử UI {} trỏ tới sprite {} không có trong bundle",
                            element.id, sprite
                        )));
                    }
                }
                UiState::Excluded => {
                    if element.sprite.is_some() {
                        return Err(BundleError::Reference(format!(
                            "phần tử UI {} bị loại thì không được có sprite trong atlas",
                            element.id
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Audio cue theo ID event.
    #[must_use]
    pub fn audio_by_id(&self, id: &str) -> Option<&AudioAsset> {
        self.audio.iter().find(|audio| audio.id == id)
    }

    /// Clip theo ID.
    #[must_use]
    pub fn clip(&self, id: &str) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.id == id)
    }

    /// Các clip phục vụ một state trình diễn.
    #[must_use]
    pub fn clips_by_state(&self, state: PresentationState) -> Vec<&Clip> {
        self.clips
            .iter()
            .filter(|clip| clip.state == state)
            .collect()
    }

    /// Phần tử UI theo ID.
    #[must_use]
    pub fn ui_element(&self, id: &str) -> Option<&UiElement> {
        self.ui.iter().find(|element| element.id == id)
    }

    /// Các phần tử UI của một màn hình.
    #[must_use]
    pub fn ui_by_screen(&self, screen: UiScreen) -> Vec<&UiElement> {
        self.ui
            .iter()
            .filter(|element| element.screen == screen)
            .collect()
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

    /// Ghi `Option<u32>` **không mất dữ liệu**: cờ hiện diện rồi mới tới giá trị.
    ///
    /// Không dùng giá trị sentinel (`u32::MAX`): mọi ID `u32` phải cho encoding khác
    /// nhau, kể cả `u32::MAX` và `None`.
    fn opt_u32(&mut self, value: Option<u32>) {
        match value {
            Some(value) => {
                self.u32(1);
                self.u32(value);
            }
            None => {
                self.u32(0);
            }
        }
    }

    fn str(&mut self, value: &str) {
        self.u32(u32::try_from(value.len()).expect("chuỗi quá dài"));
        self.buf.extend_from_slice(value.as_bytes());
    }

    /// Ghi `Option<&str>` bằng cờ hiện diện rồi mới tới giá trị (không dùng sentinel).
    fn opt_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.u32(1);
                self.str(value);
            }
            None => {
                self.u32(0);
            }
        }
    }

    fn finish(self) -> Vec<u8> {
        self.buf
    }
}

/// Nạp bundle từ thư mục: đọc metadata, kiểm tham chiếu, kiểm hash và file atlas.
///
/// `content_hash` gồm SHA-256 của từng file atlas/audio, nên thay file — kể cả
/// khi người dựng cập nhật luôn `sha256` của file — vẫn bị từ chối vì hash cũ không
/// còn khớp nội dung.
///
/// # Lỗi
///
/// Trả lỗi nếu thiếu file, metadata sai schema, tham chiếu hỏng, `content_hash` không
/// khớp, hoặc file atlas không khớp SHA-256 đã ghi.
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

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
    for audio in &bundle.audio {
        let path = dir.join(&audio.file);
        let data = std::fs::read(&path).map_err(|e| BundleError::Io {
            path: path.clone(),
            message: e.to_string(),
        })?;
        if data.len() != usize::try_from(audio.bytes).unwrap_or(usize::MAX) {
            return Err(BundleError::FileHash {
                file: audio.file.clone(),
                recorded: audio.sha256.clone(),
                computed: format!("{} byte", data.len()),
            });
        }
        let actual = sha256_hex(&data);
        if actual != audio.sha256 {
            return Err(BundleError::FileHash {
                file: audio.file.clone(),
                recorded: audio.sha256.clone(),
                computed: actual,
            });
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
                write!(f, "schema bundle {found} không hỗ trợ (cần {SCHEMA})")?;
                if *found == SCHEMA_WITH_CLIPS {
                    write!(
                        f,
                        "; schema {SCHEMA_WITH_CLIPS} không có bảng UI — dựng lại bằng `boom-assets bundle`"
                    )?;
                } else if *found == SCHEMA_SPRITE_ONLY {
                    write!(
                        f,
                        "; schema {SCHEMA_SPRITE_ONLY} không có clip animation — dựng lại bằng `boom-assets bundle`"
                    )?;
                }
                Ok(())
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
    use crate::animation::Direction;

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
            clips: Vec::new(),
            ui: Vec::new(),
            audio: Vec::new(),
        }
    }

    /// Sprite mẫu chứa ảnh của clip (một ảnh 40×40, một frame).
    fn character_sprite() -> Sprite {
        Sprite {
            id: "FxCharacter/Character/C00-WalkDown_ALayerStatic".to_owned(),
            entry: "FxCharacter/Character/C00-WalkDown_ALayerStatic".to_owned(),
            role: "character_walk".to_owned(),
            status: EvidenceStatus::Confirmed,
            source_sha256: "ee".repeat(32),
            map_id: None,
            object_name: None,
            images: vec![SpriteImage {
                index: 0,
                page: Some(0),
                x: 0,
                y: 0,
                width: 40,
                height: 40,
                pixel_sha256: "ff".repeat(32),
            }],
            frames: Vec::new(),
        }
    }

    /// Clip mẫu: một frame trỏ vào ảnh 0 của sprite nhân vật.
    fn sample_clip() -> Clip {
        Clip {
            id: "c00.walk.down.static".to_owned(),
            state: PresentationState::Walk(Direction::Down),
            pack: "FxCharacter".to_owned(),
            entry: "Character/C00-WalkDown_ALayerStatic".to_owned(),
            layer: Some(SpriteLayer::Static),
            sprite_unk: 1,
            sprite: "FxCharacter/Character/C00-WalkDown_ALayerStatic".to_owned(),
            source: Source::KrStatic,
            status: EvidenceStatus::Confirmed,
            claims: vec!["R-077".to_owned()],
            note: None,
            timing: FrameTiming::Unknown {
                reason: "chưa có nguồn thời lượng frame".to_owned(),
            },
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
            odt: None,
            sequences: Vec::new(),
        }
    }

    /// Bundle có một clip hợp lệ (không cần pack thật).
    fn bundle_with_clip() -> Bundle {
        let mut bundle = sample_bundle();
        bundle.sprites.push(character_sprite());
        bundle.clips.push(sample_clip());
        bundle
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
    fn odt_sequences_round_trip_change_the_hash_and_are_validated() {
        let mut bundle = bundle_with_clip();
        bundle.validate().expect("clip hợp lệ");
        let without = bundle.compute_content_hash();
        bundle.clips[0].odt = Some("Character/C00_A".to_owned());
        bundle.clips[0].sequences = vec![ClipSequence {
            state: "Walk Left".to_owned(),
            steps: vec![SequenceStep { frame: 0, units: 3 }],
        }];
        bundle.validate().expect("chuỗi trỏ frame có thật");
        // Chuỗi ODT nằm trong `content_hash` (đổi chuỗi ⇒ đổi hash, ADR 0014 mục 3).
        let with = bundle.compute_content_hash();
        assert_ne!(with, without);
        bundle.clips[0].sequences[0].steps[0].units = 4;
        assert_ne!(bundle.compute_content_hash(), with);
        // TOML giữ nguyên chuỗi.
        bundle.content_hash = bundle.compute_content_hash();
        let parsed = Bundle::from_toml(&bundle.to_toml().expect("xuất TOML")).expect("đọc lại");
        assert_eq!(parsed, bundle);
        assert_eq!(
            parsed.clips[0].sequence("Walk Left").map(|s| s.steps.len()),
            Some(1)
        );
        // Frame ngoài clip bị từ chối.
        bundle.clips[0].sequences[0].steps[0].frame = 99;
        assert!(bundle.validate().is_err());
        // Chuỗi rỗng bị từ chối.
        bundle.clips[0].sequences[0].steps.clear();
        assert!(bundle.validate().is_err());
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

    /// Adler-32 (checksum cuối của stream zlib).
    fn adler32(data: &[u8]) -> u32 {
        const MOD: u32 = 65521;
        let mut a = 1u32;
        let mut b = 0u32;
        for &byte in data {
            a = (a + u32::from(byte)) % MOD;
            b = (b + a) % MOD;
        }
        (b << 16) | a
    }

    /// PNG hợp lệ: signature + IHDR + IDAT + IEND, mọi pixel cùng một màu.
    ///
    /// IDAT là stream zlib chứa deflate block **không nén** (stored block), nên không
    /// cần bộ giải nén: test chỉ cần file PNG đúng kích thước và **khác byte** so với
    /// ảnh khác — đúng việc mà probe "thay toàn bộ atlas" của reviewer làm.
    fn solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        // Mỗi hàng: 1 byte filter (0 = None) rồi pixel RGBA.
        let mut row = vec![0u8; 1 + (width as usize) * 4];
        for pixel in row[1..].as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&rgba);
        }
        let raw = row.repeat(height as usize);
        // zlib header (CMF=0x78, FLG=0x01) + stored deflate block + adler32.
        let mut zlib = vec![0x78u8, 0x01];
        let mut offset = 0usize;
        loop {
            let len = (raw.len() - offset).min(0xFFFF);
            let last = offset + len == raw.len();
            zlib.push(u8::from(last));
            zlib.extend_from_slice(&(len as u16).to_le_bytes());
            zlib.extend_from_slice(&(!(len as u16)).to_le_bytes());
            zlib.extend_from_slice(&raw[offset..offset + len]);
            offset += len;
            if last {
                break;
            }
        }
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

        let mut data = minimal_png(width, height);
        data.extend_from_slice(&(zlib.len() as u32).to_be_bytes());
        data.extend_from_slice(b"IDAT");
        data.extend_from_slice(&zlib);
        let mut crc_input = b"IDAT".to_vec();
        crc_input.extend_from_slice(&zlib);
        data.extend_from_slice(&crc32_ieee(&crc_input).to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"IEND");
        data.extend_from_slice(&crc32_ieee(b"IEND").to_be_bytes());
        data
    }

    /// Ghi metadata bundle ra thư mục.
    fn write_metadata(dir: &Path, bundle: &Bundle) {
        std::fs::write(
            dir.join(METADATA_FILE),
            bundle.to_toml().expect("xuất TOML"),
        )
        .expect("ghi metadata");
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

    /// Regression review PR #55 (P2 #1): thay toàn bộ atlas bằng PNG khác (cùng kích
    /// thước, pixel khác) và cập nhật `sha256` của file — `load()` phải từ chối.
    ///
    /// Trước khi sửa, encoding bỏ `atlas.sha256` nên `content_hash` không ràng buộc với
    /// pixel thật: bundle vẫn nạp được với **cùng hash cũ** dù nội dung hiển thị đã
    /// khác (đúng probe của reviewer).
    #[test]
    fn load_rejects_atlas_replaced_with_other_pixels() {
        let dir = temp_dir("swap");
        let atlas_dir = dir.join(ATLAS_DIR);
        std::fs::create_dir_all(&atlas_dir).expect("tạo thư mục atlas");
        let original = solid_png(64, 64, [255, 0, 0, 255]);
        std::fs::write(atlas_dir.join("page-000.png"), &original).expect("ghi PNG");

        let mut bundle = sample_bundle();
        bundle.atlas[0].sha256 = sha256_hex(&original);
        bundle.content_hash = bundle.compute_content_hash();
        write_metadata(&dir, &bundle);
        assert!(load(&dir).is_ok(), "bundle gốc phải nạp được");

        // Thay toàn bộ atlas bằng PNG trong suốt cùng kích thước, cập nhật `sha256` của
        // file — giữ nguyên `content_hash` và mọi `pixel_sha256`.
        let other = solid_png(64, 64, [0, 0, 0, 0]);
        assert_ne!(other, original, "ảnh thay thế phải khác byte ảnh gốc");
        std::fs::write(atlas_dir.join("page-000.png"), &other).expect("ghi PNG thay thế");
        let mut swapped = bundle.clone();
        swapped.atlas[0].sha256 = sha256_hex(&other);
        write_metadata(&dir, &swapped);
        assert!(
            matches!(load(&dir), Err(BundleError::ContentHash { .. })),
            "atlas bị thay mà content_hash cũ vẫn phải bị từ chối"
        );

        // Tính lại hash theo metadata mới thì hash phải đổi: định danh đi theo artifact.
        let mut recomputed = swapped.clone();
        recomputed.content_hash = recomputed.compute_content_hash();
        assert_ne!(recomputed.content_hash, bundle.content_hash);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `content_hash` phải ràng buộc byte file atlas: đổi `sha256` ghi trong metadata
    /// thì hash phải đổi.
    #[test]
    fn content_hash_covers_the_atlas_file_hash() {
        let mut bundle = sample_bundle();
        let first = bundle.compute_content_hash();
        bundle.atlas[0].sha256 = "dd".repeat(32);
        assert_ne!(bundle.compute_content_hash(), first);
    }

    /// Regression review PR #55 (P2 #2): spawn nằm ngoài lưới phải bị từ chối.
    #[test]
    fn rejects_spawn_outside_the_grid() {
        let mut bundle = sample_bundle();
        bundle.spawns = vec![(0, 0), (1, 1)]; // lưới 1x1 → (1, 1) nằm ngoài
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Regression review PR #55 (P2 #2): spawn trùng toạ độ phải bị từ chối.
    #[test]
    fn rejects_duplicate_spawns() {
        let mut bundle = sample_bundle();
        bundle.spawns = vec![(0, 0), (0, 0)];
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Spawn hợp lệ (trong lưới, không trùng) vẫn phải qua.
    #[test]
    fn accepts_spawns_inside_the_grid() {
        let mut bundle = sample_bundle();
        bundle.spawns = vec![(0, 0)];
        assert!(bundle.validate().is_ok());
    }

    /// Regression review PR #55 (P2 #3): `map_id` là `u32` — mọi giá trị phải cho
    /// `content_hash` khác nhau, kể cả hai giá trị trên `i32::MAX`.
    #[test]
    fn content_hash_distinguishes_every_map_id() {
        let mut bundle = sample_bundle();
        bundle.sprites[0].map_id = Some(2_147_483_648);
        let first = bundle.compute_content_hash();
        bundle.sprites[0].map_id = Some(2_147_483_649);
        assert_ne!(
            bundle.compute_content_hash(),
            first,
            "hai map_id khác phải cho content_hash khác"
        );
    }

    /// Regression review PR #55 (P2 #3): `Option<u32>` không dùng giá trị sentinel —
    /// `None` phải khác **mọi** giá trị, kể cả `u32::MAX`.
    #[test]
    fn content_hash_distinguishes_none_from_every_id() {
        let mut bundle = sample_bundle();
        bundle.sprites[0].map_id = None;
        let no_map_id = bundle.compute_content_hash();
        bundle.sprites[0].map_id = Some(u32::MAX);
        assert_ne!(bundle.compute_content_hash(), no_map_id);

        let mut bundle = sample_bundle();
        bundle.cells[0].tile = None;
        let no_tile = bundle.compute_content_hash();
        bundle.cells[0].tile = Some(u32::MAX);
        assert_ne!(bundle.compute_content_hash(), no_tile);

        let mut bundle = sample_bundle();
        bundle.sprites[0].images[0].page = None;
        let no_page = bundle.compute_content_hash();
        bundle.sprites[0].images[0].page = Some(u32::MAX);
        assert_ne!(bundle.compute_content_hash(), no_page);
    }

    /// Clip hợp lệ thì bundle hợp lệ và clip đi vòng lại được.
    #[test]
    fn bundle_with_a_clip_is_valid_and_round_trips() {
        let mut bundle = bundle_with_clip();
        assert!(bundle.validate().is_ok());
        bundle.content_hash = bundle.compute_content_hash();
        let text = bundle.to_toml().expect("xuất TOML");
        let parsed = Bundle::from_toml(&text).expect("đọc lại TOML");
        assert_eq!(parsed, bundle);
        assert_eq!(parsed.clips.len(), 1);
        let clip = parsed.clip("c00.walk.down.static").expect("clip theo ID");
        assert_eq!(clip.state, PresentationState::Walk(Direction::Down));
        assert_eq!(clip.frames.len(), 1);
        assert_eq!(clip.timing.ms(), None, "thời lượng unknown không mang số");
        assert_eq!(
            parsed
                .clips_by_state(PresentationState::Walk(Direction::Down))
                .len(),
            1
        );
        assert_eq!(parsed.clips_by_state(PresentationState::Dead).len(), 0);
    }

    /// `content_hash` phải bao phủ clip: đổi frame hoặc thời lượng của clip thì hash đổi.
    #[test]
    fn content_hash_covers_the_clips() {
        let mut bundle = bundle_with_clip();
        let first = bundle.compute_content_hash();
        bundle.clips[0].frames[0].offset_x += 1;
        assert_ne!(bundle.compute_content_hash(), first);

        let mut bundle = bundle_with_clip();
        let first = bundle.compute_content_hash();
        bundle.clips[0].timing = FrameTiming::Known {
            ms: 100,
            source: Source::Design,
            status: Status::Design,
            claims: vec!["R-079".to_owned()],
        };
        assert_ne!(bundle.compute_content_hash(), first);
    }

    /// Clip trỏ tới sprite không có trong bundle thì bị từ chối.
    #[test]
    fn rejects_clip_without_its_sprite() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].sprite = "FxCharacter/Character/C00-DieLayerStatic".to_owned();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// `sprite` của clip phải khớp entry (`<pack>/<entry>`).
    #[test]
    fn rejects_clip_whose_sprite_does_not_match_its_entry() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].pack = "FxTile".to_owned();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Frame của clip trỏ tới ảnh không tồn tại thì bị từ chối.
    #[test]
    fn rejects_clip_frame_without_its_image() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].frames[0].image = 7;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Pivot của frame clip nằm ngoài canvas thì bị từ chối.
    #[test]
    fn rejects_clip_frame_with_pivot_outside_the_canvas() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].frames[0].pivot_y = 41;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Clip không có frame thì bị từ chối.
    #[test]
    fn rejects_clip_without_frame() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].frames.clear();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// ID clip phải là duy nhất.
    #[test]
    fn rejects_duplicate_clip_id() {
        let mut bundle = bundle_with_clip();
        bundle.clips.push(sample_clip());
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Clip không có claim thì không truy được nguồn → bị từ chối.
    #[test]
    fn rejects_clip_without_claim() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].claims.clear();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Thời lượng `known` thiếu claim (kể cả khi nguồn là `design`) thì bị từ chối —
    /// không có nguồn thì không được mang số.
    #[test]
    fn rejects_clip_with_timing_value_but_without_claim() {
        let mut bundle = bundle_with_clip();
        bundle.clips[0].timing = FrameTiming::Known {
            ms: 100,
            source: Source::Design,
            status: Status::Design,
            claims: Vec::new(),
        };
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Schema cũ (chỉ có atlas + ô) bị từ chối: client V1 cần clip để phát animation.
    #[test]
    fn rejects_the_legacy_schema_without_clips() {
        let mut bundle = bundle_with_clip();
        bundle.schema = SCHEMA_SPRITE_ONLY;
        match bundle.validate() {
            Err(BundleError::Schema { found }) => assert_eq!(found, SCHEMA_SPRITE_ONLY),
            other => panic!("sai lỗi: {other:?}"),
        }
        // Thông điệp lỗi phải nói rõ cần dựng lại bundle.
        let message = BundleError::Schema {
            found: SCHEMA_SPRITE_ONLY,
        }
        .to_string();
        assert!(
            message.contains("dựng lại"),
            "thông điệp phải hướng dẫn dựng lại bundle: {message}"
        );
    }

    /// Schema 2 (có clip nhưng chưa có bảng UI) bị từ chối: client V1 cần biết phần
    /// tử UI nào dùng được, phần tử nào phải vẽ thay bằng text.
    #[test]
    fn rejects_the_schema_without_the_ui_table() {
        let mut bundle = bundle_with_clip();
        bundle.schema = SCHEMA_WITH_CLIPS;
        match bundle.validate() {
            Err(BundleError::Schema { found }) => assert_eq!(found, SCHEMA_WITH_CLIPS),
            other => panic!("sai lỗi: {other:?}"),
        }
        let message = BundleError::Schema {
            found: SCHEMA_WITH_CLIPS,
        }
        .to_string();
        assert!(
            message.contains("bảng UI") && message.contains("dựng lại"),
            "thông điệp phải nói rõ thiếu bảng UI: {message}"
        );
    }

    /// Sprite mẫu chứa ảnh của một phần tử UI.
    fn ui_sprite() -> Sprite {
        Sprite {
            id: "FxStage/Game/ResultInfoGood".to_owned(),
            entry: "FxStage/Game/ResultInfoGood".to_owned(),
            role: "ui".to_owned(),
            status: EvidenceStatus::Confirmed,
            source_sha256: "ab".repeat(32),
            map_id: None,
            object_name: None,
            images: vec![SpriteImage {
                index: 0,
                page: Some(0),
                x: 0,
                y: 0,
                width: 40,
                height: 40,
                pixel_sha256: "cd".repeat(32),
            }],
            frames: Vec::new(),
        }
    }

    /// Phần tử UI mẫu: mũi tên kết quả, không có chữ nên hiển thị được ngay.
    fn sample_ui_element() -> UiElement {
        UiElement {
            id: "result.info_good".to_owned(),
            screen: UiScreen::Result,
            role: UiRole::Info,
            entry: Some("FxStage/Game/ResultInfoGood".to_owned()),
            state: UiState::Selected,
            reason: "đã đọc trực tiếp PNG giải mã: chỉ có mũi tên, không thấy chữ".to_owned(),
            sprite: Some("FxStage/Game/ResultInfoGood".to_owned()),
            source: Source::KrStatic,
            status: EvidenceStatus::Confirmed,
        }
    }

    /// Bundle có bảng UI hợp lệ (không cần pack thật).
    fn bundle_with_ui() -> Bundle {
        let mut bundle = sample_bundle();
        bundle.sprites.push(ui_sprite());
        bundle.ui.push(sample_ui_element());
        bundle
    }

    /// Bundle schema 4 có bảng audio hợp lệ, round-trip TOML và tra theo ID.
    #[test]
    fn bundle_with_audio_is_valid_and_round_trips() {
        let mut bundle = sample_bundle();
        bundle.audio.push(AudioAsset {
            id: "bgm_loop".to_owned(),
            entry: "MP3/Village".to_owned(),
            kind: "bgm".to_owned(),
            format: "ogg".to_owned(),
            file: "audio/bgm_loop.ogg".to_owned(),
            bytes: 270601,
            sha256: "aa".repeat(32),
            status: EvidenceStatus::Design,
        });
        assert!(bundle.validate().is_ok());
        bundle.content_hash = bundle.compute_content_hash();
        let text = bundle.to_toml().expect("xuất TOML");
        assert!(text.contains("[[audio]]"));
        let parsed = Bundle::from_toml(&text).expect("đọc lại TOML");
        assert_eq!(parsed, bundle);
        assert_eq!(parsed.audio_by_id("bgm_loop").unwrap().bytes, 270601);
    }

    #[test]
    fn schema_three_without_audio_still_validates() {
        let mut bundle = sample_bundle();
        bundle.schema = SCHEMA_WITH_UI;
        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn schema_three_cannot_carry_audio() {
        let mut bundle = sample_bundle();
        bundle.schema = SCHEMA_WITH_UI;
        bundle.audio.push(AudioAsset {
            id: "x".to_owned(),
            entry: "Wave/x".to_owned(),
            kind: "sfx".to_owned(),
            format: "ogg".to_owned(),
            file: "audio/x.ogg".to_owned(),
            bytes: 1,
            sha256: "aa".repeat(32),
            status: EvidenceStatus::Design,
        });
        assert!(bundle.validate().is_err());
    }

    /// Bảng UI hợp lệ thì bundle hợp lệ, đọc vòng lại được và tra theo ID/màn hình.
    #[test]
    fn bundle_with_ui_is_valid_and_round_trips() {
        let mut bundle = bundle_with_ui();
        assert!(bundle.validate().is_ok());
        bundle.content_hash = bundle.compute_content_hash();
        let text = bundle.to_toml().expect("xuất TOML");
        assert!(text.contains("[[ui]]"));
        let parsed = Bundle::from_toml(&text).expect("đọc lại TOML");
        assert_eq!(parsed, bundle);
        assert_eq!(parsed.ui.len(), 1);
        let element = parsed
            .ui_element("result.info_good")
            .expect("phần tử theo ID");
        assert_eq!(element.screen, UiScreen::Result);
        assert_eq!(element.role, UiRole::Info);
        assert!(element.is_displayable());
        assert!(!element.needs_text_fallback());
        assert_eq!(parsed.ui_by_screen(UiScreen::Result).len(), 1);
        assert_eq!(parsed.ui_by_screen(UiScreen::Lobby).len(), 0);
    }

    /// `content_hash` phải bao phủ bảng UI: đổi trạng thái, lý do hay sprite thì hash đổi.
    #[test]
    fn content_hash_covers_the_ui_table() {
        let mut bundle = bundle_with_ui();
        let first = bundle.compute_content_hash();
        bundle.ui[0].state = UiState::NeedsReviewText;
        assert_ne!(bundle.compute_content_hash(), first);

        let mut bundle = bundle_with_ui();
        let first = bundle.compute_content_hash();
        bundle.ui[0].reason = "lý do khác".to_owned();
        assert_ne!(bundle.compute_content_hash(), first);

        let mut bundle = bundle_with_ui();
        let first = bundle.compute_content_hash();
        bundle.ui[0].screen = UiScreen::Lobby;
        assert_ne!(bundle.compute_content_hash(), first);

        let mut bundle = bundle_with_ui();
        let first = bundle.compute_content_hash();
        bundle.ui.clear();
        assert_ne!(bundle.compute_content_hash(), first);
    }

    /// Phần tử UI trỏ tới sprite không có trong bundle thì bị từ chối.
    #[test]
    fn rejects_ui_element_without_its_sprite() {
        let mut bundle = bundle_with_ui();
        bundle.ui[0].sprite = Some("FxStage/Game/ResultInfoBar".to_owned());
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Phần tử UI không có sprite thì bị từ chối (ảnh phải nằm trong atlas).
    #[test]
    fn rejects_ui_element_without_sprite() {
        let mut bundle = bundle_with_ui();
        bundle.ui[0].sprite = None;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Phần tử UI bị loại thì không được có sprite trong atlas.
    #[test]
    fn rejects_excluded_ui_element_with_a_sprite() {
        let mut bundle = bundle_with_ui();
        bundle.ui[0].state = UiState::Excluded;
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Phần tử UI bị loại mà không có lý do thì bị từ chối: client không biết thay bằng gì.
    #[test]
    fn rejects_excluded_ui_element_without_reason() {
        let mut bundle = bundle_with_ui();
        bundle.ui[0].state = UiState::Excluded;
        bundle.ui[0].sprite = None;
        bundle.ui[0].reason = String::new();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Ảnh có chữ mà chưa duyệt thì phải có lý do (client vẽ text từ catalog thay).
    #[test]
    fn rejects_ui_needing_review_without_reason() {
        let mut bundle = bundle_with_ui();
        bundle.ui[0].state = UiState::NeedsReviewText;
        bundle.ui[0].reason = String::new();
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// ID phần tử UI phải là duy nhất.
    #[test]
    fn rejects_duplicate_ui_id() {
        let mut bundle = bundle_with_ui();
        bundle.ui.push(sample_ui_element());
        assert!(matches!(bundle.validate(), Err(BundleError::Reference(_))));
    }

    /// Trạng thái UI chỉ có ba giá trị và **không** có trạng thái "đã duyệt": ảnh có chữ
    /// chỉ được hiển thị khi người duyệt ảnh ký quyết định (quy trình IMAGE_REVIEW).
    #[test]
    fn ui_states_have_no_approved_value() {
        let states = [
            UiState::Selected,
            UiState::NeedsReviewText,
            UiState::Excluded,
        ];
        let keys: Vec<&str> = states.iter().map(|state| state.key()).collect();
        assert_eq!(keys, ["selected", "needs_review_text", "excluded"]);
        assert!(!keys.contains(&"approved"));
        assert!(!keys.contains(&"translated"));
    }
}
