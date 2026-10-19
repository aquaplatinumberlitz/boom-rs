//! Map runtime.
//!
//! [`MapGrid`] giữ ID tile/obj đúng như file nguồn theo build (KR và VN có ID khác nhau, RULES_V1 R-004).
//! Nghĩa của ID (đi được, khối cứng, khối mềm) nằm trong [`CellClassifier`] riêng theo build;
//! ID chưa phân loại thành [`CellKind::Unknown`] và core từ chối dựng thế giới chứa ô Unknown.

use std::collections::BTreeMap;
use std::fmt;

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
