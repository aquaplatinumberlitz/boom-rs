//! Lỗi chuyển đổi giữa state của core và DTO trên dây.

use std::fmt;

use boom_proto::ProtoError;

/// Lỗi khi dựng DTO từ `SimulationState` hoặc khôi phục `SimulationState` từ DTO.
///
/// Mọi lỗi là giá trị trả về: không panic, không áp dụng một phần (ADR 0009 §4.3, §5.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncError {
    /// Lỗi codec khi encode/decode.
    Proto(ProtoError),
    /// Kích thước lưới của snapshot khác địa hình của `World` đang dùng.
    GridMismatch {
        /// Kích thước trong snapshot.
        snapshot: (u16, u16),
        /// Kích thước của địa hình.
        world: (u16, u16),
    },
    /// Toạ độ người chơi nằm ngoài miền `Mpx` của phiên (ADR 0009 §4.2).
    PositionOutOfRange {
        /// Người chơi.
        player: u8,
        /// Toạ độ x.
        x: i32,
        /// Toạ độ y.
        y: i32,
        /// Trần của x (không kèm).
        max_x: i32,
        /// Trần của y (không kèm).
        max_y: i32,
    },
    /// Chỉ số ô nằm ngoài lưới.
    CellOutOfRange {
        /// Ô trong DTO.
        cell: u16,
        /// Số ô của lưới.
        cells: usize,
    },
    /// Độ dài `kinds_bits` không khớp số ô của lưới (ADR 0009 §4.1).
    KindsBitsLength {
        /// Số byte cần có.
        expected: usize,
        /// Số byte thực có.
        got: usize,
    },
    /// Loại ô trong DTO là giá trị không hợp lệ (mã 3).
    InvalidCellKind {
        /// Ô.
        cell: u16,
    },
    /// Chỉ số loại item không có trong `Rules::items` của profile.
    ItemKindOutOfRange {
        /// Chỉ số trong DTO.
        kind: u8,
        /// Số loại item trong rules.
        count: usize,
    },
    /// Chỉ số loại item của core vượt miền 1 byte của DTO.
    ItemKindTooLarge(u16),
    /// Mã hướng không hợp lệ.
    BadDirection(u8),
    /// Bit dành riêng của lệnh khác 0 (ADR 0009 §3.1).
    ReservedBits(u8),
    /// Không dựng lại được allocator: `next_entity_id` bằng 0.
    Allocator(u32),
    /// Vi phạm bất biến contract của core khi dựng lại state.
    Invariant(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proto(e) => write!(f, "lỗi codec: {e}"),
            Self::GridMismatch { snapshot, world } => write!(
                f,
                "lưới snapshot {}x{} khác địa hình {}x{}",
                snapshot.0, snapshot.1, world.0, world.1
            ),
            Self::PositionOutOfRange {
                player,
                x,
                y,
                max_x,
                max_y,
            } => write!(
                f,
                "người chơi {player} ở ({x},{y}) ngoài miền [0,{max_x})×[0,{max_y})"
            ),
            Self::CellOutOfRange { cell, cells } => {
                write!(f, "ô {cell} ngoài lưới {cells} ô")
            }
            Self::KindsBitsLength { expected, got } => {
                write!(f, "kinds_bits cần {expected} byte cho lưới, có {got} byte")
            }
            Self::InvalidCellKind { cell } => write!(f, "ô {cell} có loại không hợp lệ"),
            Self::ItemKindOutOfRange { kind, count } => {
                write!(f, "loại item {kind} không có trong rules ({count} loại)")
            }
            Self::ItemKindTooLarge(kind) => {
                write!(f, "loại item {kind} vượt miền 1 byte của DTO")
            }
            Self::BadDirection(code) => write!(f, "mã hướng {code} không hợp lệ"),
            Self::ReservedBits(bits) => write!(f, "bit dành riêng của lệnh khác 0: {bits:#04x}"),
            Self::Allocator(next) => {
                write!(
                    f,
                    "next_entity_id {next} không hợp lệ (ID 0 không được cấp)"
                )
            }
            Self::Invariant(e) => write!(f, "state dựng lại vi phạm contract: {e}"),
        }
    }
}

impl std::error::Error for SyncError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Proto(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ProtoError> for SyncError {
    fn from(e: ProtoError) -> Self {
        Self::Proto(e)
    }
}
