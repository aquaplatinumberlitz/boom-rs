//! Đọc định dạng dữ liệu của client gốc và chuyển sang schema `boom-content`.
//!
//! Hiện có: map `.map` (KR/VN). Pack `.idx/.idd`, sprite và âm thanh vào đây ở P4,
//! viết lại theo định dạng đã RE (ARCHITECTURE §7), không copy code legacy.

pub mod legacy_map;
