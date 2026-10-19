//! Closure dependency của một trận V1: `FxMap/Map/Village00` → entry pack cần phải có.
//!
//! Module này **không giải mã ảnh**: giải mã sprite `CDataI16Alpha` là task khác (issue #33).
//! Việc ở đây là chốt *entry nào* cần có, vai trò của nó, kích thước và SHA-256 của payload,
//! kèm danh sách **unknown** — phần chưa xác định được ghi rõ lý do thay vì bị bỏ qua âm thầm.
//!
//! Nguồn:
//!
//! - [PACKS](../../../../docs/specification/assets/PACKS.md) — định dạng pack `.idx/.idd`.
//! - [MAPS](../../../../docs/specification/assets/MAPS.md) — định dạng file `.map`.
//! - `data/maps/village00-kr.toml` — bảng phân loại ô của Village00 KR (PR #25).
//! - `docs/evidence/inventory/kr_v1_content.json` — vai trò của 103 entry đã fingerprint.
//! - [VN_LEGACY_SERVER_REFERENCE](../../../../docs/research/findings/VN_LEGACY_SERVER_REFERENCE.md)
//!   §1 — quy tắc tra theme (ở đó là `candidate` cho map chưa chạy).
//!
//! Quy tắc tra entry (RULES_V1 R-069, R-070, R-071):
//!
//! 1. ID tile và ID obj trong file `.map` là **chỉ số entry** của `fx/ObjectTable/ObjectTable`
//!    của build tương ứng (KR và VN dùng chung quy tắc, chỉ khác số thứ tự).
//! 2. Tên entry có dạng `<TileSet>_<Lớp>/<Tên>`; sprite của ô nằm ở pack `FxTile`, entry
//!    `<TileSet>/<Tên>`. Lớp (`Ground`/`Static`/`Box`/`Passable`/`Event`) quyết định va chạm,
//!    không đổi sprite.
//! 3. Theme của map là thư mục `Theme/*` dài nhất làm tiền tố tên map; entry theme là
//!    `FxMap/Theme/<Theme>`, preview là `FxMap/Etc/<Map>_mm`, BGM là `FxSound/MP3/<Theme>`.
//!    Quy tắc **tổng quát** này là R-071 (`candidate`: mới chỉ có bằng chứng VN legacy server
//!    và 9 map VN chạy được). Trên pack KR nó khớp 914/1.151 map, nên phần chỉ dùng cho V1 là
//!    kết quả đã xác nhận ở `Village00` (R-070: theme `Village`, preview `Village00_mm`,
//!    BGM `Village`).
//!
//! Bảng chọn nội dung (nhân vật, item, vai trò) là **dữ liệu content** của V1, không phải
//! hằng số luật chơi: nó kế thừa vai trò đã fingerprint ở `kr_v1_content.json` và phạm vi
//! V1 ở [SCOPE_V1](../../../../docs/planning/SCOPE_V1.md) §1.
//!
//! Phần **UI** của closure V1 nằm ở bảng [`UI_ASSETS`]: mỗi dòng là một phần tử UI của
//! luồng đăng nhập → sảnh → phòng → trận → kết quả, kèm trạng thái dùng. Bảng này là nguồn
//! chuẩn cho mục `[[ui]]` của bundle (schema 3) và cho cột "Trạng thái dùng" ở
//! [UI_STRINGS_V1 §3](../../../../docs/specification/localization/UI_STRINGS_V1.md).
//!
//! Bảng UI **tách khỏi manifest closure** (`build_village00_manifest`) vì hai thứ khác
//! nhau: manifest là closure *nội dung trận* (ô map, sprite, nhân vật, item) — mỗi dòng
//! là một entry pack có SHA-256; còn bảng UI là closure *luồng hiển thị* — mỗi dòng là
//! một **phần tử** có màn hình, vai trò, trạng thái duyệt ảnh và lý do thay thế, kèm
//! entry pack khi V1 dùng ảnh. `[[ui]]` của bundle trỏ tới `[[sprites]]` nên ảnh của
//! phần tử không bị nhân bản trong atlas.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use boom_content::bundle::{UiRole, UiScreen, UiState};
use boom_content::rules::Status;

use crate::legacy_map::{Build, LegacyMap, MapError, parse as parse_map};
use crate::pack::{Pack, PackError, PackIndex, load_index};

/// Schema hiện hành của manifest closure.
pub const SCHEMA: u32 = 1;

/// Pack KR chứa sprite ô (tile/object) của map.
pub const TILE_PACK: &str = "FxTile";

/// Entry KR chứa bảng tên obj: `fx/ObjectTable/ObjectTable`.
pub const OBJECT_TABLE: &str = "fx/ObjectTable/ObjectTable";

/// Map V1 (SCOPE_V1 §1).
pub const V1_MAP: &str = "Village00";

/// Theme của map V1; entry `FxMap/Theme/Village`.
pub const V1_THEME: &str = "Village";

/// Vai trò của một entry trong closure của một trận V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Sprite đứng yên của nhân vật (chưa xác nhận state nào dùng nó).
    CharacterIdle,
    /// Sprite bị nhốt của nhân vật (chưa xác nhận state nào dùng nó).
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

/// Nhãn trạng thái bằng chứng (AGENTS §4).
#[must_use]
fn status_str(status: Status) -> &'static str {
    match status {
        Status::Confirmed => "confirmed",
        Status::Candidate => "candidate",
        Status::Design => "design",
        Status::Unknown => "unknown",
    }
}

/// Một phần tử UI của luồng V1 trong closure: màn hình, vai trò, entry pack và trạng
/// thái dùng.
///
/// `entry` là `None` khi V1 **không dùng** asset nào cho phần tử này: phần tử vẫn được
/// ghi lại kèm lý do để client biết phải vẽ gì thay, thay vì im lặng thiếu ảnh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAsset {
    /// ID ổn định của phần tử UI, ví dụ `lobby.create`.
    pub id: &'static str,
    /// Màn hình của luồng V1 dùng phần tử này.
    pub screen: UiScreen,
    /// Vai trò của phần tử.
    pub role: UiRole,
    /// Entry pack của asset KR; `None` khi V1 không dùng asset nào cho phần tử này.
    pub entry: Option<&'static str>,
    /// Trạng thái dùng (xem [`UiState`]).
    pub state: UiState,
    /// Lý do hoặc ghi chú; **bắt buộc** (không rỗng) cho mọi phần tử — mọi quyết định
    /// dùng phải truy được, và phần tử chưa duyệt ảnh phải ghi rõ client vẽ text thay.
    pub reason: &'static str,
}

/// Bảng asset UI của luồng V1 (V1-032).
///
/// Mỗi dòng là **một phần tử UI** của luồng đăng nhập → sảnh → phòng → trận → kết quả.
/// Ba trạng thái dùng (AGENTS §4 cho nhãn bằng chứng; phần duyệt ảnh theo
/// [IMAGE_REVIEW](../../../../docs/specification/localization/IMAGE_REVIEW.md)):
///
/// | Trạng thái | Nghĩa | Ảnh trong atlas? | Hiển thị được? |
/// |---|---|---|---|
/// | `selected` | Có trong closure V1 và **không có chữ** cần duyệt | có | có |
/// | `needs_review_text` | Có trong closure V1 nhưng **có chữ** | có | chỉ khi ảnh được duyệt; chưa duyệt thì client vẽ text từ catalog |
/// | `excluded` | Không dùng asset này | không | thay bằng text/màu theo `reason` |
///
/// **Chưa ảnh nào ở trạng thái "đã duyệt"**: quy trình duyệt ảnh chưa sinh ra
/// `l10n/images_review.csv`, nên mọi ảnh có chữ đều là `needs_review_text`.
///
/// Nguồn: [UI_STRINGS_V1 §3](../../../../docs/specification/localization/UI_STRINGS_V1.md)
/// (danh mục ảnh có chữ của luồng V1) và ảnh giải mã đọc trực tiếp từ pack KR 00967
/// ngày 2026-10-20 — bản PNG giải mã nằm ở cache review ảnh của `boom-re-l10n`
/// (`l10n/.cache/image-review/`), cùng bộ ảnh mà `UI_STRINGS_V1 §3` mô tả. Nội dung chữ
/// trong ảnh là **bằng chứng mô tả**, không phải bản dịch.
const UI_ASSETS: &[UiAsset] = &[
    // Màn đăng nhập.
    UiAsset {
        id: "login.background",
        screen: UiScreen::Login,
        role: UiRole::Background,
        entry: Some("FxDesktop/LoginStage/CAStage"),
        state: UiState::Excluded,
        reason: "nền là JPEG 800×600: bộ giải mã bundle chỉ đọc sprite I16; ảnh còn có lời \
                 thoại tiếng Hàn, huy hiệu Nexon (ô 'N' góc trên) và bản VN có dòng bản quyền \
                 'Copyright (C) 2001 Nexon'; V1 dựng nền bằng màu, chữ lấy từ catalog",
    },
    UiAsset {
        id: "login.submit",
        screen: UiScreen::Login,
        role: UiRole::Button,
        entry: None,
        state: UiState::Excluded,
        reason: "chưa có asset nút đăng nhập trong closure; UI_STRINGS_V1 §2.2 đã chốt \
                 `login.submit` lấy key catalog nên V1 vẽ nút bằng text",
    },
    // Màn sảnh.
    UiAsset {
        id: "lobby.background",
        screen: UiScreen::Lobby,
        role: UiRole::Background,
        entry: Some("FxStage/Lobby/CAStage"),
        state: UiState::Excluded,
        reason: "nền là JPEG 800×600 có nhãn tiếng Hàn trong ảnh: hai tab chế độ '대결 모드' \
                 (bản VN: 'ĐẠI CHIẾN') và '상이 모드' (bản VN: 'NHIỆM VỤ') cùng nhãn '레벨'; \
                 V1 dựng nền bằng màu, chữ lấy từ catalog",
    },
    UiAsset {
        id: "lobby.create",
        screen: UiScreen::Lobby,
        role: UiRole::Button,
        entry: Some("FxStage/Lobby/LB_Bt_T_Create-NM"),
        state: UiState::NeedsReviewText,
        reason: "ảnh nút 9 frame có chữ Hàn (방만들기); chưa có quyết định duyệt ảnh nên V1 \
                 vẽ nút bằng text từ catalog",
    },
    UiAsset {
        id: "lobby.create_dialog",
        screen: UiScreen::Lobby,
        role: UiRole::Dialog,
        entry: Some("FxStage/Lobby/Dlg_LB-NMCreate"),
        state: UiState::NeedsReviewText,
        reason: "tiêu đề dialog nằm trong ảnh (방만들기); thân dialog là ô trống, nhãn do \
                 client vẽ; chưa duyệt ảnh thì V1 vẽ tiêu đề bằng text từ catalog",
    },
    UiAsset {
        id: "lobby.show_all",
        screen: UiScreen::Lobby,
        role: UiRole::Button,
        entry: Some("FxStage/Lobby/LB_Bt_T_ShowAllRenew"),
        state: UiState::Excluded,
        reason: "bản VN ngắn hơn nên mất nghĩa 'xem mọi phòng'; V1 không cần bộ lọc phòng, \
                 vẽ nút bằng text từ catalog",
    },
    UiAsset {
        id: "lobby.wait_room",
        screen: UiScreen::Lobby,
        role: UiRole::Button,
        entry: Some("FxStage/Lobby/LB_Bt_T_WaitRoomRenew"),
        state: UiState::Excluded,
        reason: "bản VN là danh từ, không phải hành động 'xem phòng chờ'; V1 vẽ nút bằng \
                 text từ catalog",
    },
    // Màn phòng chờ.
    UiAsset {
        id: "room.background",
        screen: UiScreen::Room,
        role: UiRole::Background,
        entry: Some("FxMap/Prepare/CAStage"),
        state: UiState::Excluded,
        reason: "nền là JPEG 800×600 có nhãn tiếng Hàn '방' (phòng), '던전' (dungeon), \
                 '게임디테일' (game detail) và nút '자동 준비' (auto-ready) — auto-ready ngoài \
                 scope FFA V1; V1 dựng nền bằng màu, chữ lấy từ catalog",
    },
    UiAsset {
        id: "room.ready",
        screen: UiScreen::Room,
        role: UiRole::Button,
        entry: Some("FxMap/Prepare/PRE_Bt_Ready"),
        state: UiState::NeedsReviewText,
        reason: "ảnh nút 6 frame có chữ Hàn (준비); chưa có quyết định duyệt ảnh nên V1 vẽ \
                 nút bằng text từ catalog",
    },
    UiAsset {
        id: "room.start",
        screen: UiScreen::Room,
        role: UiRole::Button,
        entry: Some("FxMap/Prepare/PRE_Bt_Start"),
        state: UiState::NeedsReviewText,
        reason: "ảnh nút 6 frame có chữ Hàn (시작); chưa có quyết định duyệt ảnh nên V1 vẽ nút \
                 bằng text từ catalog",
    },
    UiAsset {
        id: "room.info_dialog",
        screen: UiScreen::Room,
        role: UiRole::Dialog,
        entry: Some("FxMap/Prepare/Dlg_PRE-NMRoominfo"),
        state: UiState::NeedsReviewText,
        reason: "tiêu đề KR là '방정보변경' (đổi thông tin phòng) còn bản VN là 'THÔNG TIN' \
                 (thông tin) — khác nghĩa; nếu dùng thì nhãn lấy từ catalog, không tự chọn hay \
                 sửa nhãn trong ảnh",
    },
    UiAsset {
        id: "room.info_label",
        screen: UiScreen::Room,
        role: UiRole::Info,
        entry: Some("FxMap/Prepare/PRE_Bt_Roominfo"),
        state: UiState::Excluded,
        reason: "ảnh KR ghi '방정보변경' (đổi thông tin phòng) còn bản VN ghi 'THÔNG TIN' \
                 (thông tin) — khác nghĩa; V1 vẽ nhãn bằng text từ catalog",
    },
    UiAsset {
        id: "room.ready_indicator",
        screen: UiScreen::Room,
        role: UiRole::Info,
        entry: Some("FxStage/Lobby/ReadyBox"),
        state: UiState::Excluded,
        reason: "KR ghi '준비중' (đang chuẩn bị) còn bản VN ghi 'ĐẠI CHIẾN' — khác nghĩa hoàn \
                 toàn nên không dùng làm indicator ready; V1 vẽ trạng thái ready bằng text và màu",
    },
    // Màn trận.
    UiAsset {
        id: "match.background",
        screen: UiScreen::Match,
        role: UiRole::Background,
        entry: None,
        state: UiState::Excluded,
        reason: "nền trận là bản đồ V1: lưới ô và sprite nền đã nằm trong closure (role \
                 map/tile) nên V1 không dùng ảnh nền riêng cho màn trận",
    },
    UiAsset {
        id: "match.hud",
        screen: UiScreen::Match,
        role: UiRole::Info,
        entry: None,
        state: UiState::Excluded,
        reason: "HUD trận (đồng hồ, trạng thái nhốt/chết, tên item) chưa có asset trong \
                 closure và UI_STRINGS_V1 §2.5 ghi `unknown`; V1 vẽ bằng text từ catalog",
    },
    UiAsset {
        id: "match.navigation",
        screen: UiScreen::Match,
        role: UiRole::Button,
        entry: None,
        state: UiState::Excluded,
        reason: "trong trận không có nút điều hướng: đường về phòng mở ở màn kết quả; V1 \
                 không vẽ nút trong trận",
    },
    // Màn kết quả.
    UiAsset {
        id: "result.background",
        screen: UiScreen::Result,
        role: UiRole::Background,
        entry: None,
        state: UiState::Excluded,
        reason: "chưa có asset nền riêng cho màn kết quả trong closure; V1 dùng nền màu, \
                 chữ lấy từ catalog",
    },
    UiAsset {
        id: "result.continue",
        screen: UiScreen::Result,
        role: UiRole::Button,
        entry: None,
        state: UiState::Excluded,
        reason: "chưa có asset nút tiếp tục trong closure và UI_STRINGS_V1 §2.6 ghi \
                 `unknown`; V1 vẽ nút bằng text từ catalog",
    },
    UiAsset {
        id: "result.win",
        screen: UiScreen::Result,
        role: UiRole::ResultArt,
        entry: Some("FxStage/Game/Win"),
        state: UiState::NeedsReviewText,
        reason: "ảnh 4 frame ghép chữ Latin 'WIN!!'; phải duyệt cả chuỗi frame mới dùng, \
                 không chỉ frame 0",
    },
    UiAsset {
        id: "result.lose",
        screen: UiScreen::Result,
        role: UiRole::ResultArt,
        entry: Some("FxStage/Game/Lose"),
        state: UiState::NeedsReviewText,
        reason: "ảnh 5 frame ghép chữ Latin 'LOSE!!'; chuỗi frame chưa được người duyệt xem \
                 đủ nên V1 vẽ nhãn bằng text từ catalog",
    },
    UiAsset {
        id: "result.draw",
        screen: UiScreen::Result,
        role: UiRole::ResultArt,
        entry: Some("FxStage/Game/Draw"),
        state: UiState::NeedsReviewText,
        reason: "ảnh 5 frame ghép chữ Latin 'DRAW!!'; dấu có thể nằm ở frame khác nên phải \
                 duyệt cả chuỗi frame",
    },
    UiAsset {
        id: "result.info_bar",
        screen: UiScreen::Result,
        role: UiRole::Info,
        entry: Some("FxStage/Game/ResultInfoBar"),
        state: UiState::Excluded,
        reason: "spritesheet trộn nhãn KR/EN và có cột EXP/LUCCI (V1 không có tiền/EXP); V1 \
                 chỉ cần cột Hạng/Người chơi/Kết quả nên vẽ bằng text từ catalog",
    },
    UiAsset {
        id: "result.info_good",
        screen: UiScreen::Result,
        role: UiRole::Info,
        entry: Some("FxStage/Game/ResultInfoGood"),
        state: UiState::Selected,
        reason: "đã đọc trực tiếp PNG giải mã từ pack KR 00967 ngày 2026-10-20: chỉ có icon \
                 bàn tay (ngón cái) 4 frame, không thấy chữ nên không cần duyệt text",
    },
];

/// Bảng asset UI của luồng V1.
#[must_use]
pub fn ui_assets() -> &'static [UiAsset] {
    UI_ASSETS
}

/// Một entry trong closure: đường dẫn đầy đủ trong pack, vai trò, kích thước và SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureEntry {
    /// Đường dẫn entry, ví dụ `FxTile/Tile00/Tree00` hoặc `FxMap/Map/Village00`.
    pub entry: String,
    /// Vai trò của entry trong trận V1.
    pub role: ClosureRole,
    /// `confirmed` khi đã có bằng chứng; `candidate` khi mới là ứng viên.
    pub status: Status,
    /// Kích thước payload sau khi giải mã entry (byte).
    pub bytes: u32,
    /// SHA-256 của payload (hex, 64 ký tự).
    pub sha256: String,
    /// ID tile/obj trong map; chỉ có với vai trò [`ClosureRole::Tile`]/[`ClosureRole::Object`].
    pub map_id: Option<u32>,
    /// Tên trong `fx/ObjectTable/ObjectTable`; chỉ có với vai trò tile/object.
    pub object_name: Option<String>,
}

/// Loại phần chưa xác định trong closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnknownKind {
    /// Ô của map chưa tra được entry.
    MapCell,
    /// Entry suy ra theo quy tắc nhưng pack không có.
    MissingEntry,
    /// State trình diễn chưa xác định được sprite nào.
    CharacterState,
}

impl UnknownKind {
    /// Tên loại dùng trong manifest.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MapCell => "map_cell",
            Self::MissingEntry => "missing_entry",
            Self::CharacterState => "character_state",
        }
    }
}

/// Một phần của closure chưa xác định: ghi rõ lý do và ứng viên, không bỏ qua âm thầm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureUnknown {
    /// Loại phần chưa xác định.
    pub kind: UnknownKind,
    /// Định danh: `tile:1942`, `obj:1958` hoặc `C00:idle`.
    pub id: String,
    /// Vì sao chưa xác định được.
    pub reason: String,
    /// Entry ứng viên (có thể rỗng).
    pub candidates: Vec<String>,
}

/// Manifest closure của một trận V1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureManifest {
    /// Schema của manifest.
    pub schema: u32,
    /// Build của pack (`kr`).
    pub build: String,
    /// Tên map.
    pub map: String,
    /// Tile set của map; rỗng nếu các ô dùng nhiều tile set khác nhau.
    pub tileset: String,
    /// Các entry cần có, đã có SHA-256.
    pub entries: Vec<ClosureEntry>,
    /// Các phần chưa xác định.
    pub unknown: Vec<ClosureUnknown>,
}

impl ClosureManifest {
    /// Xuất manifest dạng TOML.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# Manifest closure một trận V1 — map {map} (build {build}).\n\
             # Sinh bởi `cargo run -p boom-assets --bin boom-closure -- <thư mục pack>`.\n\
             # Quy tắc tra entry: RULES_V1 R-069, R-070, R-071.\n\
             schema = {schema}\n\
             build = \"{build}\"\n\
             map = \"{map}\"\n\
             tileset = \"{tileset}\"",
            map = self.map,
            build = self.build,
            schema = self.schema,
            tileset = self.tileset,
        );
        for entry in &self.entries {
            let _ = writeln!(out, "\n[[entries]]");
            let _ = writeln!(out, "entry = \"{}\"", escape(&entry.entry));
            let _ = writeln!(out, "role = \"{}\"", entry.role.as_str());
            let _ = writeln!(out, "status = \"{}\"", status_str(entry.status));
            let _ = writeln!(out, "bytes = {}", entry.bytes);
            let _ = writeln!(out, "sha256 = \"{}\"", entry.sha256);
            if let Some(id) = entry.map_id {
                let _ = writeln!(out, "map_id = {id}");
            }
            if let Some(name) = &entry.object_name {
                let _ = writeln!(out, "object_name = \"{}\"", escape(name));
            }
        }
        for unknown in &self.unknown {
            let _ = writeln!(out, "\n[[unknown]]");
            let _ = writeln!(out, "kind = \"{}\"", unknown.kind.as_str());
            let _ = writeln!(out, "id = \"{}\"", escape(&unknown.id));
            let _ = writeln!(out, "reason = \"{}\"", escape(&unknown.reason));
            let _ = writeln!(out, "candidates = [{}]", toml_array(&unknown.candidates));
        }
        out
    }
}

/// Escape chuỗi TOML tối thiểu (đủ cho entry, tên và lý do trong manifest).
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

/// Mảng chuỗi TOML một dòng.
fn toml_array(values: &[String]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|v| format!("\"{}\"", escape(v)))
        .collect();
    items.join(", ")
}

/// Bảng tên obj của client gốc: `fx/ObjectTable/ObjectTable`.
///
/// Định dạng: `u32 count`, rồi `count` bản ghi `(u32 name_len, tên, i32 value)`. Chỉ phần tên
/// được dùng ở đây; `value` là offset vào khối dữ liệu còn lại, chưa có consumer đã biết.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectTable {
    names: Vec<String>,
}

impl ObjectTable {
    /// Số entry trong bảng.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Bảng không có entry nào.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Tên entry theo chỉ số; chỉ số chính là ID tile/obj trong map.
    #[must_use]
    pub fn name(&self, id: u32) -> Option<&str> {
        let index = usize::try_from(id).ok()?;
        self.names.get(index).map(String::as_str)
    }
}

/// Đọc bảng tên obj từ payload `fx/ObjectTable/ObjectTable`.
///
/// Tên trong bảng là cp949; ở đây decode lossy sang UTF-8 như phần đọc index pack
/// (`pack.rs`). Mọi tên dùng cho V1 đều là ASCII nên không mất thông tin.
pub fn parse_object_table(data: &[u8]) -> Result<ObjectTable, ClosureError> {
    let mut at = 0usize;
    let count = read_u32(data, &mut at, "số entry")?;
    // Chặn count rác trước khi cấp phát: mỗi entry tối thiểu 8 byte.
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if count > data.len().saturating_sub(at) / 8 {
        return Err(ClosureError::ObjectTableTruncated {
            count,
            what: "số entry",
        });
    }
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let len = usize::try_from(read_u32(data, &mut at, "độ dài tên")?).expect("u32 -> usize");
        let end = at
            .checked_add(len)
            .ok_or(ClosureError::ObjectTableTruncated {
                count,
                what: "độ dài tên",
            })?;
        let bytes = data
            .get(at..end)
            .ok_or(ClosureError::ObjectTableTruncated {
                count,
                what: "tên entry",
            })?;
        at = end;
        read_u32(data, &mut at, "giá trị")?;
        names.push(String::from_utf8_lossy(bytes).into_owned());
    }
    Ok(ObjectTable { names })
}

/// Tách tên entry ObjectTable `<TileSet>_<Lớp>/<Tên>` thành (tile set, tên sprite).
///
/// Lớp (`Ground`, `Static`, `Box`, `Passable`, `Event`) chỉ quyết định va chạm; sprite của
/// ô nằm ở pack [`TILE_PACK`] với entry `<TileSet>/<Tên>` (R-069).
#[must_use]
pub fn resolve_sprite(name: &str) -> Option<(&str, &str)> {
    let (prefix, object) = name.split_once('/')?;
    let (tileset, _layer) = prefix.split_once('_')?;
    if tileset.is_empty() || object.is_empty() {
        return None;
    }
    Some((tileset, object))
}

/// Tên các theme có trong index pack map (entry `Theme/<Theme>`).
#[must_use]
pub fn theme_names(index: &PackIndex) -> Vec<String> {
    let mut names: Vec<String> = index
        .entries
        .iter()
        .filter_map(|entry| entry.name.strip_prefix("Theme/"))
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Theme của một map: thư mục `Theme/*` **dài nhất** làm tiền tố tên map.
///
/// Đây là quy tắc **tổng quát** R-071, trạng thái `candidate`: nguồn là server giả lập VN
/// ([VN_LEGACY_SERVER_REFERENCE](../../../../docs/research/findings/VN_LEGACY_SERVER_REFERENCE.md)
/// §1, ở đó cũng là `candidate` cho map chưa chạy) và 9 map VN chạy được
/// ([VN_BATTLE_RUNTIME_REFERENCE](../../../../docs/research/findings/VN_BATTLE_RUNTIME_REFERENCE.md)
/// VN-B04) — bằng chứng gián tiếp, chưa có trace client nào xác nhận chọn theme.
///
/// Đo trên pack KR 00967: `FxMap` có 142 thư mục `Theme/*` và 1.151 map; rule khớp 914 map
/// (237 map không có thư mục theme nào làm tiền tố, legacy server dùng fallback
/// `ThemeDefault`), và BGM `FxSound/MP3/<theme>` chỉ tồn tại cho 32/80 theme dùng được — nên
/// phần BGM theo tên theme chỉ chắc ở `Village`.
///
/// Map V1 `Village00` đã được xác nhận riêng (R-070): `Village00` → `Village`.
/// Tiền tố `NXCL_`/`WS_`/`WinningStreak_` của VN được bỏ trước khi so.
#[must_use]
pub fn resolve_theme(map_name: &str, themes: &[String]) -> Option<String> {
    let base = ["NXCL_", "WS_", "WinningStreak_"]
        .iter()
        .find_map(|prefix| map_name.strip_prefix(prefix))
        .unwrap_or(map_name)
        .to_lowercase();
    themes
        .iter()
        .filter(|theme| base.starts_with(&theme.to_lowercase()))
        .max_by_key(|theme| theme.len())
        .cloned()
}

/// Entry cần cho một trận V1 ngoài phần ô của map.
///
/// Vai trò lấy từ `docs/evidence/inventory/kr_v1_content.json`; nhân vật và item theo
/// [SCOPE_V1](../../../../docs/planning/SCOPE_V1.md) §1. Entry theme, preview và BGM không
/// khai ở đây: chúng suy từ quy tắc tra theme (xem [`resolve_theme`], R-071 `candidate`;
/// riêng map V1 `Village00` đã xác nhận ở R-070). `Fire00` (lửa) và
/// `Bubble00` (bong bóng bị nhốt) chưa có bằng chứng consumer nên để `candidate`.
const REQUIRED_ENTRIES: &[(&str, ClosureRole, Status)] = &[
    ("FxMap/Map/Village00", ClosureRole::Map, Status::Confirmed),
    (
        "FxContents/Attr/BomberAttr",
        ClosureRole::CharacterAttr,
        Status::Confirmed,
    ),
    (
        "FxContents/Attr/ItemAttr",
        ClosureRole::ItemAttr,
        Status::Confirmed,
    ),
    ("fx/Item/Bomb", ClosureRole::ItemIcon, Status::Confirmed),
    ("fx/Item/Fire", ClosureRole::ItemIcon, Status::Confirmed),
    ("fx/Item/Roller", ClosureRole::ItemIcon, Status::Confirmed),
    (
        "FxDesktop/Bomb/Bomb00",
        ClosureRole::BombArt,
        Status::Confirmed,
    ),
    (
        "FxDesktop/Bomb/BombExplosion001",
        ClosureRole::ExplosionArt,
        Status::Confirmed,
    ),
    (
        "FxDesktop/Bomb/Fire00",
        ClosureRole::FlameArt,
        Status::Candidate,
    ),
    (
        "FxDesktop/Bomb/Bubble00",
        ClosureRole::TrappedArt,
        Status::Candidate,
    ),
];

/// Nhân vật V1 (SCOPE_V1 §1).
const CHARACTERS: &[&str] = &["C00", "C01"];

/// Hướng đi của V1.
const WALK_DIRECTIONS: &[&str] = &["Up", "Down", "Left", "Right"];

/// Lớp sprite nhân vật: `Static` (nền) và `Adjust` (lớp ngoại hình điều chỉnh).
const CHARACTER_LAYERS: &[&str] = &["Static", "Adjust"];

/// State nhân vật V1 chưa xác nhận được sprite nào: (state, lý do, họ tên ứng viên).
///
/// `walk` và `dead` đã có bằng chứng từ tên entry (`Walk*`, `DieLayer*`); `idle` và `trapped`
/// thì chưa có consumer nào được xác nhận, nên manifest ghi rõ ứng viên thay vì chọn bừa.
const UNKNOWN_CHARACTER_STATES: &[(&str, &str, &[&str])] = &[
    (
        "idle",
        "chưa có trace consumer xác nhận state idle dùng clip nào; ứng viên là họ sprite đứng yên `Blink`/`BlinkSmall`",
        &["Blink_ALayer", "BlinkSmall_ALayer"],
    ),
    (
        "trapped",
        "chưa có trace consumer xác nhận state trapped dùng clip nào; ứng viên là họ `FaintLayer` của nhân vật và bong bóng `FxDesktop/Bomb/Bubble00`",
        &["FaintLayer", "FxDesktop/Bomb/Bubble00"],
    ),
];

/// Họ tên sprite của một state nhân vật.
fn character_entry(character: &str, family: &str, layer: &str) -> String {
    format!("FxCharacter/Character/{character}-{family}{layer}")
}

/// Entry sprite nhân vật cho các state V1.
fn character_entries() -> Vec<(String, ClosureRole, Status)> {
    let mut out = Vec::new();
    for character in CHARACTERS {
        for direction in WALK_DIRECTIONS {
            for layer in CHARACTER_LAYERS {
                out.push((
                    format!("FxCharacter/Character/{character}-Walk{direction}_ALayer{layer}"),
                    ClosureRole::CharacterWalk,
                    Status::Confirmed,
                ));
            }
        }
        for layer in CHARACTER_LAYERS {
            out.push((
                format!("FxCharacter/Character/{character}-DieLayer{layer}"),
                ClosureRole::CharacterDead,
                Status::Confirmed,
            ));
        }
        for family in ["Blink_ALayer", "BlinkSmall_ALayer"] {
            for layer in CHARACTER_LAYERS {
                out.push((
                    character_entry(character, family, layer),
                    ClosureRole::CharacterIdle,
                    Status::Candidate,
                ));
            }
        }
        for layer in CHARACTER_LAYERS {
            out.push((
                format!("FxCharacter/Character/{character}-FaintLayer{layer}"),
                ClosureRole::CharacterTrapped,
                Status::Candidate,
            ));
        }
    }
    out
}

/// Dựng manifest closure của map V1 (Village00) từ thư mục chứa pack KR.
///
/// Thư mục phải có `<pack>.idx` và `<pack>.idd` của các pack KR (`FxMap`, `FxTile`, `fx`,
/// `FxCharacter`, `FxContents`, `FxDesktop`, `FxSound`).
pub fn build_village00_manifest(pack_dir: &Path) -> Result<ClosureManifest, ClosureError> {
    let map_pack = Pack::open(pack_dir, "FxMap", Build::Kr)?;
    let map = parse_map(&map_pack.read_named(&format!("Map/{V1_MAP}"))?, Build::Kr)?;
    let themes = theme_names(map_pack.index());
    let theme = resolve_theme(V1_MAP, &themes);
    let theme_in_pack = theme
        .as_deref()
        .is_some_and(|name| map_pack.index().find(&format!("Theme/{name}")).is_some());
    let preview_in_pack = map_pack.index().find(&format!("Etc/{V1_MAP}_mm")).is_some();
    drop(map_pack);

    let table = parse_object_table(&read_pack_entry(pack_dir, OBJECT_TABLE)?)?;
    let tile_index = load_index(pack_dir, TILE_PACK, Build::Kr)?;
    let sound_index = load_index(pack_dir, "FxSound", Build::Kr)?;

    let mut entries: Vec<ClosureEntry> = Vec::new();
    let mut unknown: Vec<ClosureUnknown> = Vec::new();
    let mut tilesets: Vec<String> = Vec::new();

    // Theme, preview và BGM suy từ quy tắc tra theme; entry thiếu thì ghi unknown kèm lý do.
    match (theme.as_deref(), theme_in_pack) {
        (Some(theme), true) => {
            entries.push(ClosureEntry {
                entry: format!("FxMap/Theme/{theme}"),
                role: ClosureRole::Theme,
                status: Status::Confirmed,
                bytes: 0,
                sha256: String::new(),
                map_id: None,
                object_name: None,
            });
            if sound_index.find(&format!("MP3/{theme}")).is_some() {
                entries.push(ClosureEntry {
                    entry: format!("FxSound/MP3/{theme}"),
                    role: ClosureRole::Music,
                    status: Status::Confirmed,
                    bytes: 0,
                    sha256: String::new(),
                    map_id: None,
                    object_name: None,
                });
            } else {
                unknown.push(ClosureUnknown {
                    kind: UnknownKind::MissingEntry,
                    id: format!("FxSound/MP3/{theme}"),
                    reason: format!("pack FxSound không có BGM theo tên theme {theme}"),
                    candidates: Vec::new(),
                });
            }
        }
        (Some(theme), false) => unknown.push(ClosureUnknown {
            kind: UnknownKind::MissingEntry,
            id: format!("FxMap/Theme/{theme}"),
            reason: format!("pack FxMap không có entry theme {theme} của map {V1_MAP}"),
            candidates: Vec::new(),
        }),
        (None, _) => unknown.push(ClosureUnknown {
            kind: UnknownKind::MissingEntry,
            id: format!("FxMap/Theme/{V1_MAP}"),
            reason: format!(
                "không tra được theme của map {V1_MAP} theo quy tắc thư mục Theme/* dài nhất"
            ),
            candidates: themes,
        }),
    }
    if preview_in_pack {
        entries.push(ClosureEntry {
            entry: format!("FxMap/Etc/{V1_MAP}_mm"),
            role: ClosureRole::Preview,
            status: Status::Confirmed,
            bytes: 0,
            sha256: String::new(),
            map_id: None,
            object_name: None,
        });
    } else {
        unknown.push(ClosureUnknown {
            kind: UnknownKind::MissingEntry,
            id: format!("FxMap/Etc/{V1_MAP}_mm"),
            reason: format!("pack FxMap không có preview của map {V1_MAP}"),
            candidates: Vec::new(),
        });
    }

    // Ô của map: mỗi ID tile/obj phải có entry, nếu không thì phải nằm trong unknown kèm lý do.
    for (kind, role, ids) in cell_ids(&map) {
        for id in ids {
            let label = format!("{kind}:{id}");
            let Some(name) = table.name(id) else {
                unknown.push(ClosureUnknown {
                    kind: UnknownKind::MapCell,
                    id: label,
                    reason: format!(
                        "ID {id} vượt số entry của ObjectTable ({} entry)",
                        table.len()
                    ),
                    candidates: Vec::new(),
                });
                continue;
            };
            let Some((tileset, object)) = resolve_sprite(name) else {
                unknown.push(ClosureUnknown {
                    kind: UnknownKind::MapCell,
                    id: label,
                    reason: format!("tên ObjectTable {name:?} không có dạng <TileSet>_<Lớp>/<Tên>"),
                    candidates: Vec::new(),
                });
                continue;
            };
            let path = format!("{tileset}/{object}");
            if tile_index.find(&path).is_none() {
                unknown.push(ClosureUnknown {
                    kind: UnknownKind::MapCell,
                    id: label,
                    reason: format!("pack {TILE_PACK} không có entry {path:?} cho ô {name:?}"),
                    candidates: Vec::new(),
                });
                continue;
            }
            if !tilesets.iter().any(|t| t == tileset) {
                tilesets.push(tileset.to_owned());
            }
            entries.push(ClosureEntry {
                entry: format!("{TILE_PACK}/{path}"),
                role,
                status: Status::Confirmed,
                bytes: 0,
                sha256: String::new(),
                map_id: Some(id),
                object_name: Some(name.to_owned()),
            });
        }
    }

    // Entry cố định của một trận V1.
    for (entry, role, status) in REQUIRED_ENTRIES {
        entries.push(ClosureEntry {
            entry: (*entry).to_owned(),
            role: *role,
            status: *status,
            bytes: 0,
            sha256: String::new(),
            map_id: None,
            object_name: None,
        });
    }
    for (entry, role, status) in character_entries() {
        entries.push(ClosureEntry {
            entry,
            role,
            status,
            bytes: 0,
            sha256: String::new(),
            map_id: None,
            object_name: None,
        });
    }

    // State nhân vật chưa xác nhận: ghi ứng viên, không chọn bừa.
    for (state, reason, families) in UNKNOWN_CHARACTER_STATES {
        for character in CHARACTERS {
            let mut candidates = Vec::new();
            for family in *families {
                if family.contains('/') {
                    candidates.push((*family).to_owned());
                    continue;
                }
                for layer in CHARACTER_LAYERS {
                    candidates.push(character_entry(character, family, layer));
                }
            }
            unknown.push(ClosureUnknown {
                kind: UnknownKind::CharacterState,
                id: format!("{character}:{state}"),
                reason: (*reason).to_owned(),
                candidates,
            });
        }
    }

    // Hash payload theo pack, mở từng pack một lần để không giữ nhiều `.idd` cùng lúc.
    let mut by_pack: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let stem = entry.entry.split('/').next().unwrap_or_default().to_owned();
        by_pack.entry(stem).or_default().push(index);
    }
    for (stem, indices) in by_pack {
        let pack = Pack::open(pack_dir, &stem, Build::Kr)?;
        for index in indices {
            let path = entries[index]
                .entry
                .split_once('/')
                .map_or("", |(_, rest)| rest)
                .to_owned();
            let data = pack.read_named(&path)?;
            entries[index].bytes = u32::try_from(data.len()).unwrap_or(u32::MAX);
            entries[index].sha256 = sha256_hex(&data);
        }
    }

    let tileset = if tilesets.len() == 1 {
        tilesets.remove(0)
    } else {
        if tilesets.len() > 1 {
            unknown.push(ClosureUnknown {
                kind: UnknownKind::MapCell,
                id: "tileset".to_owned(),
                reason: format!("map dùng nhiều tile set: {tilesets:?}"),
                candidates: Vec::new(),
            });
        }
        String::new()
    };

    Ok(ClosureManifest {
        schema: SCHEMA,
        build: "kr".to_owned(),
        map: V1_MAP.to_owned(),
        tileset,
        entries,
        unknown,
    })
}

/// Danh sách ID tile và ID obj đang dùng trong map, không trùng lặp.
fn cell_ids(map: &LegacyMap) -> Vec<(&'static str, ClosureRole, Vec<u32>)> {
    let mut tiles: Vec<u32> = Vec::new();
    let mut objects: Vec<u32> = Vec::new();
    for cell in &map.grid.cells {
        if let Some(tile) = cell.tile
            && !tiles.contains(&tile)
        {
            tiles.push(tile);
        }
        if let Some(object) = cell.obj
            && !objects.contains(&object)
        {
            objects.push(object);
        }
    }
    tiles.sort_unstable();
    objects.sort_unstable();
    vec![
        ("tile", ClosureRole::Tile, tiles),
        ("obj", ClosureRole::Object, objects),
    ]
}

/// Đọc và giải mã một entry từ pack theo đường dẫn đầy đủ (`<pack>/<path>`).
fn read_pack_entry(pack_dir: &Path, entry: &str) -> Result<Vec<u8>, ClosureError> {
    let (stem, path) = entry
        .split_once('/')
        .ok_or_else(|| ClosureError::BadEntryPath(entry.to_owned()))?;
    let pack = Pack::open(pack_dir, stem, Build::Kr)?;
    Ok(pack.read_named(path)?)
}

/// SHA-256 (FIPS 180-4) của payload, hex lowercase.
///
/// Tự triển khai để crate chỉ đọc asset không phải thêm dependency chỉ để băm một file.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    let mut out = String::with_capacity(64);
    for byte in sha256(data) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// SHA-256 (FIPS 180-4) của payload.
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

        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        let (mut e, mut f, mut g, mut h) = (state[4], state[5], state[6], state[7]);
        for (&k, &w) in K.iter().zip(schedule.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k)
                .wrapping_add(w);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
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

fn read_u32(data: &[u8], at: &mut usize, what: &'static str) -> Result<u32, ClosureError> {
    let end = at
        .checked_add(4)
        .ok_or(ClosureError::ObjectTableTruncated { count: 0, what })?;
    let bytes = data
        .get(*at..end)
        .ok_or(ClosureError::ObjectTableTruncated { count: 0, what })?;
    *at = end;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("đã kiểm 4 byte"),
    ))
}

/// Lỗi khi dựng closure.
#[derive(Debug)]
pub enum ClosureError {
    /// Lỗi đọc pack.
    Pack(PackError),
    /// Lỗi đọc file `.map`.
    Map(MapError),
    /// Bảng tên obj cụt hoặc sai.
    ObjectTableTruncated {
        /// Số entry khai báo (0 = không xác định được).
        count: usize,
        /// Field đang đọc khi lỗi xảy ra.
        what: &'static str,
    },
    /// Đường dẫn entry không có dạng `<pack>/<path>`.
    BadEntryPath(String),
}

impl fmt::Display for ClosureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pack(e) => write!(f, "{e}"),
            Self::Map(e) => write!(f, "{e}"),
            Self::ObjectTableTruncated { count, what } => {
                write!(
                    f,
                    "bảng tên obj cụt hoặc sai khi đọc {what} (khai báo {count} entry)"
                )
            }
            Self::BadEntryPath(path) => write!(f, "đường dẫn entry {path:?} thiếu phần pack"),
        }
    }
}

impl std::error::Error for ClosureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pack(e) => Some(e),
            Self::Map(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PackError> for ClosureError {
    fn from(value: PackError) -> Self {
        Self::Pack(value)
    }
}

impl From<MapError> for ClosureError {
    fn from(value: MapError) -> Self {
        Self::Map(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn object_table_names_are_read_in_index_order() {
        // count=2: ("BnbNavi", -1), ("Tile00_Static/Tree00", 1)
        let mut data = Vec::new();
        data.extend_from_slice(&2u32.to_le_bytes());
        for (name, value) in [("BnbNavi", -1i32), ("Tile00_Static/Tree00", 1)] {
            data.extend_from_slice(&(name.len() as u32).to_le_bytes());
            data.extend_from_slice(name.as_bytes());
            data.extend_from_slice(&value.to_le_bytes());
        }
        let table = parse_object_table(&data).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(table.name(0), Some("BnbNavi"));
        assert_eq!(table.name(1), Some("Tile00_Static/Tree00"));
        assert_eq!(table.name(2), None);
    }

    #[test]
    fn truncated_object_table_is_an_error() {
        let data = 4u32.to_le_bytes().to_vec();
        assert!(matches!(
            parse_object_table(&data),
            Err(ClosureError::ObjectTableTruncated { .. })
        ));
    }

    #[test]
    fn sprite_name_splits_tileset_and_object() {
        assert_eq!(
            resolve_sprite("Tile00_Static/Tree00"),
            Some(("Tile00", "Tree00"))
        );
        assert_eq!(
            resolve_sprite("Tile00_Ground/Grass00"),
            Some(("Tile00", "Grass00"))
        );
        // Tên không phải ô của map (script object) không có dạng `<TileSet>_<Lớp>/<Tên>`.
        assert_eq!(resolve_sprite("Game/MonPudadak1"), None);
        assert_eq!(resolve_sprite("Tile00"), None);
    }

    #[test]
    fn theme_is_the_longest_matching_theme_folder() {
        // Kiểm quy tắc R-071 (`candidate`): map V1 `Village00` đã xác nhận riêng ở R-070.
        let themes: Vec<String> = ["Village", "ExVillage", "Volcano"]
            .iter()
            .map(|t| (*t).to_owned())
            .collect();
        assert_eq!(
            resolve_theme("Village00", &themes),
            Some("Village".to_owned())
        );
        // Tiền tố riêng của VN được bỏ trước khi so.
        assert_eq!(
            resolve_theme("NXCL_Village03", &themes),
            Some("Village".to_owned())
        );
        assert_eq!(resolve_theme("KhongCoTheme", &themes), None);
    }

    #[test]
    fn manifest_toml_lists_entries_and_unknowns() {
        let manifest = ClosureManifest {
            schema: SCHEMA,
            build: "kr".to_owned(),
            map: V1_MAP.to_owned(),
            tileset: "Tile00".to_owned(),
            entries: vec![ClosureEntry {
                entry: "FxTile/Tile00/Tree00".to_owned(),
                role: ClosureRole::Object,
                status: Status::Confirmed,
                bytes: 3831,
                sha256: "aa".repeat(32),
                map_id: Some(1958),
                object_name: Some("Tile00_Static/Tree00".to_owned()),
            }],
            unknown: vec![ClosureUnknown {
                kind: UnknownKind::CharacterState,
                id: "C00:idle".to_owned(),
                reason: "chưa có trace".to_owned(),
                candidates: vec!["FxCharacter/Character/C00-Blink_ALayerStatic".to_owned()],
            }],
        };
        let toml = manifest.to_toml();
        assert!(toml.contains("map = \"Village00\""));
        assert!(toml.contains("entry = \"FxTile/Tile00/Tree00\""));
        assert!(toml.contains("role = \"object\""));
        assert!(toml.contains("map_id = 1958"));
        assert!(toml.contains("[[unknown]]"));
        assert!(toml.contains("id = \"C00:idle\""));
    }

    /// Bảng UI của closure phải nhất quán: ID duy nhất, mọi quyết định đều có lý do,
    /// và mọi màn hình của luồng V1 đều có **nền** lẫn **nút điều hướng** — hoặc ghi rõ
    /// thay bằng gì (vẽ text/màu) khi V1 không dùng asset.
    #[test]
    fn ui_table_is_consistent_and_covers_every_v1_screen() {
        let table = ui_assets();
        assert!(!table.is_empty());

        let mut ids: Vec<&str> = Vec::with_capacity(table.len());
        for asset in table {
            assert!(!asset.id.is_empty(), "phần tử UI thiếu id");
            assert!(!ids.contains(&asset.id), "phần tử UI {} bị lặp", asset.id);
            ids.push(asset.id);
            // Mọi quyết định dùng phải truy được: không có lý do thì không biết thay
            // bằng gì.
            assert!(
                !asset.reason.is_empty(),
                "phần tử UI {} không có lý do",
                asset.id
            );
        }

        for screen in [
            UiScreen::Login,
            UiScreen::Lobby,
            UiScreen::Room,
            UiScreen::Match,
            UiScreen::Result,
        ] {
            let of_screen: Vec<&UiAsset> = table
                .iter()
                .filter(|asset| asset.screen == screen)
                .collect();
            assert!(!of_screen.is_empty(), "màn {screen:?} chưa có phần tử UI");
            assert!(
                of_screen
                    .iter()
                    .any(|asset| asset.role == UiRole::Background),
                "màn {screen:?} chưa có phần tử nền"
            );
            assert!(
                of_screen.iter().any(|asset| asset.role == UiRole::Button),
                "màn {screen:?} chưa có nút điều hướng"
            );
        }
    }

    /// Ảnh **có chữ** chưa được duyệt thì không được ở trạng thái hiển thị được; bảng
    /// closure chỉ dùng ba trạng thái nên không có trạng thái "đã duyệt".
    #[test]
    fn ui_table_never_marks_an_image_with_text_as_displayable() {
        /// Ảnh có chữ, đọc trực tiếp từ PNG giải mã của pack KR 00967 ngày 2026-10-20.
        const TEXT_BEARING: &[&str] = &[
            "lobby.create",
            "lobby.create_dialog",
            "room.ready",
            "room.start",
            "room.info_dialog",
            "result.win",
            "result.lose",
            "result.draw",
        ];
        let table = ui_assets();
        // Cả ba trạng thái đều phải có mặt trong bảng: `selected` (dùng được ngay),
        // `needs_review_text` (có chữ, chờ duyệt) và `excluded` (không dùng asset).
        assert!(table.iter().any(|asset| asset.state == UiState::Selected));
        assert!(
            table
                .iter()
                .any(|asset| asset.state == UiState::NeedsReviewText)
        );
        assert!(table.iter().any(|asset| asset.state == UiState::Excluded));
        for asset in table {
            if TEXT_BEARING.contains(&asset.id) {
                assert_ne!(
                    asset.state,
                    UiState::Selected,
                    "ảnh có chữ {} không được ở trạng thái hiển thị được",
                    asset.id
                );
            }
        }
    }

    /// Mọi entry UI có trong closure phải tồn tại thật trong pack KR, và ảnh của nó
    /// phải giải được bằng bộ đọc sprite I16 (nền JPEG đã bị loại nên không vào đây).
    #[test]
    fn ui_entries_in_the_closure_exist_in_the_pack() {
        let Some(root) = std::env::var_os("BOOM_KR_PACK_DIR") else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let root = Path::new(&root);
        for asset in ui_assets() {
            if asset.state == UiState::Excluded {
                continue;
            }
            let Some(entry) = asset.entry else {
                continue;
            };
            let (pack, path) = entry.split_once('/').expect("entry có phần pack");
            let handle = crate::pack::Pack::open(root, pack, crate::legacy_map::Build::Kr)
                .unwrap_or_else(|e| panic!("mở pack {pack}: {e}"));
            let blob = handle
                .read_named(path)
                .unwrap_or_else(|e| panic!("đọc entry {entry}: {e}"));
            crate::sprite::parse(&blob).unwrap_or_else(|e| panic!("giải sprite {entry}: {e}"));
        }
    }
}
