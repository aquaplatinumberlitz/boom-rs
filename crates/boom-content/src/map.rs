//! Map runtime.
//!
//! [`MapGrid`] giữ ID tile/obj đúng như file nguồn theo build (KR và VN có ID khác nhau, RULES_V1 R-004).
//! Nghĩa của ID (đi được, khối cứng, khối mềm) nằm trong [`CellClassifier`] riêng theo build;
//! ID chưa phân loại thành [`CellKind::Unknown`] và core từ chối dựng thế giới chứa ô Unknown.

use std::collections::BTreeMap;
use std::collections::VecDeque;
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

/// Ô **đi được** ngay: nền trống.
///
/// `Breakable` là gạch phá được — phải phá trước khi đi qua nên không tính là đường
/// đi tức thời; `Solid` là vật cản; `Unknown` là ô chưa phân loại.
#[must_use]
pub fn is_walkable(kind: CellKind) -> bool {
    matches!(kind, CellKind::Floor)
}

/// Kết quả kiểm tra một spawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnInfo {
    /// Toạ độ ô của spawn.
    pub position: (u16, u16),
    /// Loại ô nằm dưới spawn.
    pub kind: CellKind,
    /// Số ô trống đi được từ spawn (không tính ô của chính spawn).
    pub reachable: u16,
    /// Số ô trống cách ≥ khoảng cách tối thiểu.
    pub far_enough: u16,
    /// Khoảng cách lớn nhất tới ô trống từ spawn.
    pub max_distance: u16,
}

/// Báo cáo kiểm tra spawn của một map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpawnReport {
    /// Khoảng cách tối thiểu mà nghiệm thu yêu cầu (ô).
    pub min_distance: u16,
    /// Kết quả từng spawn, theo thứ tự spawn của map.
    pub spawns: Vec<SpawnInfo>,
}

/// Lỗi kiểm tra spawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnError {
    /// Spawn nằm ngoài lưới.
    OutOfGrid {
        /// Toạ độ spawn.
        position: (u16, u16),
    },
    /// Hai spawn trùng nhau.
    Duplicate {
        /// Toạ độ spawn bị lặp.
        position: (u16, u16),
    },
    /// Spawn nằm trên ô chặn (không phải nền trống).
    Blocked {
        /// Toạ độ spawn.
        position: (u16, u16),
        /// Loại ô nằm dưới spawn.
        kind: CellKind,
    },
    /// Spawn không có đường đi tới ô trống đủ xa.
    NoEscape {
        /// Toạ độ spawn.
        position: (u16, u16),
        /// Số ô trống đi được từ spawn.
        reachable: u16,
        /// Khoảng cách lớn nhất tới ô trống.
        max_distance: u16,
    },
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfGrid { position } => {
                write!(f, "spawn ({}, {}) nằm ngoài lưới", position.0, position.1)
            }
            Self::Duplicate { position } => {
                write!(f, "spawn ({}, {}) bị lặp", position.0, position.1)
            }
            Self::Blocked { position, kind } => write!(
                f,
                "spawn ({}, {}) nằm trên ô {kind:?} — không phải nền trống",
                position.0, position.1
            ),
            Self::NoEscape {
                position,
                reachable,
                max_distance,
            } => write!(
                f,
                "spawn ({}, {}) không có ô trống đủ xa: đi được {reachable} ô, xa nhất {max_distance} ô",
                position.0, position.1
            ),
        }
    }
}

impl std::error::Error for SpawnError {}

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

    /// Kiểm nghiệm thu spawn của map (gate G1 content/map).
    ///
    /// Mỗi spawn phải thỏa **bốn** điều kiện:
    ///
    /// 1. nằm trong lưới;
    /// 2. không trùng spawn khác;
    /// 3. không nằm trên ô chặn — ô dưới spawn phải là nền trống (`Floor`); gạch phá
    ///    được (`Breakable`) và vật cản (`Solid`) đều không phải nơi đứng được ngay;
    /// 4. có **đường thoát**: BFS trên lưới địa hình phải tới được ít nhất một ô trống
    ///    cách ≥ `min_distance` ô (chỉ đi qua ô `Floor`).
    ///
    /// Lỗi đầu tiên trả về là lỗi của cả bản kiểm, kèm số đo để điều tra.
    pub fn check_spawns(&self, min_distance: u16) -> Result<SpawnReport, SpawnError> {
        let mut seen: Vec<(u16, u16)> = Vec::with_capacity(self.spawns.len());
        let mut spawns = Vec::with_capacity(self.spawns.len());
        for &(x, y) in &self.spawns {
            let position = (x, y);
            let kind = self
                .kind(i32::from(x), i32::from(y))
                .ok_or(SpawnError::OutOfGrid { position })?;
            if seen.contains(&position) {
                return Err(SpawnError::Duplicate { position });
            }
            seen.push(position);
            if !is_walkable(kind) {
                return Err(SpawnError::Blocked { position, kind });
            }
            let (reachable, far_enough, max_distance) = self.escape_from(x, y, min_distance);
            if far_enough == 0 {
                return Err(SpawnError::NoEscape {
                    position,
                    reachable,
                    max_distance,
                });
            }
            spawns.push(SpawnInfo {
                position,
                kind,
                reachable,
                far_enough,
                max_distance,
            });
        }
        Ok(SpawnReport {
            min_distance,
            spawns,
        })
    }

    /// BFS từ một ô nền trống: (số ô trống đi được, số ô trống cách ≥ `min_distance`,
    /// khoảng cách lớn nhất tới ô trống).
    fn escape_from(&self, x: u16, y: u16, min_distance: u16) -> (u16, u16, u16) {
        let width = usize::from(self.width);
        let mut distance: Vec<u16> = vec![u16::MAX; self.kinds.len()];
        let start = usize::from(y) * width + usize::from(x);
        distance[start] = 0;
        let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
        queue.push_back((i32::from(x), i32::from(y)));
        let mut reachable = 0u16;
        let mut far_enough = 0u16;
        let mut max_distance = 0u16;
        while let Some((cx, cy)) = queue.pop_front() {
            let current = distance[usize::try_from(cy).expect("không âm") * width
                + usize::try_from(cx).expect("không âm")];
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nx = cx + dx;
                let ny = cy + dy;
                let Some(kind) = self.kind(nx, ny) else {
                    continue;
                };
                if !is_walkable(kind) {
                    continue;
                }
                let index = usize::try_from(ny).expect("không âm") * width
                    + usize::try_from(nx).expect("không âm");
                if distance[index] != u16::MAX {
                    continue;
                }
                let next = current + 1;
                distance[index] = next;
                reachable += 1;
                if next >= min_distance {
                    far_enough += 1;
                }
                max_distance = max_distance.max(next);
                queue.push_back((nx, ny));
            }
        }
        (reachable, far_enough, max_distance)
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

    /// Spawn có đường thoát: ô trống cách ≥ 2 ô ⇒ đạt nghiệm thu.
    #[test]
    fn spawn_with_a_free_cell_two_steps_away_passes() {
        let terrain = Terrain::from_ascii(&["#####", "#0..#", "#####"]).unwrap();
        let report = terrain.check_spawns(2).expect("spawn có đường thoát");
        assert_eq!(report.min_distance, 2);
        assert_eq!(report.spawns.len(), 1);
        let spawn = &report.spawns[0];
        assert_eq!(spawn.position, (1, 1));
        assert_eq!(spawn.kind, CellKind::Floor);
        assert_eq!(spawn.reachable, 2, "đi được hai ô trống");
        assert_eq!(spawn.far_enough, 1, "một ô trống cách ≥ 2 ô");
        assert_eq!(spawn.max_distance, 2);
    }

    /// Spawn bị chặn (ô dưới spawn là gạch) thì báo lỗi, không bỏ qua âm thầm.
    #[test]
    fn spawn_on_a_blocked_cell_is_rejected() {
        let terrain = Terrain {
            width: 3,
            height: 3,
            kinds: vec![
                CellKind::Solid,
                CellKind::Solid,
                CellKind::Solid,
                CellKind::Solid,
                CellKind::Breakable,
                CellKind::Solid,
                CellKind::Solid,
                CellKind::Solid,
                CellKind::Solid,
            ],
            spawns: vec![(1, 1)],
        };
        assert_eq!(
            terrain.check_spawns(2),
            Err(SpawnError::Blocked {
                position: (1, 1),
                kind: CellKind::Breakable
            })
        );
    }

    /// Hai spawn trùng ô là lỗi.
    #[test]
    fn duplicate_spawns_are_rejected() {
        let terrain = Terrain {
            width: 3,
            height: 3,
            kinds: vec![CellKind::Floor; 9],
            spawns: vec![(1, 1), (1, 1)],
        };
        assert_eq!(
            terrain.check_spawns(2),
            Err(SpawnError::Duplicate { position: (1, 1) })
        );
    }

    /// Spawn không có ô trống đủ xa thì báo lỗi kèm số đo.
    #[test]
    fn spawn_without_an_escape_route_is_rejected() {
        let terrain = Terrain::from_ascii(&["#####", "#0#.#", "#####"]).unwrap();
        assert_eq!(
            terrain.check_spawns(2),
            Err(SpawnError::NoEscape {
                position: (1, 1),
                reachable: 0,
                max_distance: 0
            })
        );
    }

    /// Spawn nằm ngoài lưới là lỗi.
    #[test]
    fn spawn_outside_the_grid_is_rejected() {
        let terrain = Terrain {
            width: 2,
            height: 2,
            kinds: vec![CellKind::Floor; 4],
            spawns: vec![(5, 5)],
        };
        assert_eq!(
            terrain.check_spawns(2),
            Err(SpawnError::OutOfGrid { position: (5, 5) })
        );
    }

    /// Bốn spawn ở bốn góc, mỗi góc có đường thoát: giống hình học Village00.
    #[test]
    fn four_spawns_at_the_corners_all_have_an_escape_route() {
        let terrain = Terrain::from_ascii(&["0..1", "....", "....", "2..3"]).unwrap();
        let report = terrain
            .check_spawns(2)
            .expect("cả bốn spawn đều có đường thoát");
        assert_eq!(report.spawns.len(), 4);
        for spawn in &report.spawns {
            assert!(
                spawn.far_enough >= 1,
                "spawn {:?} không có ô trống đủ xa",
                spawn.position
            );
        }
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
