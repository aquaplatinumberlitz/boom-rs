//! Kiểu dữ liệu công khai của máy trạng thái phòng (`boom-room`).
//!
//! API ở đây là contract cho server P7: lệnh đã xác thực danh tính do server bind, mang request ID
//! để dedup; kết quả là event + snapshot thuần. Không có socket, đồng hồ thật hay lưu trữ.

use std::fmt;

/// Danh tính phiên người chơi do server xác thực và bind. Không phải tên hiển thị/tài khoản.
///
/// Server cấp ID mới khi auth lại (SCOPE_V1 §3: một tài khoản một phiên), nhờ vậy bảng dedup
/// request theo `PlayerId` không tích theo lịch sử tài khoản.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u64);

/// ID phòng do [`crate::Lobby`] cấp, duy nhất trong một lobby.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomId(pub u64);

/// ID lượt do [`crate::Lobby`] cấp, duy nhất trong một lobby.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoundId(pub u64);

/// Slot cố định trong phòng, từ 0 tới `RoomPolicy::max_players() - 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(pub u8);

/// ID lệnh do client/server sinh cho một phiên; cùng ID nghĩa là cùng lệnh (at-most-once).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(pub u64);

/// Hai template nhân vật của V1 (SCOPE_V1 §1). Không mang chỉ số; chỉ số nối ở content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterId {
    C00,
    C01,
}

impl CharacterId {
    pub const ALL: [Self; 2] = [Self::C00, Self::C01];

    /// Mã ổn định dùng trong protocol/replay, khớp tên asset `C00`/`C01`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::C00 => "C00",
            Self::C01 => "C01",
        }
    }
}

/// Policy phòng do server nạp từ profile/scope và truyền vào lobby.
///
/// Crate không đặt giá trị mặc định: `2–4 người` là policy DESIGN ở SCOPE_V1 §3, không phải hằng số
/// của máy trạng thái. Test dùng đúng giá trị scope làm fixture; server P7 chịu trách nhiệm nạp từ
/// rules profile/SCOPE_V1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomPolicy {
    max_players: u8,
    min_players_to_start: u8,
}

impl RoomPolicy {
    pub fn new(max_players: u8, min_players_to_start: u8) -> Result<Self, PolicyError> {
        if max_players == 0 {
            return Err(PolicyError::NoSlots);
        }
        if min_players_to_start == 0 {
            return Err(PolicyError::NoStartThreshold);
        }
        if min_players_to_start > max_players {
            return Err(PolicyError::StartThresholdAboveCapacity {
                min: min_players_to_start,
                max: max_players,
            });
        }
        Ok(Self {
            max_players,
            min_players_to_start,
        })
    }

    /// Số slot tối đa của một phòng.
    pub const fn max_players(self) -> u8 {
        self.max_players
    }

    /// Số người tối thiểu để được start.
    pub const fn min_players_to_start(self) -> u8 {
        self.min_players_to_start
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError {
    NoSlots,
    NoStartThreshold,
    StartThresholdAboveCapacity { min: u8, max: u8 },
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSlots => write!(f, "policy phòng phải có ít nhất một slot"),
            Self::NoStartThreshold => write!(f, "policy phòng phải có ngưỡng start dương"),
            Self::StartThresholdAboveCapacity { min, max } => {
                write!(f, "ngưỡng start {min} lớn hơn sức chứa {max} của phòng")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// Pha của phòng trong phạm vi task #13.
///
/// `Loading`/`Playing`/`Results` của SCOPE_V1 §3 thuộc vòng đời lượt (V1-010/022/023) và sẽ được
/// thêm cùng task tương ứng; task này chỉ cần waiting → start một lượt duy nhất.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomPhase {
    Waiting,
    Started,
}

impl RoomPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Started => "started",
        }
    }
}

/// Một thành viên trong snapshot phòng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberView {
    pub player: PlayerId,
    pub slot: SlotId,
    pub character: CharacterId,
    pub ready: bool,
}

/// Một người trong roster đã chốt lúc start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoundMember {
    pub player: PlayerId,
    pub slot: SlotId,
    pub character: CharacterId,
}

/// Lượt đã chốt; roster bất biến với các lệnh phòng sau đó.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundView {
    pub id: RoundId,
    pub members: Vec<RoundMember>,
}

/// Snapshot phòng để server gửi client hoặc test đối chiếu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomView {
    pub id: RoomId,
    pub host: PlayerId,
    pub phase: RoomPhase,
    /// Sắp theo slot tăng dần.
    pub members: Vec<MemberView>,
    pub round: Option<RoundView>,
}

/// Lệnh phòng đã được server xác thực danh tính và bind `player`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomCommand {
    pub player: PlayerId,
    pub request: RequestId,
    pub action: RoomAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomAction {
    /// Tạo phòng; người tạo là host và vào slot 0 với nhân vật đã chọn.
    Create { character: CharacterId },
    /// Vào phòng đang chờ; chỉ khi còn slot.
    Join {
        room: RoomId,
        character: CharacterId,
    },
    /// Rời phòng ở mọi pha; nếu là host thì chuyển cho người vào sớm nhất còn lại.
    Leave,
    /// Host kick người khác khi phòng đang chờ.
    Kick { target: PlayerId },
    /// Đổi nhân vật; đổi khác giá trị sẽ hủy toàn bộ ready (SCOPE_V1 §3).
    SelectCharacter { character: CharacterId },
    /// Bật/tắt ready; host cũng phải ready.
    SetReady { ready: bool },
    /// Host start khi đủ 2–4 người và mọi người ready; chỉ tạo một lượt.
    Start,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyResetReason {
    /// Có người vào/rời/bị kick.
    RosterChanged,
    /// Có người đổi nhân vật.
    SelectionChanged,
}

/// Thay đổi phòng phát ra cho server/client. Thứ tự trong một lệnh là ổn định.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomEvent {
    RoomCreated {
        room: RoomId,
        host: PlayerId,
        character: CharacterId,
    },
    PlayerJoined {
        room: RoomId,
        player: PlayerId,
        slot: SlotId,
        character: CharacterId,
    },
    PlayerLeft {
        room: RoomId,
        player: PlayerId,
    },
    PlayerKicked {
        room: RoomId,
        player: PlayerId,
        by: PlayerId,
    },
    HostChanged {
        room: RoomId,
        host: PlayerId,
        previous: PlayerId,
    },
    CharacterSelected {
        room: RoomId,
        player: PlayerId,
        character: CharacterId,
    },
    ReadyChanged {
        room: RoomId,
        player: PlayerId,
        ready: bool,
    },
    /// Chỉ phát khi thực sự có ready bị hủy.
    ReadyReset {
        room: RoomId,
        reason: ReadyResetReason,
        cleared: Vec<PlayerId>,
    },
    RoundStarted {
        room: RoomId,
        round: RoundId,
        members: Vec<RoundMember>,
    },
}

/// Kết quả một lệnh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Applied(Vec<RoomEvent>),
    /// Request ID đã xử lý thành công trước đó; không đổi state, không phát event lần hai.
    Duplicate,
}

impl Outcome {
    pub fn events(&self) -> &[RoomEvent] {
        match self {
            Self::Applied(events) => events,
            Self::Duplicate => &[],
        }
    }

    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::Duplicate)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    PlayerAlreadyInRoom {
        player: PlayerId,
        room: RoomId,
    },
    PlayerNotInRoom {
        player: PlayerId,
    },
    RoomNotFound {
        room: RoomId,
    },
    RoomFull {
        room: RoomId,
        capacity: u8,
    },
    NotHost {
        player: PlayerId,
        room: RoomId,
    },
    CannotKickSelf {
        player: PlayerId,
    },
    TargetNotInRoom {
        target: PlayerId,
        room: RoomId,
    },
    NotWaiting {
        room: RoomId,
        phase: RoomPhase,
    },
    AlreadyStarted {
        room: RoomId,
        round: RoundId,
    },
    NotEnoughPlayers {
        room: RoomId,
        required: u8,
        present: u8,
    },
    NotAllReady {
        room: RoomId,
        waiting: Vec<PlayerId>,
    },
    StaleRequest {
        player: PlayerId,
        last: RequestId,
        received: RequestId,
    },
    IdExhausted,
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayerAlreadyInRoom { player, room } => {
                write!(f, "người chơi {:?} đã ở phòng {:?}", player, room)
            }
            Self::PlayerNotInRoom { player } => {
                write!(f, "người chơi {:?} không ở phòng nào", player)
            }
            Self::RoomNotFound { room } => write!(f, "không có phòng {:?}", room),
            Self::RoomFull { room, capacity } => {
                write!(f, "phòng {:?} đã đủ {capacity} slot", room)
            }
            Self::NotHost { player, room } => {
                write!(
                    f,
                    "người chơi {:?} không phải host phòng {:?}",
                    player, room
                )
            }
            Self::CannotKickSelf { player } => {
                write!(f, "host {:?} không thể kick chính mình", player)
            }
            Self::TargetNotInRoom { target, room } => {
                write!(f, "người chơi {:?} không ở phòng {:?}", target, room)
            }
            Self::NotWaiting { room, phase } => write!(
                f,
                "phòng {:?} đang ở pha {}, lệnh chỉ hợp lệ khi waiting",
                room,
                phase.label()
            ),
            Self::AlreadyStarted { room, round } => {
                write!(f, "phòng {:?} đã start lượt {:?}", room, round)
            }
            Self::NotEnoughPlayers {
                room,
                required,
                present,
            } => write!(
                f,
                "phòng {room:?} có {present} người, cần ít nhất {required} để start"
            ),
            Self::NotAllReady { room, waiting } => {
                write!(f, "phòng {room:?} còn người chưa ready: {waiting:?}")
            }
            Self::StaleRequest {
                player,
                last,
                received,
            } => write!(
                f,
                "request {:?} của {:?} cũ hơn request đã xử lý {:?}",
                received, player, last
            ),
            Self::IdExhausted => write!(f, "đã hết ID phòng/lượt"),
        }
    }
}

impl std::error::Error for RoomError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_rejects_invalid_values() {
        assert_eq!(RoomPolicy::new(0, 1), Err(PolicyError::NoSlots));
        assert_eq!(RoomPolicy::new(4, 0), Err(PolicyError::NoStartThreshold));
        assert_eq!(
            RoomPolicy::new(2, 3),
            Err(PolicyError::StartThresholdAboveCapacity { min: 3, max: 2 })
        );
    }

    #[test]
    fn character_codes_are_stable() {
        assert_eq!(CharacterId::ALL.map(CharacterId::code), ["C00", "C01"]);
    }
}
