//! Đọc pack `.idx/.idd` của client gốc (định dạng ở `docs/specification/assets/PACKS.md`).
//!
//! Viết lại từ mô tả format và dữ liệu đã kiểm; không copy code legacy. Ba lớp:
//!
//! 1. `.idx` được XOR-chain bằng khóa pack. KR lấy khóa ở dword đầu file; VN dùng bảng khóa
//!    theo tên pack đã RE. Sau khi giải mã là `version, count` và danh sách entry.
//! 2. Entry có `key != 0` được giải mã bằng XOR-chain có đa thức, rồi kiểm CRC32.
//! 3. Entry tên chứa `_mm` là `u32 comp_len, u32 raw_len, zlib` và được inflate.
//!
//! Tên entry trong index là cp949; ở đây decode lossy sang UTF-8 (đường dẫn V1 đều ASCII).
//! Lỗi format/CRC/zlib trả `PackError`, không panic và không bỏ qua entry hỏng.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::legacy_map::Build;

/// Hằng số trộn của `PackFS__DecryptEntry` (PACKS §Entry).
const ENTRY_CRYPT_MAGIC: u32 = 0x9C6C_95CE;

/// Khóa `.idx` của các pack VN đã RE (PACKS §Pack). KR đọc khóa từ dword đầu `.idx`.
const VN_PACK_KEYS: &[(&str, u32)] = &[
    ("fx", 1_906_092_848),
    ("charte", 95_551_996),
    ("emote", 95_551_996),
    ("fxasset", 95_551_996),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackEntry {
    /// Tên trong index, decode lossy từ cp949.
    pub name: String,
    pub offset: u32,
    pub size: u32,
    /// 0 = dữ liệu thô/zlib; khác 0 = entry được mã hóa.
    pub key: u32,
    /// CRC32 với `init = key`; 0/1 nghĩa là index kiểu cũ không có CRC.
    pub crc: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackIndex {
    /// `None` với index kiểu cũ (dword đầu là count, không có version/CRC).
    pub version: Option<u32>,
    pub entries: Vec<PackEntry>,
}

impl PackIndex {
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&PackEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }
}

/// Pack đã nạp cả index và toàn bộ `.idd`; `read` giải mã từng entry khi cần.
pub struct Pack {
    index: PackIndex,
    idd: Vec<u8>,
}

impl Pack {
    /// Nạp `<root>/<stem>.idx` và `<root>/<stem>.idd` theo build.
    pub fn open(root: &Path, stem: &str, build: Build) -> Result<Self, PackError> {
        let index = load_index(root, stem, build)?;
        let path = root.join(format!("{stem}.idd"));
        let idd = fs::read(&path).map_err(|source| PackError::Io { path, source })?;
        Ok(Self { index, idd })
    }

    #[must_use]
    pub fn index(&self) -> &PackIndex {
        &self.index
    }

    /// Đọc và giải mã một entry đã tra trong index.
    pub fn read(&self, entry: &PackEntry) -> Result<Vec<u8>, PackError> {
        let offset = entry.offset as usize;
        let size = entry.size as usize;
        let out_of_range = || PackError::EntryOutOfRange {
            name: entry.name.clone(),
            offset,
            size,
            idd_len: self.idd.len(),
        };
        let end = offset.checked_add(size).ok_or_else(out_of_range)?;
        let stored = self.idd.get(offset..end).ok_or_else(out_of_range)?;
        decode_entry(&entry.name, stored, entry.key, entry.crc)
    }

    pub fn read_named(&self, name: &str) -> Result<Vec<u8>, PackError> {
        let entry = self
            .index
            .find(name)
            .ok_or_else(|| PackError::UnknownEntry {
                name: name.to_owned(),
            })?;
        self.read(entry)
    }
}

/// Đọc và giải mã `.idx`, chưa cần `.idd`.
pub fn load_index(root: &Path, stem: &str, build: Build) -> Result<PackIndex, PackError> {
    let path = root.join(format!("{stem}.idx"));
    let data = fs::read(&path).map_err(|source| PackError::Io { path, source })?;
    let key = match build {
        Build::Kr => key_from_index(&data)?,
        Build::Vn => vn_pack_key(stem).ok_or_else(|| PackError::UnknownVnPack {
            stem: stem.to_owned(),
        })?,
    };
    parse_index(&decrypt_index(&data, key))
}

/// Khóa KR: dword đầu `.idx` (PACKS §Pack).
pub fn key_from_index(data: &[u8]) -> Result<u32, PackError> {
    let bytes = data.get(..4).ok_or(PackError::IndexTooShort)?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("đã kiểm 4 byte"),
    ))
}

/// Khóa `.idx` của pack VN theo bảng đã RE.
#[must_use]
pub fn vn_pack_key(stem: &str) -> Option<u32> {
    VN_PACK_KEYS
        .iter()
        .find(|(name, _)| *name == stem)
        .map(|(_, key)| *key)
}

/// XOR-chain toàn bộ `.idx`; byte lẻ cuối dùng dword cuối đã giải.
#[must_use]
pub fn decrypt_index(data: &[u8], key: u32) -> Vec<u8> {
    let words = data.len() / 4;
    let mut out = Vec::with_capacity(data.len());
    let mut previous = key;
    for index in 0..words {
        let start = index * 4;
        let current =
            u32::from_le_bytes(data[start..start + 4].try_into().expect("đã kiểm 4 byte"));
        previous = if index == 0 {
            current ^ key
        } else {
            previous ^ current
        };
        out.extend_from_slice(&previous.to_le_bytes());
    }
    let tail = &data[words * 4..];
    for (index, &byte) in tail.iter().enumerate() {
        out.push(byte ^ ((previous >> (8 * index)) & 0xFF) as u8);
    }
    out
}

/// Giải mã một entry `.idd` có `key != 0` (PACKS §Entry).
#[must_use]
pub fn decrypt_entry(data: &[u8], key: u32) -> Vec<u8> {
    let words = data.len() / 4;
    let mut out = Vec::with_capacity(data.len());
    let mut previous = key;
    for index in 0..words {
        let start = index * 4;
        let current =
            u32::from_le_bytes(data[start..start + 4].try_into().expect("đã kiểm 4 byte"));
        let word = if index == 0 {
            current ^ key ^ ENTRY_CRYPT_MAGIC
        } else {
            let index = index as u32;
            let poly = index
                .wrapping_mul(index)
                .wrapping_mul(index.wrapping_add(966))
                .wrapping_mul(index.wrapping_add(2_020));
            previous ^ current ^ poly ^ ENTRY_CRYPT_MAGIC
        };
        previous = word;
        out.extend_from_slice(&word.to_le_bytes());
    }
    let tail = &data[words * 4..];
    for (index, &byte) in tail.iter().enumerate() {
        out.push(byte ^ ((previous >> (8 * index)) & 0xFF) as u8);
    }
    out
}

/// Parse index đã giải mã. `first == 0` là format mới (version + CRC); khác 0 là format cũ.
pub fn parse_index(data: &[u8]) -> Result<PackIndex, PackError> {
    let mut at = 0usize;
    let first = read_u32(data, &mut at, "count")?;
    let (version, count, has_crc) = if first == 0 {
        let version = read_u32(data, &mut at, "version")?;
        let count = read_u32(data, &mut at, "count")?;
        (Some(version), count, true)
    } else {
        (None, first, false)
    };

    let remaining = data.len() - at;
    // Entry ngắn nhất (format cũ, tên rỗng) là 4 + 12 = 16 byte; chặn count rác trước khi cấp phát.
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    if count > remaining / 16 {
        return Err(PackError::TooManyEntries {
            count: u32::try_from(count).unwrap_or(u32::MAX),
            remaining,
        });
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let name_len = read_u32(data, &mut at, "độ dài tên")? as usize;
        let name_end = at.checked_add(name_len).ok_or(PackError::Truncated {
            what: "tên",
            at,
            need: name_len,
        })?;
        let name_bytes = data.get(at..name_end).ok_or(PackError::Truncated {
            what: "tên",
            at,
            need: name_len,
        })?;
        at = name_end;
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        let offset = read_u32(data, &mut at, "offset")?;
        let size = read_u32(data, &mut at, "size")?;
        let key = read_u32(data, &mut at, "key")?;
        let crc = if has_crc {
            read_u32(data, &mut at, "crc")?
        } else {
            0
        };
        entries.push(PackEntry {
            name,
            offset,
            size,
            key,
            crc,
        });
    }
    if at != data.len() {
        return Err(PackError::TrailingBytes {
            at,
            len: data.len(),
        });
    }
    Ok(PackIndex { version, entries })
}

/// Giải mã nội dung entry: XOR + CRC, zlib cho `_mm`, còn lại là byte thô.
pub fn decode_entry(name: &str, stored: &[u8], key: u32, crc: u32) -> Result<Vec<u8>, PackError> {
    if key != 0 {
        let plain = decrypt_entry(stored, key);
        if crc > 1 {
            let actual = crc32(key, &plain);
            if actual != crc {
                return Err(PackError::CrcMismatch {
                    name: name.to_owned(),
                    expected: crc,
                    actual,
                });
            }
        }
        return Ok(plain);
    }
    if !name.contains("_mm") {
        return Ok(stored.to_vec());
    }
    if stored.len() < 8 {
        return Err(PackError::ZlibTruncated {
            name: name.to_owned(),
        });
    }
    let compressed = u32::from_le_bytes(stored[..4].try_into().expect("đã kiểm 4 byte")) as usize;
    let raw = u32::from_le_bytes(stored[4..8].try_into().expect("đã kiểm 4 byte")) as usize;
    let compressed_end = 8usize
        .checked_add(compressed)
        .filter(|&end| end <= stored.len())
        .ok_or_else(|| PackError::ZlibTruncated {
            name: name.to_owned(),
        })?;
    let out =
        miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&stored[8..compressed_end], raw)
            .map_err(|_| PackError::Inflate {
                name: name.to_owned(),
            })?;
    if out.len() != raw {
        return Err(PackError::SizeMismatch {
            name: name.to_owned(),
            expected: raw,
            actual: out.len(),
        });
    }
    Ok(out)
}

/// CRC32 kiểu protocol mạng: đa thức `0x04C11DB7`, init cho trước, không đảo kết quả
/// (`Crc32Update`; PACKS §Entry "CRC32 giống protocol mạng").
fn crc32(init: u32, data: &[u8]) -> u32 {
    let mut crc = init;
    for &byte in data {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04C1_1DB7
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn read_u32(data: &[u8], at: &mut usize, what: &'static str) -> Result<u32, PackError> {
    let start = *at;
    let end = start.checked_add(4).ok_or(PackError::Truncated {
        what,
        at: start,
        need: 4,
    })?;
    let bytes = data.get(start..end).ok_or(PackError::Truncated {
        what,
        at: start,
        need: 4,
    })?;
    *at = end;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("đã kiểm 4 byte"),
    ))
}

#[derive(Debug)]
pub enum PackError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    IndexTooShort,
    Truncated {
        what: &'static str,
        at: usize,
        need: usize,
    },
    TooManyEntries {
        count: u32,
        remaining: usize,
    },
    TrailingBytes {
        at: usize,
        len: usize,
    },
    UnknownVnPack {
        stem: String,
    },
    EntryOutOfRange {
        name: String,
        offset: usize,
        size: usize,
        idd_len: usize,
    },
    UnknownEntry {
        name: String,
    },
    CrcMismatch {
        name: String,
        expected: u32,
        actual: u32,
    },
    ZlibTruncated {
        name: String,
    },
    Inflate {
        name: String,
    },
    SizeMismatch {
        name: String,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::IndexTooShort => write!(f, "file .idx ngắn hơn 4 byte, không có khóa"),
            Self::Truncated { what, at, need } => {
                write!(f, "index cụt khi đọc {what} tại byte {at}, cần {need} byte")
            }
            Self::TooManyEntries { count, remaining } => write!(
                f,
                "index khai báo {count} entry nhưng chỉ còn {remaining} byte (không hợp lệ)"
            ),
            Self::TrailingBytes { at, len } => {
                write!(f, "index còn {len} byte, parse dừng ở {at}")
            }
            Self::UnknownVnPack { stem } => {
                write!(f, "chưa có khóa VN cho pack {stem}")
            }
            Self::EntryOutOfRange {
                name,
                offset,
                size,
                idd_len,
            } => write!(
                f,
                "entry {name} vượt .idd: offset {offset}, size {size}, .idd {idd_len} byte"
            ),
            Self::UnknownEntry { name } => write!(f, "không có entry {name}"),
            Self::CrcMismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "CRC {name} sai: index ghi {expected:#010x}, tính được {actual:#010x}"
            ),
            Self::ZlibTruncated { name } => {
                write!(f, "entry zlib {name} thiếu header/byte nén")
            }
            Self::Inflate { name } => write!(f, "không inflate được entry {name}"),
            Self::SizeMismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "entry {name} giải nén {actual} byte, header ghi {expected}"
            ),
        }
    }
}

impl std::error::Error for PackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vector cố định do tool tham chiếu sinh độc lập: plain index 2 entry + key 0x0BADF00D.
    const IDX_KEY: u32 = 0x0BAD_F00D;
    const IDX_PLAIN: &[u8] = &[
        0x00, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00,
        0x00, 0x4d, 0x61, 0x70, 0x2f, 0x56, 0x69, 0x6c, 0x6c, 0x61, 0x67, 0x65, 0x30, 0x30, 0x12,
        0x9e, 0x13, 0x02, 0x2a, 0x07, 0x00, 0x00, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55,
        0x08, 0x00, 0x00, 0x00, 0x45, 0x74, 0x63, 0x2f, 0x41, 0x5f, 0x6d, 0x6d, 0x04, 0x00, 0x00,
        0x00, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    const IDX_CIPHER: &[u8] = &[
        0x0d, 0xf0, 0xad, 0x0b, 0x0f, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00,
        0x00, 0x40, 0x61, 0x70, 0x2f, 0x1b, 0x08, 0x1c, 0x43, 0x37, 0x0e, 0x09, 0x5c, 0x51, 0x75,
        0xfb, 0x23, 0x32, 0x38, 0x99, 0x13, 0x02, 0x6e, 0x34, 0x22, 0x11, 0xcc, 0x44, 0x44, 0x44,
        0x80, 0x77, 0x66, 0x55, 0x4d, 0x74, 0x63, 0x2f, 0x04, 0x2b, 0x0e, 0x42, 0x45, 0x5f, 0x6d,
        0x6d, 0x09, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// Vector cố định của entry mã hóa: plain `hello, pack!`, key 0xA1B2C3D4.
    const ENTRY_KEY: u32 = 0xA1B2_C3D4;
    const ENTRY_PLAIN: &[u8] = b"hello, pack!";
    const ENTRY_CIPHER: &[u8] = &[
        0x72, 0x33, 0xb2, 0x51, 0xca, 0x0e, 0x3d, 0x80, 0x00, 0xac, 0x50, 0xcd,
    ];
    const ENTRY_CRC: u32 = 0x5D0B_89A9;

    /// zlib của chuỗi `hello zlib` (Python `zlib.compress`).
    const ZLIB_HELLO: &[u8] = &[
        0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xa8, 0xca, 0xc9, 0x4c, 0x02, 0x00, 0x15,
        0x95, 0x03, 0xe6,
    ];

    #[test]
    fn decrypts_reference_index_vector() {
        assert_eq!(decrypt_index(IDX_CIPHER, IDX_KEY), IDX_PLAIN);
        assert_eq!(key_from_index(IDX_CIPHER).unwrap(), IDX_KEY);

        let index = parse_index(IDX_PLAIN).unwrap();
        assert_eq!(index.version, Some(15));
        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries[0].name, "Map/Village00");
        assert_eq!(index.entries[0].offset, 34_840_082);
        assert_eq!(index.entries[0].size, 1_834);
        assert_eq!(index.entries[0].key, 0x1122_3344);
        assert_eq!(index.entries[0].crc, 0x5566_7788);
        assert_eq!(index.entries[1].name, "Etc/A_mm");
        assert_eq!(index.entries[1].offset, 4);
        assert_eq!(index.entries[1].size, 13);
        assert_eq!(index.entries[1].key, 0);
        assert_eq!(index.entries[1].crc, 0);
    }

    #[test]
    fn parses_old_index_without_version_or_crc() {
        // Format cũ: dword đầu là count, entry chỉ có offset/size/key.
        let mut plain = Vec::new();
        plain.extend_from_slice(&1u32.to_le_bytes());
        plain.extend_from_slice(&1u32.to_le_bytes());
        plain.push(b'A');
        plain.extend_from_slice(&10u32.to_le_bytes());
        plain.extend_from_slice(&20u32.to_le_bytes());
        plain.extend_from_slice(&0u32.to_le_bytes());
        let index = parse_index(&plain).unwrap();
        assert_eq!(index.version, None);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.entries[0].name, "A");
        assert_eq!(index.entries[0].offset, 10);
        assert_eq!(index.entries[0].size, 20);
        assert_eq!(index.entries[0].crc, 0);
    }

    #[test]
    fn rejects_bad_index_lengths_and_counts() {
        assert!(matches!(
            key_from_index(&[1, 2, 3]),
            Err(PackError::IndexTooShort)
        ));
        assert!(matches!(
            parse_index(&IDX_PLAIN[..10]),
            Err(PackError::Truncated { .. })
        ));
        let mut trailing = IDX_PLAIN.to_vec();
        trailing.push(0);
        assert!(matches!(
            parse_index(&trailing),
            Err(PackError::TrailingBytes { .. })
        ));
        assert!(matches!(
            parse_index(&1u32.to_le_bytes()),
            Err(PackError::TooManyEntries { .. })
        ));
    }

    #[test]
    fn decrypts_and_checks_reference_entry_vector() {
        assert_eq!(decrypt_entry(ENTRY_CIPHER, ENTRY_KEY), ENTRY_PLAIN);
        assert_eq!(crc32(ENTRY_KEY, ENTRY_PLAIN), ENTRY_CRC);
        assert_eq!(
            decode_entry("Map/Village00", ENTRY_CIPHER, ENTRY_KEY, ENTRY_CRC).unwrap(),
            ENTRY_PLAIN
        );
        assert!(matches!(
            decode_entry("Map/Village00", ENTRY_CIPHER, ENTRY_KEY, ENTRY_CRC ^ 1),
            Err(PackError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn inflates_mm_entry_and_passes_raw_entry_through() {
        let mut stored = Vec::new();
        stored.extend_from_slice(&(ZLIB_HELLO.len() as u32).to_le_bytes());
        stored.extend_from_slice(&(b"hello zlib".len() as u32).to_le_bytes());
        stored.extend_from_slice(ZLIB_HELLO);
        assert_eq!(
            decode_entry("Etc/Village00_mm", &stored, 0, 0).unwrap(),
            b"hello zlib"
        );

        // Header không khớp dữ liệu nén phải báo lỗi, không trả rác.
        let mut broken = stored.clone();
        broken[4..8].copy_from_slice(&99u32.to_le_bytes());
        assert!(matches!(
            decode_entry("Etc/Village00_mm", &broken, 0, 0),
            Err(PackError::SizeMismatch { .. })
        ));

        assert_eq!(
            decode_entry("Attr/BomberAttr", b"raw", 0, 0).unwrap(),
            b"raw"
        );
    }

    #[test]
    fn has_vn_key_table_only_for_rev_engine_packs() {
        assert_eq!(vn_pack_key("fx"), Some(1_906_092_848));
        assert_eq!(vn_pack_key("charte"), Some(95_551_996));
        assert_eq!(vn_pack_key("emote"), Some(95_551_996));
        assert_eq!(vn_pack_key("fxasset"), Some(95_551_996));
        assert_eq!(vn_pack_key("FxMap"), None);
    }
}
