//! Giao thức V1 (ADR 0009, ARCHITECTURE §5.4): envelope viết tay có `wire_schema`, payload postcard.
//!
//! Chỉ chứa DTO và codec; không phụ thuộc `boom-core` (ARCHITECTURE §1). Chuyển đổi giữa `SimulationState`
//! và DTO nằm ở adapter `boom-sync`. Chưa triển khai — task đợt 2 (#44).
