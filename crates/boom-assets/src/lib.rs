//! Đọc định dạng dữ liệu của client gốc và chuyển sang schema `boom-content`.
//!
//! Hiện có: map `.map` (KR/VN) và pack `.idx/.idd` (index, entry mã hóa, zlib), manifest
//! closure của một trận V1, giải mã sprite `CDataI16Alpha` v2, bộ trích animation V1 (entry → clip,
//! frame, pivot, thời lượng), và dựng bundle nội dung (atlas ảnh + metadata ô) cho client.
//! Viết lại theo định dạng đã RE (ARCHITECTURE §7), không copy code legacy.

pub mod animation;
pub mod bundle;
pub mod closure;
pub mod legacy_map;
pub mod pack;
pub mod sprite;
