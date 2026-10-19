//! Đọc định dạng dữ liệu của client gốc và chuyển sang schema `boom-content`.
//!
//! Hiện có: map `.map` (KR/VN) và pack `.idx/.idd` (index, entry mã hóa, zlib).
//! Viết lại theo định dạng đã RE (ARCHITECTURE §7), không copy code legacy.

pub mod closure;
pub mod legacy_map;
pub mod pack;
