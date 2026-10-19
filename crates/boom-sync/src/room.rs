//! Adapter phòng: `boom_room` ↔ DTO `boom_proto::room` (ADR 0009 Addendum B).
//!
//! `boom-proto` không phụ thuộc `boom-room`; module này là chỗ duy nhất biết ánh xạ
//! `RoomAction`/`RoomView`. Cổng trạng thái [`RoomGate`] chỉ cho kind phòng đi qua sau `AuthOk`
//! (Addendum A); đây là kiểm **tầng phiên**, không phải kiểm quyền của từng lệnh.

use std::fmt;

use boom_proto::{
    LobbyList, LobbyRoom, MessageKind, RoomAction as RoomActionDto, RoomCharacter,
    RoomCommand as RoomCommandDto, RoomMember as RoomMemberDto, RoomPhase as RoomPhaseDto,
    RoomRejectReason, RoomState, RoundInfo, RoundMember as RoundMemberDto,
};
use boom_room::{
    CharacterId, MemberView, PlayerId, RequestId, RoomAction, RoomCommand, RoomError, RoomPhase,
    RoomView, RoundMember, RoundView, SlotId,
};

/// Lỗi ánh xạ DTO phòng ↔ `boom-room`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomSyncError {
    /// Mã nhân vật không phải C00/C01.
    UnknownCharacter(u8),
    /// Host không nằm trong danh sách thành viên.
    HostNotMember(u64),
    /// Hai thành viên trùng slot.
    DuplicateSlot(u8),
    /// Hai thành viên trùng player ID.
    DuplicatePlayer(u64),
    /// Roster round có người không còn trong phòng.
    RoundMemberNotInRoom(u64),
}

impl fmt::Display for RoomSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCharacter(code) => write!(f, "mã nhân vật {code} không hợp lệ"),
            Self::HostNotMember(player) => write!(f, "host {player} không ở trong phòng"),
            Self::DuplicateSlot(slot) => write!(f, "slot {slot} bị trùng"),
            Self::DuplicatePlayer(player) => write!(f, "player {player} bị trùng"),
            Self::RoundMemberNotInRoom(player) => {
                write!(f, "player {player} trong roster không còn ở phòng")
            }
        }
    }
}

impl std::error::Error for RoomSyncError {}

/// Cổng trạng thái phiên cho kind phòng (ADR 0009 Addendum A §A.1, Addendum B).
///
/// `RoomGate` chỉ phân loại **được phép nhận hay không** dựa trên đã `AuthOk`; nó không thay
/// auth store hay kiểm quyền host. Trước `AuthOk`, mọi kind phòng bị trả
/// [`RoomGateError::RequiresAuth`] để server `Reject`/đóng, không decode payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomGate {
    authenticated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomGateError {
    /// Kind phòng đến trước `AuthOk`.
    RequiresAuth(MessageKind),
}

impl fmt::Display for RoomGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresAuth(kind) => {
                write!(f, "kind {kind:?} chỉ hợp lệ sau AuthOk")
            }
        }
    }
}

impl std::error::Error for RoomGateError {}

impl RoomGate {
    pub const fn new() -> Self {
        Self {
            authenticated: false,
        }
    }

    /// Đánh dấu phiên đã `AuthOk`.
    pub fn on_auth_ok(&mut self) {
        self.authenticated = true;
    }

    pub const fn is_authenticated(self) -> bool {
        self.authenticated
    }

    /// Phân loại kind trước khi decode: kind phòng trước `AuthOk` bị từ chối; kind khác trả Ok
    /// để tầng handshake/auth xử lý theo luồng của nó.
    pub fn classify(self, kind: MessageKind) -> Result<(), RoomGateError> {
        if kind.is_room() && !self.authenticated {
            return Err(RoomGateError::RequiresAuth(kind));
        }
        Ok(())
    }
}

/// Đổi `RoomView` của `boom-room` sang DTO `RoomState`.
pub fn to_room_state(view: &RoomView) -> RoomState {
    RoomState {
        room_id: view.id.0,
        host: view.host.0,
        phase: phase_to_dto(view.phase),
        members: view.members.iter().map(to_member).collect(),
        round: view.round.as_ref().map(to_round),
    }
}

/// Đổi DTO `RoomState` về `RoomView`, kiểm các bất biến tối thiểu mà DTO có thể chở.
pub fn from_room_state(state: &RoomState) -> Result<RoomView, RoomSyncError> {
    let mut members = Vec::with_capacity(state.members.len());
    let mut slots = Vec::new();
    let mut players = Vec::new();
    for member in &state.members {
        if slots.contains(&member.slot) {
            return Err(RoomSyncError::DuplicateSlot(member.slot));
        }
        if players.contains(&member.player) {
            return Err(RoomSyncError::DuplicatePlayer(member.player));
        }
        slots.push(member.slot);
        players.push(member.player);
        members.push(MemberView {
            player: PlayerId(member.player),
            slot: SlotId(member.slot),
            character: character_from_dto(member.character)?,
            ready: member.ready,
        });
    }
    if !players.contains(&state.host) {
        return Err(RoomSyncError::HostNotMember(state.host));
    }
    let round = match &state.round {
        Some(round) => Some(RoundView {
            id: boom_room::RoundId(round.id),
            members: round
                .members
                .iter()
                .map(|member| {
                    if !players.contains(&member.player) {
                        return Err(RoomSyncError::RoundMemberNotInRoom(member.player));
                    }
                    Ok(RoundMember {
                        player: PlayerId(member.player),
                        slot: SlotId(member.slot),
                        character: character_from_dto(member.character)?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
        None => None,
    };
    Ok(RoomView {
        id: boom_room::RoomId(state.room_id),
        host: PlayerId(state.host),
        phase: phase_from_dto(state.phase),
        members,
        round,
    })
}

/// Đổi lệnh phòng core sang DTO.
pub fn to_room_command(command: &RoomCommand) -> RoomCommandDto {
    RoomCommandDto {
        player: command.player.0,
        request: command.request.0,
        action: action_to_dto(&command.action),
    }
}

/// Đổi lệnh phòng DTO về core; enum nhân vật đã được kiểm khi decode.
pub fn from_room_command(command: &RoomCommandDto) -> RoomCommand {
    RoomCommand {
        player: PlayerId(command.player),
        request: RequestId(command.request),
        action: action_from_dto(&command.action),
    }
}

/// Tạo danh sách sảnh từ các `RoomView`; tên phòng do server gắn sau (DTO không có trong RoomView).
pub fn to_lobby_list(views: &[RoomView]) -> LobbyList {
    LobbyList {
        rooms: views
            .iter()
            .map(|view| LobbyRoom {
                room_id: view.id.0,
                host: view.host.0,
                phase: phase_to_dto(view.phase),
                member_count: u8::try_from(view.members.len()).unwrap_or(u8::MAX),
                name: String::new(),
            })
            .collect(),
    }
}

/// Ánh xạ lỗi phòng sang lý do trên dây.
pub fn reject_reason(error: &RoomError) -> RoomRejectReason {
    match error {
        RoomError::PlayerAlreadyInRoom { .. } => RoomRejectReason::PlayerAlreadyInRoom,
        RoomError::PlayerNotInRoom { .. } => RoomRejectReason::PlayerNotInRoom,
        RoomError::RoomNotFound { .. } => RoomRejectReason::RoomNotFound,
        RoomError::RoomFull { .. } => RoomRejectReason::RoomFull,
        RoomError::NotHost { .. } => RoomRejectReason::NotHost,
        RoomError::CannotKickSelf { .. } => RoomRejectReason::CannotKickSelf,
        RoomError::TargetNotInRoom { .. } => RoomRejectReason::TargetNotInRoom,
        RoomError::NotWaiting { .. } => RoomRejectReason::NotWaiting,
        RoomError::AlreadyStarted { .. } => RoomRejectReason::AlreadyStarted,
        RoomError::NotEnoughPlayers { .. } => RoomRejectReason::NotEnoughPlayers,
        RoomError::NotAllReady { .. } => RoomRejectReason::NotAllReady,
        RoomError::StaleRequest { .. } => RoomRejectReason::StaleRequest,
        RoomError::IdExhausted => RoomRejectReason::IdExhausted,
    }
}

fn phase_to_dto(phase: RoomPhase) -> RoomPhaseDto {
    match phase {
        RoomPhase::Waiting => RoomPhaseDto::Waiting,
        RoomPhase::Started => RoomPhaseDto::Started,
    }
}

fn phase_from_dto(phase: RoomPhaseDto) -> RoomPhase {
    match phase {
        RoomPhaseDto::Waiting => RoomPhase::Waiting,
        RoomPhaseDto::Started => RoomPhase::Started,
    }
}

fn character_from_dto(character: RoomCharacter) -> Result<CharacterId, RoomSyncError> {
    match character {
        RoomCharacter::C00 => Ok(CharacterId::C00),
        RoomCharacter::C01 => Ok(CharacterId::C01),
    }
}

fn character_to_dto(character: CharacterId) -> RoomCharacter {
    match character {
        CharacterId::C00 => RoomCharacter::C00,
        CharacterId::C01 => RoomCharacter::C01,
    }
}

fn to_member(member: &MemberView) -> RoomMemberDto {
    RoomMemberDto {
        player: member.player.0,
        slot: member.slot.0,
        character: character_to_dto(member.character),
        ready: member.ready,
    }
}

fn to_round(round: &RoundView) -> RoundInfo {
    RoundInfo {
        id: round.id.0,
        members: round
            .members
            .iter()
            .map(|member| RoundMemberDto {
                player: member.player.0,
                slot: member.slot.0,
                character: character_to_dto(member.character),
            })
            .collect(),
    }
}

fn action_to_dto(action: &RoomAction) -> RoomActionDto {
    match action {
        RoomAction::Create { character } => RoomActionDto::Create {
            character: character_to_dto(*character),
        },
        RoomAction::Join { room, character } => RoomActionDto::Join {
            room: room.0,
            character: character_to_dto(*character),
        },
        RoomAction::Leave => RoomActionDto::Leave,
        RoomAction::Kick { target } => RoomActionDto::Kick { target: target.0 },
        RoomAction::SelectCharacter { character } => RoomActionDto::SelectCharacter {
            character: character_to_dto(*character),
        },
        RoomAction::SetReady { ready } => RoomActionDto::SetReady { ready: *ready },
        RoomAction::Start => RoomActionDto::Start,
    }
}

fn action_from_dto(action: &RoomActionDto) -> RoomAction {
    match action {
        RoomActionDto::Create { character } => RoomAction::Create {
            character: character_from_dto(*character).expect("enum đã kiểm"),
        },
        RoomActionDto::Join { room, character } => RoomAction::Join {
            room: boom_room::RoomId(*room),
            character: character_from_dto(*character).expect("enum đã kiểm"),
        },
        RoomActionDto::Leave => RoomAction::Leave,
        RoomActionDto::Kick { target } => RoomAction::Kick {
            target: PlayerId(*target),
        },
        RoomActionDto::SelectCharacter { character } => RoomAction::SelectCharacter {
            character: character_from_dto(*character).expect("enum đã kiểm"),
        },
        RoomActionDto::SetReady { ready } => RoomAction::SetReady { ready: *ready },
        RoomActionDto::Start => RoomAction::Start,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_room::{Lobby, RoomPolicy};

    fn view() -> RoomView {
        let mut lobby = Lobby::new(RoomPolicy::new(4, 2).unwrap());
        let _ = lobby
            .apply(RoomCommand {
                player: PlayerId(7),
                request: RequestId(1),
                action: RoomAction::Create {
                    character: CharacterId::C01,
                },
            })
            .unwrap();
        lobby.room(boom_room::RoomId(1)).unwrap()
    }

    #[test]
    fn room_view_round_trip_through_dto() {
        let original = view();
        let dto = to_room_state(&original);
        let back = from_room_state(&dto).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn started_round_view_round_trips_through_dto() {
        let mut lobby = Lobby::new(RoomPolicy::new(4, 2).unwrap());
        for (player, request, action) in [
            (
                7,
                1,
                RoomAction::Create {
                    character: CharacterId::C00,
                },
            ),
            (
                8,
                1,
                RoomAction::Join {
                    room: boom_room::RoomId(1),
                    character: CharacterId::C01,
                },
            ),
            (7, 2, RoomAction::SetReady { ready: true }),
            (8, 2, RoomAction::SetReady { ready: true }),
            (7, 3, RoomAction::Start),
        ] {
            lobby
                .apply(RoomCommand {
                    player: PlayerId(player),
                    request: RequestId(request),
                    action,
                })
                .unwrap();
        }
        let original = lobby.room(boom_room::RoomId(1)).unwrap();
        assert!(original.round.is_some());
        let dto = to_room_state(&original);
        assert_eq!(from_room_state(&dto).unwrap(), original);
    }

    #[test]
    fn command_round_trip_through_dto() {
        let original = RoomCommand {
            player: PlayerId(7),
            request: RequestId(42),
            action: RoomAction::Join {
                room: boom_room::RoomId(3),
                character: CharacterId::C01,
            },
        };
        let dto = to_room_command(&original);
        assert_eq!(from_room_command(&dto), original);

        for action in [
            RoomAction::Create {
                character: CharacterId::C00,
            },
            RoomAction::Join {
                room: boom_room::RoomId(3),
                character: CharacterId::C01,
            },
            RoomAction::Leave,
            RoomAction::Kick {
                target: PlayerId(9),
            },
            RoomAction::SelectCharacter {
                character: CharacterId::C01,
            },
            RoomAction::SetReady { ready: true },
            RoomAction::Start,
        ] {
            let command = RoomCommand {
                player: PlayerId(7),
                request: RequestId(42),
                action,
            };
            assert_eq!(
                from_room_command(&to_room_command(&command)),
                command,
                "mọi RoomAction phải ánh xạ 1-1"
            );
        }
    }

    #[test]
    fn lobby_list_uses_room_member_count() {
        let list = to_lobby_list(&[view()]);
        assert_eq!(list.rooms.len(), 1);
        assert_eq!(list.rooms[0].member_count, 1);
        assert_eq!(list.rooms[0].room_id, 1);
    }

    #[test]
    fn gate_requires_auth_ok() {
        let mut gate = RoomGate::new();
        assert!(gate.classify(MessageKind::RoomCommand).is_err());
        assert!(gate.classify(MessageKind::RoomState).is_err());
        gate.on_auth_ok();
        assert!(gate.classify(MessageKind::RoomCommand).is_ok());
        assert!(gate.classify(MessageKind::RoomState).is_ok());
    }

    #[test]
    fn reject_reason_covers_room_error() {
        assert_eq!(
            reject_reason(&RoomError::StaleRequest {
                player: PlayerId(1),
                last: RequestId(2),
                received: RequestId(1),
            }),
            RoomRejectReason::StaleRequest
        );
    }
}
