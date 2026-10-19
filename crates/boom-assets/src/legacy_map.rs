//! Đọc file `.map` của client gốc (định dạng ở `docs/specification/assets/MAPS.md`).
//!
//! KR và VN khác nhau ở header: KR có thêm một dword trước cặp `60000, 22000`, nên tọa độ spawn
//! bắt đầu ở +80 thay vì +76 (RULES_V1 R-003). Phần sau spawn cùng bố cục.
//! Script/đuôi sau placement chưa có consumer đã biết nên được giữ nguyên dạng byte.

use std::fmt;

use boom_content::map::{CellIds, MapGrid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Kr,
    Vn,
}

impl Build {
    fn spawn_offset(self) -> usize {
        match self {
            Build::Kr => 80,
            Build::Vn => 76,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropEntry {
    /// Tên item như trong file (cp949, đã bỏ NUL cuối).
    pub name: Vec<u8>,
    pub weight: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub name: Vec<u8>,
    pub x: u8,
    pub y: u8,
    /// Phần trăm theo MAPS §1.1 (độ tin cậy trung bình).
    pub prob: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyMap {
    pub version: i32,
    /// Các dword header trước danh sách spawn, giữ nguyên để đối chiếu field chưa rõ nghĩa.
    pub header: Vec<i32>,
    pub grid: MapGrid,
    /// Lớp overlay của version 1001 (`None` = `-1`); rỗng với version 1000.
    pub overlay: Vec<Option<u32>>,
    pub drops: Vec<DropEntry>,
    /// Trường L2 sau bảng rơi (100 ở đa số file; nghi là cơ số %).
    pub drop_base: u32,
    pub placements: Vec<Placement>,
    /// Byte còn lại (script/đuôi chưa có consumer).
    pub tail: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MapError {
    Truncated { at: usize, need: usize },
    Version(i32),
    Dimensions(i32, i32),
    Count { what: &'static str, value: i32 },
    SpawnOutside(i32, i32),
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { at, need } => write!(f, "file cụt tại byte {at}, cần thêm {need}"),
            Self::Version(v) => write!(f, "version map {v} không hỗ trợ (1000/1001)"),
            Self::Dimensions(w, h) => write!(f, "kích thước {w}x{h} không hợp lệ"),
            Self::Count { what, value } => write!(f, "số lượng {what} = {value} không hợp lệ"),
            Self::SpawnOutside(x, y) => write!(f, "spawn ({x},{y}) nằm ngoài lưới"),
        }
    }
}

impl std::error::Error for MapError {}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], MapError> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len());
        let Some(end) = end else {
            return Err(MapError::Truncated {
                at: self.at,
                need: n,
            });
        };
        let out = &self.data[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn i32(&mut self) -> Result<i32, MapError> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u8(&mut self) -> Result<u8, MapError> {
        Ok(self.take(1)?[0])
    }

    fn count(&mut self, what: &'static str, max: i32) -> Result<usize, MapError> {
        let value = self.i32()?;
        if !(0..=max).contains(&value) {
            return Err(MapError::Count { what, value });
        }
        Ok(usize::try_from(value).expect("đã kiểm không âm"))
    }

    /// ID ô; `-1` (hoặc số âm khác) là trống.
    fn id(&mut self) -> Result<Option<u32>, MapError> {
        Ok(u32::try_from(self.i32()?).ok())
    }

    fn name(&mut self) -> Result<Vec<u8>, MapError> {
        let len = self.count("độ dài tên", 4096)?;
        let mut name = self.take(len)?.to_vec();
        while name.last() == Some(&0) {
            name.pop();
        }
        Ok(name)
    }
}

pub fn parse(data: &[u8], build: Build) -> Result<LegacyMap, MapError> {
    let mut c = Cursor { data, at: 0 };
    let header = (0..build.spawn_offset() / 4)
        .map(|_| c.i32())
        .collect::<Result<Vec<_>, _>>()?;
    let version = header[0];
    let per_cell = match version {
        1000 => 2,
        1001 => 3,
        v => return Err(MapError::Version(v)),
    };
    let (w, h) = (header[3], header[4]);
    if !(1..=255).contains(&w) || !(1..=255).contains(&h) {
        return Err(MapError::Dimensions(w, h));
    }
    let width = u16::try_from(w).expect("đã kiểm miền");
    let height = u16::try_from(h).expect("đã kiểm miền");
    let spawn_count = header[5];
    if !(0..=64).contains(&spawn_count) {
        return Err(MapError::Count {
            what: "spawn",
            value: spawn_count,
        });
    }
    let mut spawns = Vec::new();
    for _ in 0..spawn_count {
        let (x, y) = (c.i32()?, c.i32()?);
        if !(0..w).contains(&x) || !(0..h).contains(&y) {
            return Err(MapError::SpawnOutside(x, y));
        }
        spawns.push((
            u16::try_from(x).expect("trong lưới"),
            u16::try_from(y).expect("trong lưới"),
        ));
    }
    let rects = c.count("rect", 20)?;
    c.take(rects * 16)?;
    let n = usize::from(width) * usize::from(height);
    let mut cells = Vec::with_capacity(n);
    let mut overlay = Vec::new();
    for _ in 0..n {
        cells.push(CellIds {
            tile: c.id()?,
            obj: c.id()?,
        });
        if per_cell == 3 {
            overlay.push(c.id()?);
        }
    }
    let drop_count = c.count("bảng rơi", 1024)?;
    let mut drops = Vec::with_capacity(drop_count);
    for _ in 0..drop_count {
        let name = c.name()?;
        let weight = c.count("trọng số", i32::MAX)?;
        drops.push(DropEntry {
            name,
            weight: u32::try_from(weight).expect("đã kiểm miền"),
        });
    }
    let drop_base = u32::try_from(c.count("L2", i32::MAX)?).expect("đã kiểm miền");
    let placement_count = c.count("placement", 1024)?;
    let mut placements = Vec::with_capacity(placement_count);
    for _ in 0..placement_count {
        let name = c.name()?;
        placements.push(Placement {
            name,
            x: c.u8()?,
            y: c.u8()?,
            prob: c.u8()?,
        });
    }
    Ok(LegacyMap {
        version,
        header,
        grid: MapGrid {
            width,
            height,
            cells,
            spawns,
        },
        overlay,
        drops,
        drop_base,
        placements,
        tail: data[c.at..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dựng file map tổng hợp 3x2 theo bố cục MAPS §1.1.
    fn synthetic(build: Build, version: i32) -> Vec<u8> {
        let mut words: Vec<i32> = vec![version, 8, 1, 3, 2, 2, 1, 0, 0, 0, 0, 60000, 22000];
        if build == Build::Kr {
            words.insert(11, 0);
        }
        words.resize(build.spawn_offset() / 4, 0);
        words.extend([0, 0, 2, 1]); // spawn (0,0), (2,1)
        words.push(0); // rect
        for i in 0..6 {
            words.extend([100 + i, if i == 4 { 7 } else { -1 }]);
            if version == 1001 {
                words.push(-1);
            }
        }
        let mut out: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        out.extend(1i32.to_le_bytes());
        out.extend(4i32.to_le_bytes());
        out.extend(b"Bom\0");
        out.extend(3650i32.to_le_bytes());
        out.extend(100i32.to_le_bytes());
        out.extend(1i32.to_le_bytes());
        out.extend(4i32.to_le_bytes());
        out.extend(b"Shoe");
        out.extend([1, 0, 30]);
        out.extend([0; 8]);
        out
    }

    #[test]
    fn parses_both_builds() {
        for build in [Build::Kr, Build::Vn] {
            for version in [1000, 1001] {
                let m = parse(&synthetic(build, version), build).unwrap();
                assert_eq!((m.grid.width, m.grid.height), (3, 2));
                assert_eq!(m.grid.spawns, vec![(0, 0), (2, 1)]);
                assert_eq!(m.grid.cells[4].obj, Some(7));
                assert_eq!(m.grid.cells[0].obj, None);
                assert_eq!(m.overlay.len(), if version == 1001 { 6 } else { 0 });
                assert_eq!(
                    m.drops,
                    vec![DropEntry {
                        name: b"Bom".to_vec(),
                        weight: 3650
                    }]
                );
                assert_eq!(m.drop_base, 100);
                assert_eq!(m.placements[0].prob, 30);
                assert_eq!(m.tail.len(), 8);
            }
        }
    }

    #[test]
    fn wrong_build_is_detected() {
        // Đọc file KR theo header VN làm lệch spawn: phải báo lỗi thay vì ra lưới rác.
        assert!(parse(&synthetic(Build::Kr, 1000), Build::Vn).is_err());
    }

    #[test]
    fn truncated_file_is_an_error() {
        let data = synthetic(Build::Kr, 1000);
        assert!(matches!(
            parse(&data[..100], Build::Kr),
            Err(MapError::Truncated { .. })
        ));
    }
}
