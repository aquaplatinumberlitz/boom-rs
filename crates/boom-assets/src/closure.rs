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
//!   §1 — quy tắc tra theme.
//!
//! Quy tắc tra entry (RULES_V1 R-069, R-070):
//!
//! 1. ID tile và ID obj trong file `.map` là **chỉ số entry** của `fx/ObjectTable/ObjectTable`
//!    của build tương ứng (KR và VN dùng chung quy tắc, chỉ khác số thứ tự).
//! 2. Tên entry có dạng `<TileSet>_<Lớp>/<Tên>`; sprite của ô nằm ở pack `FxTile`, entry
//!    `<TileSet>/<Tên>`. Lớp (`Ground`/`Static`/`Box`/`Passable`/`Event`) quyết định va chạm,
//!    không đổi sprite.
//! 3. Theme của map là entry `FxMap/Theme/<Theme>`; preview là `FxMap/Etc/<Map>_mm`.
//!
//! Bảng chọn nội dung (nhân vật, item, vai trò) là **dữ liệu content** của V1, không phải
//! hằng số luật chơi: nó kế thừa vai trò đã fingerprint ở `kr_v1_content.json` và phạm vi
//! V1 ở [SCOPE_V1](../../../../docs/planning/SCOPE_V1.md) §1.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use boom_content::rules::Status;

use crate::legacy_map::{Build, LegacyMap, MapError, parse as parse_map};
use crate::pack::{Pack, PackError, load_index};

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
    /// State trình diễn chưa xác định được sprite nào.
    CharacterState,
}

impl UnknownKind {
    /// Tên loại dùng trong manifest.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MapCell => "map_cell",
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
             # Quy tắc tra entry: RULES_V1 R-069, R-070.\n\
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

/// Entry cần cho một trận V1 ngoài phần ô của map.
///
/// Vai trò lấy từ `docs/evidence/inventory/kr_v1_content.json`; nhân vật và item theo
/// [SCOPE_V1](../../../../docs/planning/SCOPE_V1.md) §1. `Fire00` (lửa) và `Bubble00`
/// (bong bóng bị nhốt) chưa có bằng chứng consumer nên để `candidate`.
const REQUIRED_ENTRIES: &[(&str, ClosureRole, Status)] = &[
    ("FxMap/Map/Village00", ClosureRole::Map, Status::Confirmed),
    ("FxMap/Theme/Village", ClosureRole::Theme, Status::Confirmed),
    (
        "FxMap/Etc/Village00_mm",
        ClosureRole::Preview,
        Status::Confirmed,
    ),
    ("FxSound/MP3/Village", ClosureRole::Music, Status::Confirmed),
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
    let map_entry = format!("FxMap/Map/{V1_MAP}");
    let map = parse_map(&read_pack_entry(pack_dir, &map_entry)?, Build::Kr)?;
    let table = parse_object_table(&read_pack_entry(pack_dir, OBJECT_TABLE)?)?;
    let tile_index = load_index(pack_dir, TILE_PACK, Build::Kr)?;

    let mut entries: Vec<ClosureEntry> = Vec::new();
    let mut unknown: Vec<ClosureUnknown> = Vec::new();
    let mut tilesets: Vec<String> = Vec::new();

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
}
