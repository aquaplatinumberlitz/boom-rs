//! Chuyển `InputCommand` trên dây → `PlayerInput` của core (ADR 0009 §3).
//!
//! Input mạng chỉ là **đề nghị**; server (scheduler) kiểm round, sequence, cửa sổ và rate limit
//! rồi mới gộp các lệnh đã chín thành `PlayerInput` cho tick đó. Việc gộp nằm ở
//! [`merge_commands`] để server và client dùng chung một cách hiểu.

use boom_core::{Dir, PlayerInput};
use boom_proto::InputCommand;

use crate::error::SyncError;

/// Một lệnh đã được authority chấp nhận → `PlayerInput` của core.
///
/// Kiểm bit dành riêng bằng 0 và mã hướng hợp lệ; lệnh sai bị từ chối chứ không đoán.
pub fn command_input(command: &InputCommand) -> Result<PlayerInput, SyncError> {
    let bits = command.bits;
    if !bits.reserved_is_zero() {
        return Err(SyncError::ReservedBits(bits.0));
    }
    let dir = dir_from_code(bits.direction()).ok_or(SyncError::BadDirection(bits.direction()))?;
    Ok(PlayerInput {
        dir,
        place_bomb: bits.place_bomb(),
    })
}

/// Gộp các lệnh đã chín của một tick thành một `PlayerInput` (ADR 0009 §3.3 bước 7).
///
/// Hướng lấy của lệnh **mới nhất**; `place_bomb` đúng khi có ít nhất một cạnh bom trong các lệnh
/// đã chín (nhiều cạnh bom trong cùng tick chỉ tạo tối đa một `place_bomb`).
///
/// Danh sách phải theo thứ tự `seq` tăng dần; hàm không tự sắp lại.
pub fn merge_commands(commands: &[InputCommand]) -> Result<PlayerInput, SyncError> {
    let mut merged = PlayerInput::default();
    for command in commands {
        let input = command_input(command)?;
        merged.dir = input.dir;
        merged.place_bomb |= input.place_bomb;
    }
    Ok(merged)
}

/// Mã hướng trên dây → hướng của core (ADR 0009 §3.1).
fn dir_from_code(code: u8) -> Option<Dir> {
    Some(match code {
        boom_proto::dto::DIR_NONE => Dir::None,
        boom_proto::dto::DIR_UP => Dir::Up,
        boom_proto::dto::DIR_DOWN => Dir::Down,
        boom_proto::dto::DIR_LEFT => Dir::Left,
        boom_proto::dto::DIR_RIGHT => Dir::Right,
        _ => return None,
    })
}
