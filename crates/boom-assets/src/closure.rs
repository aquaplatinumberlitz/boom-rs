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
//! V1 ở [SCOPE_V1](../../../../docs/planning/SCOPE_V1.md) §1. Bảng đó nay là **data** ở
//! `data/assets/closure-v1.toml` (nguồn chuẩn duy nhất, nạp qua
//! [`boom_content::assets`]): mỗi dòng mang vai trò, nhãn bằng chứng và claim RULES_V1.
//!
//! Phần **UI** của closure V1 cũng nằm ở data: `data/assets/ui-v1.toml`. Mỗi dòng là một
//! phần tử UI của luồng đăng nhập → sảnh → phòng → trận → kết quả, kèm trạng thái dùng.
//! Bảng UI **tách khỏi manifest closure** (`build_village00_manifest`) vì hai thứ khác
//! nhau: manifest là closure *nội dung trận* (ô map, sprite, nhân vật, item) — mỗi dòng
//! là một entry pack có SHA-256; còn bảng UI là closure *luồng hiển thị* — mỗi dòng là
//! một **phần tử** có màn hình, vai trò, trạng thái duyệt ảnh và lý do thay thế, kèm
//! entry pack khi V1 dùng ảnh. `[[ui]]` của bundle trỏ tới `[[sprites]]` nên ảnh của
//! phần tử không bị nhân bản trong atlas.
//!
//! # Bảng state của client trong ODT (R-094)
//!
//! `fx/ObjectTable/ObjectTable` không chỉ chứa bảng tên ô của map: sau bảng tên còn là
//! các **khối dữ liệu**, mỗi khối là bảng animation của một đối tượng. Khối
//! `Character/<Id>_A` liệt kê **state trình diễn của client** theo thứ tự:
//!
//! | nhãn | state | entry |
//! |---|---|---|
//! | `00 Blink` | `Blink` | `Character/C00-Blink_A` |
//! | `01 Die` | `Die` | `Character/C00-Die` |
//! | `02 Faint` | `Faint` | `Character/C00-Faint` |
//! | `03 Hurrah` | `Hurrah` | `Character/C00-Hurrah_A` |
//! | `12 Stop Left` … `15 Stop Down` | đứng yên theo hướng | `Character/C00-Walk<Dir>_A` (frame 0) |
//! | `20 Walk Left` … `23 Walk Down` | đi theo hướng | `Character/C00-Walk<Dir>_A` |
//! | `24 Start` | `Start` | `Character/C00-Start_A` |
//! | `25 Locked In` (+ `52`…`56`) | bị khoá (bị nhốt) | `Character/C00-Die` (frame 0…1, giữ vô hạn) |
//!
//! Mỗi state còn kèm danh sách `(chỉ số frame, thời lượng)`; **số frame khác nhau** dùng
//! trong state khớp **đúng** số frame của sprite trong pack (`Blink` 3 frame, `Die` 14
//! frame, `Faint` 3 frame, `Hurrah` 4 frame, `Walk<Dir>` 5 frame) nên bảng ODT là bảng
//! animation của chính build KR 00967, không phải dữ liệu của build khác.
//!
//! Tên entry trong ODT là **tiền tố** của entry pack: client ghép thêm `Layer` +
//! `Static`/`Adjust` (`C00-Blink_A` → `C00-Blink_ALayerStatic`). Vì vậy closure dùng
//! chính ODT để **kiểm** entry nhân vật mình khai, thay vì tin vào bảng cứng.
//!
//! **Giới hạn của bảng ODT** (R-094, AGENTS §4): nó xác nhận **nhãn → entry**, **không**
//! xác nhận state core của V1 dùng nhãn nào. Mapping `idle` → `Blink`, `trapped` →
//! `Faint` là **quyết định của V1** (R-086 `design`) ⇒ closure gắn nhãn `design` cho hai
//! sprite đó, **không** phải `confirmed`. Họ `BlinkSmall_A` **không** có trong bảng state
//! `Character/<Id>_A` của C00/C01 ⇒ V1 **không chọn** họ này cho `idle`; họ sprite đó
//! **có** trong ODT ở khối khác (`Character/<Id>_Display_A`) nên **không** kết luận là
//! client không dùng.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use boom_content::assets::{AssetError, ClosureTable};
use boom_content::rules::Status;

/// Vai trò của một entry trong closure: định nghĩa ở `boom-content` (nguồn chuẩn chung
/// với bảng data) và re-export để code closure dùng như trước.
pub use boom_content::assets::ClosureRole;

use crate::legacy_map::{Build, LegacyMap, MapError, parse as parse_map};
use crate::pack::{Pack, PackError, PackIndex, load_index};

/// Schema hiện hành của manifest closure.
pub const SCHEMA: u32 = 1;

/// Pack KR chứa sprite ô (tile/object) của map.
pub const TILE_PACK: &str = "FxTile";

/// Pack KR chứa sprite nhân vật (`Character/...`).
pub const CHARACTER_PACK: &str = "FxCharacter";

/// Entry KR chứa bảng tên obj **và** bảng animation của client:
/// `fx/ObjectTable/ObjectTable`.
pub const OBJECT_TABLE: &str = "fx/ObjectTable/ObjectTable";

/// Map V1 (SCOPE_V1 §1).
pub const V1_MAP: &str = "Village00";

/// Theme của map V1; entry `FxMap/Theme/Village`.
pub const V1_THEME: &str = "Village";

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
/// Định dạng: `u32 count`, rồi `count` bản ghi `(u32 name_len, tên, u32 value)`. Tên
/// trong bảng là cp949; ở đây decode lossy sang UTF-8 như phần đọc index pack
/// (`pack.rs`). Mọi tên dùng cho V1 đều là ASCII nên không mất thông tin.
///
/// `value` là **offset khối dữ liệu** của entry đó trong cùng payload: với
/// `Character/<Id>_A` đó là bảng animation của nhân vật (xem [`character_animations`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectTable {
    names: Vec<String>,
    values: Vec<u32>,
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

    /// Offset khối dữ liệu của entry theo **tên**; `None` nếu bảng không có tên đó.
    #[must_use]
    pub fn value_of(&self, name: &str) -> Option<u32> {
        let index = self.names.iter().position(|entry| entry == name)?;
        self.values.get(index).copied()
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
    let mut values = Vec::with_capacity(count);
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
        values.push(read_u32(data, &mut at, "offset dữ liệu")?);
        names.push(String::from_utf8_lossy(bytes).into_owned());
    }
    Ok(ObjectTable { names, values })
}

/// Một frame của state trình diễn, theo bảng animation của client trong ODT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationFrame {
    /// Chỉ số frame trong sprite.
    pub frame: u32,
    /// Thời lượng frame theo bảng client (đơn vị client, chưa chốt được là tick nào).
    pub duration: u32,
}

/// Một state trình diễn của client, theo bảng animation trong ODT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterAnimation {
    /// Nhãn state như client ghi, ví dụ `02 Faint`.
    pub label: String,
    /// Tên state, ví dụ `Faint` (phần sau dấu cách đầu của nhãn).
    pub state: String,
    /// Tiền tố entry pack, ví dụ `Character/C00-Faint`; client ghép thêm
    /// `Layer` + `Static`/`Adjust`.
    pub entry: String,
    /// Danh sách frame của state theo bảng client.
    pub frames: Vec<AnimationFrame>,
}

/// Đọc bảng animation của client cho một nhân vật từ payload ODT.
///
/// Khối dữ liệu của `Character/<Id>_A` trong `fx/ObjectTable/ObjectTable` là bảng
/// animation của nhân vật đó (R-094). Định dạng:
///
/// ```text
/// u32 số state
/// state × số state:
///   u32 độ dài nhãn, nhãn ("<số> <tên state>")
///   u32 độ dài tên, tên entry
///   u32 số frame, (u32 chỉ số frame, u32 thời lượng) × số frame
/// ```
///
/// Trả về vec rỗng nếu ODT không có khối cho nhân vật đó.
pub fn character_animations(
    data: &[u8],
    table: &ObjectTable,
    character: &str,
) -> Result<Vec<CharacterAnimation>, ClosureError> {
    let block = format!("Character/{character}_A");
    let Some(offset) = table.value_of(&block) else {
        return Ok(Vec::new());
    };
    parse_animation_block(data, offset)
}

/// Đọc khối bảng animation `block` bất kỳ trong ODT, ví dụ `Bomb/Bubble`, `Character/C00_A`.
///
/// Cùng định dạng với [`character_animations`] (R-094). Trả vec rỗng nếu ODT không có khối.
///
/// # Errors
///
/// Trả [`ClosureError::AnimationTable`] khi khối bị cắt cụt hoặc mang số rác.
pub fn animation_block(
    data: &[u8],
    table: &ObjectTable,
    block: &str,
) -> Result<Vec<CharacterAnimation>, ClosureError> {
    let Some(offset) = table.value_of(block) else {
        return Ok(Vec::new());
    };
    parse_animation_block(data, offset)
}

/// Đọc một khối bảng animation của ODT tại `offset`.
fn parse_animation_block(
    data: &[u8],
    offset: u32,
) -> Result<Vec<CharacterAnimation>, ClosureError> {
    let mut at = usize::try_from(offset).map_err(|_| ClosureError::AnimationTable {
        offset,
        what: "offset khối animation",
    })?;
    let count = read_u32(data, &mut at, "số state")?;
    // Mỗi state tối thiểu 12 byte; chặn số rác trước khi cấp phát.
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if count > data.len().saturating_sub(at) / 12 {
        return Err(ClosureError::AnimationTable {
            offset,
            what: "số state",
        });
    }
    let mut states = Vec::with_capacity(count);
    for _ in 0..count {
        let label = read_string(data, &mut at, 64, offset, "nhãn state")?;
        let entry = read_string(data, &mut at, 128, offset, "tên entry")?;
        let frame_count = read_u32(data, &mut at, "số frame")?;
        let frame_count = usize::try_from(frame_count).unwrap_or(usize::MAX);
        if frame_count > data.len().saturating_sub(at) / 8 {
            return Err(ClosureError::AnimationTable {
                offset,
                what: "số frame",
            });
        }
        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let frame = read_u32(data, &mut at, "chỉ số frame")?;
            let duration = read_u32(data, &mut at, "thời lượng frame")?;
            frames.push(AnimationFrame { frame, duration });
        }
        // Nhãn ghi dạng `<số> <tên state>`; tên state là phần sau dấu cách đầu.
        let state = label
            .split_once(' ')
            .map_or(label.as_str(), |(_, rest)| rest)
            .to_owned();
        states.push(CharacterAnimation {
            label,
            state,
            entry,
            frames,
        });
    }
    Ok(states)
}

/// Đọc một chuỗi có độ dài `u32` ở vị trí `at`; tối đa `max_len` byte.
fn read_string(
    data: &[u8],
    at: &mut usize,
    max_len: usize,
    offset: u32,
    what: &'static str,
) -> Result<String, ClosureError> {
    let len = usize::try_from(read_u32(data, at, what)?).unwrap_or(usize::MAX);
    if len == 0 || len > max_len {
        return Err(ClosureError::AnimationTable { offset, what });
    }
    let end = at
        .checked_add(len)
        .ok_or(ClosureError::AnimationTable { offset, what })?;
    let bytes = data
        .get(*at..end)
        .ok_or(ClosureError::AnimationTable { offset, what })?;
    *at = end;
    Ok(String::from_utf8_lossy(bytes).into_owned())
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

/// Bảng closure V1, nạp từ `data/assets/closure-v1.toml`.
///
/// # Errors
///
/// Trả [`AssetError`] khi bảng data sai quy tắc (dữ liệu nhúng trong crate được test
/// chặn nên trên build thường lỗi này không xảy ra).
pub fn v1_closure_table() -> Result<ClosureTable, AssetError> {
    ClosureTable::from_toml(boom_content::assets::CLOSURE_V1)
}

/// Nhân vật V1 (SCOPE_V1 §1).
const CHARACTERS: &[&str] = &["C00", "C01"];

/// Lớp sprite nhân vật: `Static` (nền) và `Adjust` (lớp ngoại hình điều chỉnh).
const CHARACTER_LAYERS: &[&str] = &["Static", "Adjust"];

/// Entry sprite nhân vật cho các state V1.
///
/// Nhãn bằng chứng của mỗi họ là **nhãn của vai trò** (xem bảng `character_families` ở
/// `data/assets/closure-v1.toml`), không phải kết quả của việc "có thấy tiền tố entry
/// trong ODT hay không": bảng ODT xác nhận **nhãn → entry**, còn mapping state V1 → clip là
/// quyết định của V1 (R-086).
fn character_entries(table: &ClosureTable) -> Vec<(String, ClosureRole, Status)> {
    let mut out = Vec::new();
    for character in CHARACTERS {
        for family in &table.character_families {
            for layer in CHARACTER_LAYERS {
                out.push((
                    format!(
                        "{CHARACTER_PACK}/Character/{character}-{}Layer{layer}",
                        family.family
                    ),
                    family.role,
                    family.status,
                ));
            }
        }
    }
    out
}

/// Dựng manifest closure của map V1 (Village00) từ thư mục chứa pack KR.
///
/// Thư mục phải có `<pack>.idx` và `<pack>.idd` của các pack KR (`FxMap`, `FxTile`, `fx`,
/// `FxCharacter`, `FxContents`, `FxDesktop`, `FxSound`).
///
/// Bảng closure lấy từ `data/assets/closure-v1.toml` (nguồn chuẩn duy nhất).
///
/// # Errors
///
/// Trả [`ClosureError`] khi đọc pack lỗi, bảng ODT sai định dạng, hoặc bảng data sai
/// quy tắc.
pub fn build_village00_manifest(pack_dir: &Path) -> Result<ClosureManifest, ClosureError> {
    let table = v1_closure_table()?;
    build_village00_manifest_with(pack_dir, &table)
}

/// Dựng manifest closure với bảng closure cho trước.
///
/// # Errors
///
/// Trả [`ClosureError`] khi đọc pack lỗi hoặc bảng ODT sai định dạng.
pub fn build_village00_manifest_with(
    pack_dir: &Path,
    closure: &ClosureTable,
) -> Result<ClosureManifest, ClosureError> {
    let map_pack = Pack::open(pack_dir, "FxMap", Build::Kr)?;
    let map = parse_map(&map_pack.read_named(&format!("Map/{V1_MAP}"))?, Build::Kr)?;
    let themes = theme_names(map_pack.index());
    let theme = resolve_theme(V1_MAP, &themes);
    let theme_in_pack = theme
        .as_deref()
        .is_some_and(|name| map_pack.index().find(&format!("Theme/{name}")).is_some());
    let preview_in_pack = map_pack.index().find(&format!("Etc/{V1_MAP}_mm")).is_some();
    drop(map_pack);

    let odt_payload = read_pack_entry(pack_dir, OBJECT_TABLE)?;
    let table = parse_object_table(&odt_payload)?;
    let tile_index = load_index(pack_dir, TILE_PACK, Build::Kr)?;
    let sound_index = load_index(pack_dir, "FxSound", Build::Kr)?;

    // Bảng animation của client cho hai nhân vật V1 (R-094): nguồn để **kiểm** entry
    // nhân vật mà V1 khai và để đối chiếu mapping state V1 → state của client.
    let mut animations: BTreeMap<String, Vec<CharacterAnimation>> = BTreeMap::new();
    for character in CHARACTERS {
        animations.insert(
            (*character).to_owned(),
            character_animations(&odt_payload, &table, character)?,
        );
    }

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

    // Entry cố định của một trận V1 (bảng `entries` của `data/assets/closure-v1.toml`).
    for mapping in &closure.entries {
        entries.push(ClosureEntry {
            entry: mapping.entry.clone(),
            role: mapping.role,
            status: mapping.status,
            bytes: 0,
            sha256: String::new(),
            map_id: None,
            object_name: None,
        });
    }
    for (entry, role, status) in character_entries(closure) {
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

    // Bảng state của client (ODT, R-094) phải có **họ entry mà V1 dùng**; lệch thì ghi
    // rõ lý do và ứng viên — không bỏ qua âm thầm.
    for character in CHARACTERS {
        let states = animations
            .get(*character)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for family in &closure.character_families {
            if !family.used_by_v1 {
                continue;
            }
            let prefix = format!("Character/{character}-{}", family.family);
            if states.iter().any(|state| state.entry == prefix) {
                continue;
            }
            unknown.push(ClosureUnknown {
                kind: UnknownKind::CharacterState,
                id: format!("{character}:{}", family.family),
                reason: format!(
                    "bảng state của client trong ODT không có entry {prefix} mà V1 dùng cho \
                     vai trò {}",
                    family.role.as_str()
                ),
                candidates: states
                    .iter()
                    .map(|state| state.entry.clone())
                    .take(8)
                    .collect(),
            });
        }
    }

    // Mapping state V1 → clip: đối chiếu **nhãn** của client trong bảng ODT. Bảng xác
    // nhận **nhãn → entry**; mapping state V1 → nhãn là **quyết định của V1** (R-086
    // `design`), nên lệch ở đây là lệch giữa quyết định của V1 và dữ liệu của client.
    //
    // Bảng mapping nằm ở `data/assets/closure-v1.toml` (mục `[[character_states]]`):
    //
    // | state V1 | state của client (ODT) | họ entry V1 dùng |
    // |---|---|---|
    // | `idle` (đứng yên) | `Blink` — overlay nháy mắt; tư thế đứng của client là frame 0 của `Walk<hướng>_A` (state `Stop <hướng>`) | `Blink_A` |
    // | `trapped` (bị nhốt) | `Faint` — animation 3 frame; state "bị khoá" của client (`Locked In`) dùng `Die` frame 0…1 giữ vô hạn, kèm bong bóng | `Faint` |
    //
    // Closure **kiểm** hai điều trên lúc dựng manifest: ODT phải có state của client tương
    // ứng và entry của state đó phải đúng họ mà V1 dùng. Lệch thì manifest ghi `unknown`
    // kèm lý do — không bỏ qua âm thầm.
    for character in CHARACTERS {
        let states = animations
            .get(*character)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for mapping in &closure.character_states {
            let v1_state = mapping.v1_state.as_str();
            let client_state = mapping.client_state.as_str();
            let family = mapping.family.as_str();
            let expected = format!("Character/{character}-{family}");
            let found = states.iter().find(|state| state.state == client_state);
            match found {
                Some(state) if state.entry == expected => {}
                Some(state) => unknown.push(ClosureUnknown {
                    kind: UnknownKind::CharacterState,
                    id: format!("{character}:{v1_state}"),
                    reason: format!(
                        "bảng state của client trong ODT có state `{}` nhưng entry là {} \
                         (V1 chọn họ {family} theo R-086 `design`)",
                        client_state, state.entry
                    ),
                    candidates: vec![state.entry.clone()],
                }),
                None => unknown.push(ClosureUnknown {
                    kind: UnknownKind::CharacterState,
                    id: format!("{character}:{v1_state}"),
                    reason: format!(
                        "bảng state của client trong ODT không có state `{client_state}`; \
                         V1 chọn họ {family} theo R-086 `design`"
                    ),
                    candidates: states
                        .iter()
                        .map(|state| state.entry.clone())
                        .take(8)
                        .collect(),
                }),
            }
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
    /// Khối bảng animation của client trong ODT cắt hoặc sai định dạng.
    AnimationTable {
        /// Offset của khối trong payload ODT.
        offset: u32,
        /// Field đang đọc khi lỗi xảy ra.
        what: &'static str,
    },
    /// Đường dẫn entry không có dạng `<pack>/<path>`.
    BadEntryPath(String),
    /// Bảng mapping trong `data/assets/` sai quy tắc.
    Assets(AssetError),
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
            Self::AnimationTable { offset, what } => {
                write!(
                    f,
                    "bảng animation của client cụt hoặc sai khi đọc {what} (offset {offset})"
                )
            }
            Self::BadEntryPath(path) => write!(f, "đường dẫn entry {path:?} thiếu phần pack"),
            Self::Assets(e) => write!(f, "bảng mapping closure sai quy tắc: {e}"),
        }
    }
}

impl std::error::Error for ClosureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pack(e) => Some(e),
            Self::Map(e) => Some(e),
            Self::Assets(e) => Some(e),
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

impl From<AssetError> for ClosureError {
    fn from(value: AssetError) -> Self {
        Self::Assets(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::assets::UiMapping;
    use boom_content::bundle::{UiRole, UiScreen, UiState};

    /// Bảng asset UI V1, nạp từ `data/assets/ui-v1.toml`.
    fn v1_ui_table() -> Vec<UiMapping> {
        boom_content::assets::UiTable::from_toml(boom_content::assets::UI_V1)
            .expect("bảng UI V1 trong data/assets/ phải hợp lệ")
            .elements
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
        let table = v1_ui_table();
        assert!(!table.is_empty());

        let mut ids: Vec<&str> = Vec::with_capacity(table.len());
        for asset in &table {
            assert!(!asset.id.is_empty(), "phần tử UI thiếu id");
            assert!(
                !ids.contains(&asset.id.as_str()),
                "phần tử UI {} bị lặp",
                asset.id
            );
            ids.push(asset.id.as_str());
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
            let of_screen: Vec<&UiMapping> = table
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
        let table = v1_ui_table();
        // Cả ba trạng thái đều phải có mặt trong bảng: `selected` (dùng được ngay),
        // `needs_review_text` (có chữ, chờ duyệt) và `excluded` (không dùng asset).
        assert!(table.iter().any(|asset| asset.state == UiState::Selected));
        assert!(
            table
                .iter()
                .any(|asset| asset.state == UiState::NeedsReviewText)
        );
        assert!(table.iter().any(|asset| asset.state == UiState::Excluded));
        for asset in &table {
            if TEXT_BEARING.contains(&asset.id.as_str()) {
                assert_ne!(
                    asset.state,
                    UiState::Selected,
                    "ảnh có chữ {} không được ở trạng thái hiển thị được",
                    asset.id
                );
            }
        }
    }

    /// Bảng animation của client nằm trong khối dữ liệu của ODT: đọc được state,
    /// entry và danh sách `(frame, thời lượng)` (không cần pack thật).
    #[test]
    fn animation_table_is_read_from_the_odt_data_block() {
        let label = "00 Blink";
        let entry = "Character/C00-Blink_A";
        let mut block: Vec<u8> = Vec::new();
        block.extend_from_slice(&1u32.to_le_bytes());
        block.extend_from_slice(&(label.len() as u32).to_le_bytes());
        block.extend_from_slice(label.as_bytes());
        block.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        block.extend_from_slice(entry.as_bytes());
        block.extend_from_slice(&2u32.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&5u32.to_le_bytes());
        block.extend_from_slice(&1u32.to_le_bytes());
        block.extend_from_slice(&15u32.to_le_bytes());

        // Bảng tên ODT: [số entry][(độ dài tên, tên, offset khối) × số].
        let name = "Character/C00_A";
        let table_len = 4 + 4 + name.len() + 4;
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(name.len() as u32).to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(&(table_len as u32).to_le_bytes());
        data.extend_from_slice(&block);

        let table = parse_object_table(&data).expect("đọc bảng tên ODT");
        assert_eq!(table.name(0), Some(name));
        assert_eq!(table.value_of(name), Some(table_len as u32));

        let states = character_animations(&data, &table, "C00").expect("đọc bảng animation");
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].label, label);
        assert_eq!(states[0].state, "Blink");
        assert_eq!(states[0].entry, entry);
        assert_eq!(states[0].frames.len(), 2);
        assert_eq!(states[0].frames[0].frame, 0);
        assert_eq!(states[0].frames[0].duration, 5);
        assert_eq!(states[0].frames[1].frame, 1);
        assert_eq!(states[0].frames[1].duration, 15);

        // Nhân vật không có khối thì không có state nào, không phải lỗi.
        assert!(
            character_animations(&data, &table, "C01")
                .expect("đọc bảng animation")
                .is_empty()
        );
    }

    /// Nhãn bằng chứng của sprite nhân vật phải **tách hai tầng** (AGENTS §4):
    ///
    /// - vai trò đã xác nhận bằng pack (đi, chết) ⇒ `confirmed`;
    /// - vai trò mà V1 **chọn** theo mapping của mình (đứng yên, bị nhốt) ⇒ `design`,
    ///   vì bảng ODT chỉ xác nhận **nhãn → entry**, không xác nhận state core V1 dùng
    ///   nhãn đó;
    /// - ứng viên mà V1 **không chọn** (`BlinkSmall`) ⇒ `candidate`.
    ///
    /// Không nhãn nào được nâng lên `confirmed` chỉ vì tiền tố entry có trong ODT.
    #[test]
    fn character_sprite_labels_separate_evidence_from_the_v1_choice() {
        let table = v1_closure_table().expect("bảng closure V1 hợp lệ");
        let status_of = |role: ClosureRole, family: &str| {
            character_entries(&table)
                .into_iter()
                .find(|(entry, entry_role, _)| {
                    *entry_role == role && entry.contains(&format!("-{family}LayerStatic"))
                })
                .map(|(_, _, status)| status)
        };

        assert_eq!(
            status_of(ClosureRole::CharacterWalk, "WalkDown_A"),
            Some(Status::Confirmed),
            "đi: xác nhận bằng tên entry và dãi frame trong pack (R-077)"
        );
        assert_eq!(
            status_of(ClosureRole::CharacterDead, "Die"),
            Some(Status::Confirmed),
            "chết: xác nhận bằng tên entry và dãi frame trong pack (R-077)"
        );
        assert_eq!(
            status_of(ClosureRole::CharacterIdle, "Blink_A"),
            Some(Status::Design),
            "đứng yên: mapping là quyết định của V1 (R-086), không phải bằng chứng client"
        );
        assert_eq!(
            status_of(ClosureRole::CharacterTrapped, "Faint"),
            Some(Status::Design),
            "bị nhốt: mapping là quyết định của V1 (R-086), không phải bằng chứng client"
        );
        assert_eq!(
            status_of(ClosureRole::CharacterIdle, "BlinkSmall_A"),
            Some(Status::Candidate),
            "ứng viên: V1 không chọn họ này cho đứng yên"
        );
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
        for asset in v1_ui_table() {
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
