//! Loại message theo bảng kind của ADR 0009 §2 và Addendum A §A.1.
//!
//! V1 chỉ chấp nhận `wire_schema = 1` khớp chính xác. Message mới chỉ được gửi khi capability
//! tương ứng được hai bên chọn trong handshake; gặp kind lạ thì từ chối, không bỏ qua rồi đoán.
//!
//! Addendum A thêm kind auth (`0x0004`–`0x0007`) và kind health (`0x0050`/`0x0051`) cho admin
//! listener loopback (ADR 0011 §8.2.1). Vì vậy mỗi kind có **kênh** ([`Channel`]) và **chiều**
//! ([`Direction`]): kênh game không nhận kind health, admin listener chỉ nhận `HealthRequest`.
//! Sai kênh xử lý như kind lạ — đóng kết nối và log, không đoán.

/// Kênh trao đổi message (ADR 0009 Addendum A §A.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Kênh game: client ↔ server (`Hello`/`Welcome`/auth/input/snapshot/round).
    Game,
    /// Kênh admin loopback (ADR 0011 §8.2.1): chỉ `HealthRequest`/`HealthResponse`.
    Admin,
}

/// Chiều trao đổi của một kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Client → server.
    ClientToServer,
    /// Server → client.
    ServerToClient,
    /// Hai chiều (`Ping`/`Pong`).
    Both,
}

/// Bên đang xử lý kết nối, để biết chiều message nào là hợp lệ khi **nhận**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Server (listener của `boom-server`): nhận message từ client.
    Server,
    /// Client (`boom-client`): nhận message từ server.
    Client,
}

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
    /// C→S: đăng ký hoặc đăng nhập sau `Welcome` (ADR 0009 Addendum A §A.1).
    Auth = 0x0004,
    /// S→C: auth thành công, phát token phiên (ADR 0009 Addendum A §A.1).
    AuthOk = 0x0005,
    /// S→C: auth thất bại, có mã lý do (ADR 0009 Addendum A §A.1).
    AuthFail = 0x0006,
    /// C→S: thu hồi phiên rồi server đóng kết nối (ADR 0009 Addendum A §A.1).
    Logout = 0x0007,
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
    /// S→C: trạng thái phòng (ADR 0009 Addendum B).
    RoomState = 0x0041,
    /// C→S: xin danh sách phòng sảnh (ADR 0009 Addendum B).
    LobbyListRequest = 0x0042,
    /// S→C: danh sách phòng + số người (ADR 0009 Addendum B).
    LobbyList = 0x0043,
    /// C→S: chat phòng (ADR 0009 Addendum B).
    RoomChat = 0x0044,
    /// S→C: chat phòng đã nhận (ADR 0009 Addendum B).
    RoomChatMessage = 0x0045,
    /// C→S: client đã sẵn sàng vào round (ADR 0009 Addendum B).
    LoadingAck = 0x0046,
    /// C→S: client đã nhận kết quả (ADR 0009 Addendum B).
    ResultAck = 0x0047,
    /// S→C: bắt đầu round, seed/rules/map/roster (ADR 0009 Addendum B).
    RoundStart = 0x0048,
    /// S→C: kết quả round (ADR 0009 Addendum B).
    RoundResult = 0x0049,
    /// S→C: lệnh phòng bị từ chối (ADR 0009 Addendum B).
    RoomRejected = 0x004A,
    /// C→S: cây hỏi sức khoẻ runtime qua admin listener loopback (ADR 0011 §8.2.1).
    HealthRequest = 0x0050,
    /// S→C: trả lời sức khoẻ runtime qua admin listener loopback (ADR 0011 §8.2.1).
    HealthResponse = 0x0051,
}

impl MessageKind {
    /// Kind từ mã trong envelope; `None` khi mã không có trong bảng.
    pub fn from_u16(code: u16) -> Option<Self> {
        Some(match code {
            0x0001 => Self::Hello,
            0x0002 => Self::Welcome,
            0x0003 => Self::Reject,
            0x0004 => Self::Auth,
            0x0005 => Self::AuthOk,
            0x0006 => Self::AuthFail,
            0x0007 => Self::Logout,
            0x000E => Self::Ping,
            0x000F => Self::Pong,
            0x0010 => Self::InputBatch,
            0x0011 => Self::InputRejected,
            0x0020 => Self::Snapshot,
            0x0021 => Self::SnapshotRequest,
            0x0030 => Self::RoundEvents,
            0x0040 => Self::RoomCommand,
            0x0041 => Self::RoomState,
            0x0042 => Self::LobbyListRequest,
            0x0043 => Self::LobbyList,
            0x0044 => Self::RoomChat,
            0x0045 => Self::RoomChatMessage,
            0x0046 => Self::LoadingAck,
            0x0047 => Self::ResultAck,
            0x0048 => Self::RoundStart,
            0x0049 => Self::RoundResult,
            0x004A => Self::RoomRejected,
            0x0050 => Self::HealthRequest,
            0x0051 => Self::HealthResponse,
            _ => return None,
        })
    }

    /// Mã trong envelope.
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Kind đã có payload định nghĩa ở V1 chưa.
    ///
    /// Toàn bộ kind room đã được định nghĩa ở Addendum B; kind lạ vẫn bị `from_u16` từ chối.
    pub const fn is_defined(self) -> bool {
        true
    }

    /// Kind thuộc nhóm phòng/sảnh (Addendum B); chỉ hợp lệ sau `AuthOk`.
    pub const fn is_room(self) -> bool {
        matches!(
            self,
            Self::RoomCommand
                | Self::RoomState
                | Self::LobbyListRequest
                | Self::LobbyList
                | Self::RoomChat
                | Self::RoomChatMessage
                | Self::LoadingAck
                | Self::ResultAck
                | Self::RoundStart
                | Self::RoundResult
                | Self::RoomRejected
        )
    }

    /// Kênh mà kind này thuộc (ADR 0009 Addendum A §A.1).
    ///
    /// Chỉ `HealthRequest`/`HealthResponse` thuộc kênh admin; mọi kind khác thuộc kênh game.
    pub const fn channel(self) -> Channel {
        match self {
            Self::HealthRequest | Self::HealthResponse => Channel::Admin,
            _ => Channel::Game,
        }
    }

    /// Chiều của kind này (ADR 0009 Addendum A §A.1).
    pub const fn direction(self) -> Direction {
        match self {
            Self::Hello
            | Self::Auth
            | Self::Logout
            | Self::InputBatch
            | Self::SnapshotRequest
            | Self::RoomCommand
            | Self::LobbyListRequest
            | Self::RoomChat
            | Self::LoadingAck
            | Self::ResultAck
            | Self::HealthRequest => Direction::ClientToServer,
            Self::Welcome
            | Self::Reject
            | Self::AuthOk
            | Self::AuthFail
            | Self::InputRejected
            | Self::Snapshot
            | Self::RoundEvents
            | Self::RoomState
            | Self::LobbyList
            | Self::RoomChatMessage
            | Self::RoundStart
            | Self::RoundResult
            | Self::RoomRejected
            | Self::HealthResponse => Direction::ServerToClient,
            Self::Ping | Self::Pong => Direction::Both,
        }
    }

    /// Kind này có hợp lệ trên `channel` không.
    ///
    /// Kênh game **không** nhận kind health; admin listener **chỉ** nhận kind health
    /// (ADR 0009 Addendum A §A.1).
    pub fn is_valid_on(self, channel: Channel) -> bool {
        self.channel() == channel
    }

    /// Kind này có thể **nhận** trên `channel` với vai trò `role` không.
    ///
    /// Vai trò quyết định chiều hợp lệ khi nhận: server nhận message chiều client→server, client
    /// nhận message chiều server→client; kind hai chiều (`Ping`/`Pong`) thì cả hai đều nhận được.
    ///
    /// Vì vậy admin listener (`Role::Server`) chỉ nhận `HealthRequest` — `HealthResponse` là chiều
    /// server→client — và kênh game không nhận kind health (ADR 0009 Addendum A §A.1).
    pub fn is_receivable_on(self, channel: Channel, role: Role) -> bool {
        if !self.is_valid_on(channel) {
            return false;
        }
        matches!(
            (role, self.direction()),
            (Role::Server, Direction::ClientToServer | Direction::Both)
                | (Role::Client, Direction::ServerToClient | Direction::Both)
        )
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
            MessageKind::Auth,
            MessageKind::AuthOk,
            MessageKind::AuthFail,
            MessageKind::Logout,
            MessageKind::Ping,
            MessageKind::Pong,
            MessageKind::InputBatch,
            MessageKind::InputRejected,
            MessageKind::Snapshot,
            MessageKind::SnapshotRequest,
            MessageKind::RoundEvents,
            MessageKind::RoomCommand,
            MessageKind::RoomState,
            MessageKind::LobbyListRequest,
            MessageKind::LobbyList,
            MessageKind::RoomChat,
            MessageKind::RoomChatMessage,
            MessageKind::LoadingAck,
            MessageKind::ResultAck,
            MessageKind::RoundStart,
            MessageKind::RoundResult,
            MessageKind::RoomRejected,
            MessageKind::HealthRequest,
            MessageKind::HealthResponse,
        ] {
            assert_eq!(MessageKind::from_u16(kind.code()), Some(kind));
        }
    }

    /// Mã kind của Addendum A phải đúng số trong ADR 0009 Addendum A §A.1.
    #[test]
    fn addendum_a_kind_codes_match_adr() {
        assert_eq!(MessageKind::Auth.code(), 0x0004);
        assert_eq!(MessageKind::AuthOk.code(), 0x0005);
        assert_eq!(MessageKind::AuthFail.code(), 0x0006);
        assert_eq!(MessageKind::Logout.code(), 0x0007);
        assert_eq!(MessageKind::HealthRequest.code(), 0x0050);
        assert_eq!(MessageKind::HealthResponse.code(), 0x0051);
    }

    #[test]
    fn unknown_code_is_rejected() {
        assert_eq!(MessageKind::from_u16(0x0000), None);
        assert_eq!(MessageKind::from_u16(0x00FF), None);
        assert_eq!(MessageKind::from_u16(0xFFFF), None);
    }

    #[test]
    fn room_kinds_are_defined_after_addendum_b() {
        for kind in [
            MessageKind::RoomCommand,
            MessageKind::RoomState,
            MessageKind::LobbyListRequest,
            MessageKind::LobbyList,
            MessageKind::RoomChat,
            MessageKind::RoomChatMessage,
            MessageKind::LoadingAck,
            MessageKind::ResultAck,
            MessageKind::RoundStart,
            MessageKind::RoundResult,
            MessageKind::RoomRejected,
        ] {
            assert!(kind.is_defined());
            assert!(kind.is_room());
            assert_eq!(kind.channel(), Channel::Game);
        }
        assert!(!MessageKind::HealthRequest.is_room());
    }

    /// Kênh game không nhận kind health; admin listener chỉ nhận `HealthRequest`.
    #[test]
    fn health_kinds_are_admin_only() {
        for kind in [MessageKind::HealthRequest, MessageKind::HealthResponse] {
            assert_eq!(kind.channel(), Channel::Admin);
            assert!(!kind.is_valid_on(Channel::Game));
            assert!(kind.is_valid_on(Channel::Admin));
        }
        for kind in [
            MessageKind::Hello,
            MessageKind::Welcome,
            MessageKind::Reject,
            MessageKind::Auth,
            MessageKind::AuthOk,
            MessageKind::AuthFail,
            MessageKind::Logout,
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
            assert_eq!(kind.channel(), Channel::Game);
            assert!(kind.is_valid_on(Channel::Game));
            assert!(!kind.is_valid_on(Channel::Admin));
        }
    }

    /// Admin listener chỉ **nhận** `HealthRequest`; `HealthResponse` là chiều server→client.
    #[test]
    fn receivable_respects_channel_and_direction() {
        // Server nhận từ client: chiều C→S (hoặc hai chiều).
        assert!(MessageKind::HealthRequest.is_receivable_on(Channel::Admin, Role::Server));
        assert!(!MessageKind::HealthResponse.is_receivable_on(Channel::Admin, Role::Server));
        assert!(!MessageKind::HealthRequest.is_receivable_on(Channel::Game, Role::Server));
        assert!(!MessageKind::HealthResponse.is_receivable_on(Channel::Game, Role::Server));

        assert!(MessageKind::Auth.is_receivable_on(Channel::Game, Role::Server));
        assert!(MessageKind::Logout.is_receivable_on(Channel::Game, Role::Server));
        assert!(!MessageKind::AuthOk.is_receivable_on(Channel::Game, Role::Server));
        assert!(!MessageKind::AuthFail.is_receivable_on(Channel::Game, Role::Server));

        // Client nhận từ server: chiều S→C (hoặc hai chiều).
        assert!(MessageKind::AuthOk.is_receivable_on(Channel::Game, Role::Client));
        assert!(MessageKind::AuthFail.is_receivable_on(Channel::Game, Role::Client));
        assert!(!MessageKind::Auth.is_receivable_on(Channel::Game, Role::Client));
        assert!(!MessageKind::Logout.is_receivable_on(Channel::Game, Role::Client));

        // `Ping`/`Pong` hai chiều nên hai bên đều nhận được trên kênh game.
        for role in [Role::Server, Role::Client] {
            assert!(MessageKind::Ping.is_receivable_on(Channel::Game, role));
            assert!(MessageKind::Pong.is_receivable_on(Channel::Game, role));
        }
    }
}
