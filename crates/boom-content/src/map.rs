//! Map runtime.
//!
//! [`MapGrid`] giữ ID tile/obj đúng như file nguồn theo build (KR và VN có ID khác nhau, RULES_V1 R-004).
//! Nghĩa của ID (đi được, khối cứng, khối mềm) nằm trong [`CellClassifier`] riêng theo build;
//! ID chưa phân loại thành [`CellKind::Unknown`] và core từ chối dựng thế giới chứa ô Unknown.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::rules::{Source, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellIds {
    /// `None` khi file ghi `-1`.
    pub tile: Option<u32>,
    pub obj: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapGrid {
    pub width: u16,
    pub height: u16,
    /// Theo hàng: chỉ số `y * width + x`.
    pub cells: Vec<CellIds>,
    /// Tọa độ ô `(x, y)` theo thứ tự trong file.
    pub spawns: Vec<(u16, u16)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CellKind {
    Floor,
    Solid,
    Breakable,
    Unknown,
}

/// Bảng phân loại obj ID của một build/theme. Chỉ dựa trên obj; tile là nền.
#[derive(Clone, Debug, Default)]
pub struct CellClassifier {
    /// Loại của ô không có obj (`-1`); `Unknown` cho tới khi được xác nhận (RULES_V1 R-007).
    pub empty: Option<CellKind>,
    pub objects: BTreeMap<u32, CellKind>,
}

impl CellClassifier {
    pub fn classify(&self, ids: CellIds) -> CellKind {
        match ids.obj {
            None => self.empty.unwrap_or(CellKind::Unknown),
            Some(id) => self.objects.get(&id).copied().unwrap_or(CellKind::Unknown),
        }
    }
}

/// Loại ô được phép ghi trong bảng phân loại. Khác [`CellKind`] ở chỗ không có `Unknown`:
/// dữ liệu phải khai báo dứt khoát, ID lạ bị từ chối thay vì âm thầm thành `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellRule {
    Floor,
    Solid,
    Breakable,
}

impl CellRule {
    pub fn kind(self) -> CellKind {
        match self {
            Self::Floor => CellKind::Floor,
            Self::Solid => CellKind::Solid,
            Self::Breakable => CellKind::Breakable,
        }
    }
}

/// Một obj trong bảng phân loại: tên lấy từ ObjectTable và lớp quyết định loại ô.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapObjectRule {
    pub name: String,
    pub class: String,
}

/// Bảng phân loại ô cho một map/theme theo build, có nguồn/trạng thái như rules profile.
///
/// Định dạng TOML (schema 1): `map`, `build` (`kr`/`vn`), `width`, `height`, `source`, `status`,
/// `empty`, `[classes]` (tên lớp → [`CellRule`]), `[tiles]` (ID nền đã biết) và `[objects.<id>]`.
/// Obj ID là chỉ số entry trong `fx/ObjectTable/ObjectTable` của build tương ứng.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapProfile {
    pub schema: u32,
    pub profile_id: String,
    pub map: String,
    pub build: String,
    pub width: u16,
    pub height: u16,
    pub source: Source,
    pub status: Status,
    pub empty: CellRule,
    pub classes: BTreeMap<String, CellRule>,
    #[serde(default)]
    pub tiles: BTreeMap<u32, String>,
    pub objects: BTreeMap<u32, MapObjectRule>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MapProfileError {
    Parse(String),
    Schema(u32),
    Build(String),
    Status(Status),
    Dimensions(u16, u16),
    Size {
        profile: String,
        width: u16,
        height: u16,
    },
    MissingClass {
        id: u32,
        class: String,
    },
    UnknownTile(u32),
    UnknownObject(u32),
    Terrain(TerrainError),
}

impl fmt::Display for MapProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "TOML bảng phân loại sai: {e}"),
            Self::Schema(v) => write!(f, "schema bảng phân loại {v} không hỗ trợ"),
            Self::Build(v) => write!(f, "build {v:?} không hợp lệ (kr/vn)"),
            Self::Status(s) => write!(f, "trạng thái {s:?} không dùng cho bảng phân loại"),
            Self::Dimensions(w, h) => write!(f, "kích thước {w}x{h} không hợp lệ"),
            Self::Size {
                profile,
                width,
                height,
            } => write!(
                f,
                "bảng {profile} là {width}x{height} nhưng lưới không khớp"
            ),
            Self::MissingClass { id, class } => {
                write!(
                    f,
                    "obj {id} trỏ lớp {class:?} chưa khai báo trong [classes]"
                )
            }
            Self::UnknownTile(id) => write!(f, "tile {id} chưa có trong bảng [tiles]"),
            Self::UnknownObject(id) => write!(f, "obj {id} chưa có trong bảng [objects]"),
            Self::Terrain(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for MapProfileError {}

impl From<TerrainError> for MapProfileError {
    fn from(e: TerrainError) -> Self {
        Self::Terrain(e)
    }
}

impl MapProfile {
    /// Schema TOML hiện hành của bảng phân loại ô.
    pub const SCHEMA: u32 = 1;

    pub fn from_toml(text: &str) -> Result<Self, MapProfileError> {
        let profile: Self =
            toml::from_str(text).map_err(|e| MapProfileError::Parse(e.to_string()))?;
        if profile.schema != Self::SCHEMA {
            return Err(MapProfileError::Schema(profile.schema));
        }
        if !matches!(profile.build.as_str(), "kr" | "vn") {
            return Err(MapProfileError::Build(profile.build));
        }
        if profile.status == Status::Unknown {
            return Err(MapProfileError::Status(profile.status));
        }
        if profile.width == 0 || profile.height == 0 {
            return Err(MapProfileError::Dimensions(profile.width, profile.height));
        }
        for (&id, object) in &profile.objects {
            if !profile.classes.contains_key(&object.class) {
                return Err(MapProfileError::MissingClass {
                    id,
                    class: object.class.clone(),
                });
            }
        }
        Ok(profile)
    }

    pub fn classifier(&self) -> Result<CellClassifier, MapProfileError> {
        let mut objects = BTreeMap::new();
        for (&id, object) in &self.objects {
            let rule =
                self.classes
                    .get(&object.class)
                    .ok_or_else(|| MapProfileError::MissingClass {
                        id,
                        class: object.class.clone(),
                    })?;
            objects.insert(id, rule.kind());
        }
        Ok(CellClassifier {
            empty: Some(self.empty.kind()),
            objects,
        })
    }

    /// Phân loại lưới; từ chối kích thước khác profile, tile/obj ID lạ trước khi gọi core.
    pub fn classify(&self, grid: &MapGrid) -> Result<Terrain, MapProfileError> {
        if (grid.width, grid.height) != (self.width, self.height) {
            return Err(MapProfileError::Size {
                profile: self.profile_id.clone(),
                width: self.width,
                height: self.height,
            });
        }
        for cell in &grid.cells {
            if let Some(tile) = cell.tile
                && !self.tiles.contains_key(&tile)
            {
                return Err(MapProfileError::UnknownTile(tile));
            }
            if let Some(obj) = cell.obj
                && !self.objects.contains_key(&obj)
            {
                return Err(MapProfileError::UnknownObject(obj));
            }
        }
        Ok(grid.classify(&self.classifier()?)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terrain {
    pub width: u16,
    pub height: u16,
    pub kinds: Vec<CellKind>,
    pub spawns: Vec<(u16, u16)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TerrainError {
    Empty,
    RaggedRow(usize),
    BadChar(char),
    DuplicateSpawn(u8),
    MissingSpawn(u8),
    SizeMismatch,
}

impl fmt::Display for TerrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "bản đồ rỗng"),
            Self::RaggedRow(y) => write!(f, "hàng {y} khác độ dài"),
            Self::BadChar(c) => write!(f, "ký tự bản đồ không hợp lệ {c:?}"),
            Self::DuplicateSpawn(n) => write!(f, "spawn {n} xuất hiện hai lần"),
            Self::MissingSpawn(n) => write!(f, "thiếu spawn {n}"),
            Self::SizeMismatch => write!(f, "số ô không khớp kích thước"),
        }
    }
}

impl std::error::Error for TerrainError {}

impl MapGrid {
    pub fn classify(&self, classifier: &CellClassifier) -> Result<Terrain, TerrainError> {
        if self.cells.len() != usize::from(self.width) * usize::from(self.height) {
            return Err(TerrainError::SizeMismatch);
        }
        Ok(Terrain {
            width: self.width,
            height: self.height,
            kinds: self.cells.iter().map(|&c| classifier.classify(c)).collect(),
            spawns: self.spawns.clone(),
        })
    }
}

impl Terrain {
    /// Bản đồ ASCII cho fixture: `#` khối cứng, `+` khối mềm, `.` nền, `?` chưa biết,
    /// chữ số `0`–`9` là spawn theo thứ tự (ô nền).
    pub fn from_ascii<S: AsRef<str>>(rows: &[S]) -> Result<Self, TerrainError> {
        let height = rows.len();
        let width = rows.first().map_or(0, |r| r.as_ref().chars().count());
        if width == 0 || height == 0 {
            return Err(TerrainError::Empty);
        }
        let mut kinds = Vec::with_capacity(width * height);
        let mut spawns: BTreeMap<u8, (u16, u16)> = BTreeMap::new();
        for (y, row) in rows.iter().enumerate() {
            let row = row.as_ref();
            if row.chars().count() != width {
                return Err(TerrainError::RaggedRow(y));
            }
            for (x, c) in row.chars().enumerate() {
                let kind = match c {
                    '#' => CellKind::Solid,
                    '+' => CellKind::Breakable,
                    '.' => CellKind::Floor,
                    '?' => CellKind::Unknown,
                    '0'..='9' => {
                        let n = u8::try_from(c.to_digit(10).expect("chữ số")).expect("0..9");
                        let pos = (to_u16(x)?, to_u16(y)?);
                        if spawns.insert(n, pos).is_some() {
                            return Err(TerrainError::DuplicateSpawn(n));
                        }
                        CellKind::Floor
                    }
                    other => return Err(TerrainError::BadChar(other)),
                };
                kinds.push(kind);
            }
        }
        for (expected, &n) in spawns.keys().enumerate() {
            if usize::from(n) != expected {
                return Err(TerrainError::MissingSpawn(
                    u8::try_from(expected).expect("0..9"),
                ));
            }
        }
        Ok(Self {
            width: to_u16(width)?,
            height: to_u16(height)?,
            kinds,
            spawns: spawns.into_values().collect(),
        })
    }

    pub fn kind(&self, x: i32, y: i32) -> Option<CellKind> {
        let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
        let w = usize::from(self.width);
        if x >= w || y >= usize::from(self.height) {
            return None;
        }
        self.kinds.get(y * w + x).copied()
    }

    pub fn has_unknown(&self) -> bool {
        self.kinds.contains(&CellKind::Unknown)
    }
}

fn to_u16(v: usize) -> Result<u16, TerrainError> {
    u16::try_from(v).map_err(|_| TerrainError::SizeMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_fixture() {
        let t = Terrain::from_ascii(&["#####", "#0+1#", "#####"]).unwrap();
        assert_eq!((t.width, t.height), (5, 3));
        assert_eq!(t.spawns, vec![(1, 1), (3, 1)]);
        assert_eq!(t.kind(2, 1), Some(CellKind::Breakable));
        assert_eq!(t.kind(5, 1), None);
        assert_eq!(t.kind(-1, 0), None);
        assert!(!t.has_unknown());
    }

    #[test]
    fn ascii_rejects_gaps_in_spawn_numbers() {
        assert_eq!(
            Terrain::from_ascii(&["0.2"]),
            Err(TerrainError::MissingSpawn(1))
        );
    }

    #[test]
    fn village00_profile_parses_and_classifies_keys() {
        let profile =
            MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml")).unwrap();
        assert_eq!(profile.map, "Village00");
        assert_eq!(profile.status, Status::Confirmed);
        let classifier = profile.classifier().unwrap();
        let cell = |obj| CellIds {
            tile: Some(1943),
            obj,
        };
        assert_eq!(classifier.classify(cell(None)), CellKind::Floor);
        assert_eq!(classifier.classify(cell(Some(1958))), CellKind::Solid);
        assert_eq!(classifier.classify(cell(Some(1933))), CellKind::Breakable);
    }

    #[test]
    fn map_profile_rejects_unknown_ids_instead_of_guessing() {
        let profile =
            MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml")).unwrap();
        let mut grid = MapGrid {
            width: 15,
            height: 13,
            cells: vec![
                CellIds {
                    tile: Some(1943),
                    obj: None,
                };
                15 * 13
            ],
            spawns: vec![(0, 0)],
        };
        grid.cells[0].obj = Some(999_999);
        assert_eq!(
            profile.classify(&grid),
            Err(MapProfileError::UnknownObject(999_999))
        );
    }

    #[test]
    fn unclassified_ids_stay_unknown() {
        let grid = MapGrid {
            width: 2,
            height: 1,
            cells: vec![
                CellIds {
                    tile: Some(1),
                    obj: None,
                },
                CellIds {
                    tile: Some(1),
                    obj: Some(7),
                },
            ],
            spawns: vec![(0, 0)],
        };
        let t = grid.classify(&CellClassifier::default()).unwrap();
        assert_eq!(t.kinds, vec![CellKind::Unknown, CellKind::Unknown]);
        let mut known = CellClassifier {
            empty: Some(CellKind::Floor),
            ..Default::default()
        };
        known.objects.insert(7, CellKind::Solid);
        let t = grid.classify(&known).unwrap();
        assert_eq!(t.kinds, vec![CellKind::Floor, CellKind::Solid]);
    }
}
