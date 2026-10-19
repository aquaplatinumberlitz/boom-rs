//! Loại message theo bảng kind của ADR 0009 §2.
//!
//! V1 chỉ chấp nhận `wire_schema = 1` khớp chính xác. Message mới chỉ được gửi khi capability
//! tương ứng được hai bên chọn trong handshake; gặp kind lạ thì từ chối, không bỏ qua rồi đoán.

/// Kind của một message trong envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum MessageKind {
    /// C→S: handshake.
    Hello = 0x0001,
    /// S→C: handshake.
    Welcome = 0x0002,
    /// S→C: lỗi handshake/session.
    Reject = 0x0003,
    /// Hai chiều: đo RTT/liveness; payload là `u64`.
    Ping = 0x000E,
    /// Hai chiều: trả nguyên `Ping`.
    Pong = 0x000F,
    /// C→S: input trận, có cửa sổ dư thừa.
    InputBatch = 0x0010,
    /// S→C: lệnh bị từ chối; mang ack để resync.
    InputRejected = 0x0011,
    /// S→C: snapshot đầy đủ.
    Snapshot = 0x0020,
    /// C→S: xin snapshot đầy đủ để resync.
    SnapshotRequest = 0x0021,
    /// S→C: event của round, có `EventId`.
    RoundEvents = 0x0030,
    /// C→S: lệnh phòng (dành cho task backend; ADR 0009 chưa định nghĩa).
    RoomCommand = 0x0040,
    /// S→C: trạng thái phòng (dành cho task backend; ADR 0009 chưa định nghĩa).
    RoomState = 0x0041,
}

impl MessageKind {
    /// Kind từ mã trong envelope; `None` khi mã không có trong bảng.
    pub fn from_u16(code: u16) -> Option<Self> {
        Some(match code {
            0x0001 => Self::Hello,
            0x0002 => Self::Welcome,
            0x0003 => Self::Reject,
            0x000E => Self::Ping,
            0x000F => Self::Pong,
            0x0010 => Self::InputBatch,
            0x0011 => Self::InputRejected,
            0x0020 => Self::Snapshot,
            0x0021 => Self::SnapshotRequest,
            0x0030 => Self::RoundEvents,
            0x0040 => Self::RoomCommand,
            0x0041 => Self::RoomState,
            _ => return None,
        })
    }

    /// Mã trong envelope.
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Kind đã có payload định nghĩa ở V1 chưa.
    ///
    /// `RoomCommand`/`RoomState` có mã trong bảng nhưng ADR 0009 chưa định nghĩa payload; chúng thuộc
    /// task backend (#13) nên crate này từ chối chứ không đoán.
    pub const fn is_defined(self) -> bool {
        !matches!(self, Self::RoomCommand | Self::RoomState)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for kind in [
            MessageKind::Hello,
            MessageKind::Welcome,
            MessageKind::Reject,
            MessageKind::Ping,
            MessageKind::Pong,
            MessageKind::InputBatch,
            MessageKind::InputRejected,
            MessageKind::Snapshot,
            MessageKind::SnapshotRequest,
            MessageKind::RoundEvents,
            MessageKind::RoomCommand,
            MessageKind::RoomState,
        ] {
            assert_eq!(MessageKind::from_u16(kind.code()), Some(kind));
        }
    }

    #[test]
    fn unknown_code_is_rejected() {
        assert_eq!(MessageKind::from_u16(0x0000), None);
        assert_eq!(MessageKind::from_u16(0x00FF), None);
        assert_eq!(MessageKind::from_u16(0xFFFF), None);
    }

    #[test]
    fn room_kinds_are_known_but_not_defined() {
        assert!(!MessageKind::RoomCommand.is_defined());
        assert!(!MessageKind::RoomState.is_defined());
        assert!(MessageKind::Snapshot.is_defined());
    }
}
