//! DTO phòng/sảnh/vòng đời (ADR 0009 Addendum B).
//!
//! Các message dưới đây là **additive** với `wire_schema = 1`; chúng chỉ hợp lệ trên kênh game
//! sau `AuthOk` (Addendum A). `boom-proto` không phụ thuộc `boom-room`: DTO dùng số/enum ổn định,
//! adapter nằm ở `boom-sync::room`.

use serde::{Deserialize, Serialize};

use crate::dto::bounded_str;
use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::{
    ProtocolLimits, WIRE_MAX_CHAT_BYTES, WIRE_MAX_LOBBY_ROOMS, WIRE_MAX_NAME_BYTES,
    WIRE_MAX_PLAYERS, WIRE_MAX_ROUND_MEMBERS, WIRE_MAX_ROUND_RESULTS,
};
use crate::message::Message;

/// Template nhân vật trên dây; mã khớp `boom_room::CharacterId` (C00=0, C01=1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomCharacter {
    C00,
    C01,
}

impl RoomCharacter {
    pub const fn code(self) -> u8 {
        match self {
            Self::C00 => 0,
            Self::C01 => 1,
        }
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::C00),
            1 => Some(Self::C01),
            _ => None,
        }
    }
}

/// Pha phòng. `Loading`/`Playing`/`Results` thuộc vòng đời round và được biểu diễn bằng
/// `RoundStart`/`RoundResult`; DTO này chỉ cần waiting/started như `boom-room`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomPhase {
    Waiting,
    Started,
}

/// Một thành viên trong `RoomState`.
///
/// `name` là **tên hiển thị**, do **server** điền từ account store (`AuthOk.display_name`),
/// không phải client khai (ADR 0005, ADR 0009 Addendum B §B.6). `name` là field **cuối** ⇒
/// không đổi thứ tự/kiểu của field cũ (thay đổi đó mới là tăng `wire_schema`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomMember {
    pub player: u64,
    pub slot: u8,
    pub character: RoomCharacter,
    pub ready: bool,
    /// Tên hiển thị của thành viên (server điền; xem ADR 0009 Addendum B §B.6).
    #[serde(deserialize_with = "de_name")]
    pub name: String,
}

/// Một thành viên roster đã chốt lúc start.
///
/// `name` là **tên hiển thị** như [`RoomMember::name`]: server điền từ account store, kể cả
/// người đã rời phòng sau `Start` (roster là bản lịch sử — ADR 0009 Addendum B §B.2/§B.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundMember {
    pub player: u64,
    pub slot: u8,
    pub character: RoomCharacter,
    /// Tên hiển thị của thành viên roster (server điền; xem ADR 0009 Addendum B §B.6).
    #[serde(deserialize_with = "de_name")]
    pub name: String,
}

/// Round đã chốt trong `RoomState`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundInfo {
    pub id: u64,
    #[serde(deserialize_with = "de_round_members")]
    pub members: Vec<RoundMember>,
}

/// Lệnh phòng ánh xạ 1-1 `boom_room::RoomAction`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomAction {
    Create { character: RoomCharacter },
    Join { room: u64, character: RoomCharacter },
    Leave,
    Kick { target: u64 },
    SelectCharacter { character: RoomCharacter },
    SetReady { ready: bool },
    Start,
}

/// `RoomCommand` (C→S, kind `0x0040`): lệnh phòng + request ID. **Không** mang player: server
/// bind player từ phiên đã `AuthOk` (ADR 0005, Addendum A).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCommand {
    pub request: u64,
    pub action: RoomAction,
}

/// `RoomState` (S→C, kind `0x0041`): snapshot phòng.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomState {
    pub room_id: u64,
    pub host: u64,
    pub phase: RoomPhase,
    #[serde(deserialize_with = "de_room_members")]
    pub members: Vec<RoomMember>,
    pub round: Option<RoundInfo>,
}

/// Một phòng trong danh sách sảnh.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyRoom {
    pub room_id: u64,
    pub host: u64,
    pub phase: RoomPhase,
    pub member_count: u8,
    #[serde(deserialize_with = "de_name")]
    pub name: String,
}

/// `LobbyListRequest` (C→S, kind `0x0042`): xin danh sách phòng đang chờ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyListRequest {}

/// `LobbyList` (S→C, kind `0x0043`): danh sách phòng + số người.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyList {
    #[serde(deserialize_with = "de_lobby_rooms")]
    pub rooms: Vec<LobbyRoom>,
}

/// `RoomChat` (C→S, kind `0x0044`): chat phòng. Server bind room/player từ phiên; không chạy
/// slash command, chỉ chuyển text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomChat {
    pub request: u64,
    #[serde(deserialize_with = "de_chat")]
    pub text: String,
}

/// `RoomChatMessage` (S→C, kind `0x0045`): chat đã được server nhận.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomChatMessage {
    pub room_id: u64,
    pub from: u64,
    #[serde(deserialize_with = "de_name")]
    pub from_name: String,
    #[serde(deserialize_with = "de_chat")]
    pub text: String,
}

/// `LoadingAck` (C→S, kind `0x0046`): client đã sẵn sàng vào round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadingAck {
    pub round_id: u64,
}

/// `ResultAck` (C→S, kind `0x0047`): client đã nhận kết quả.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultAck {
    pub round_id: u64,
}

/// Xin lại kết quả đã commit; server bind account từ phiên (§B.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundResultRequest {
    pub round_id: u64,
}

impl RoundResultRequest {
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

/// `RoundStart` (S→C, kind `0x0048`): thông báo bắt đầu round, đủ seed/rules/map/roster.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundStart {
    pub round_id: u64,
    pub seed: u64,
    #[serde(deserialize_with = "de_name")]
    pub rules_profile_id: String,
    pub rules_hash: u64,
    pub content_sha256: [u8; 32],
    #[serde(deserialize_with = "de_name")]
    pub map: String,
    #[serde(deserialize_with = "de_name")]
    pub mode: String,
    #[serde(deserialize_with = "de_round_members")]
    pub roster: Vec<RoundMember>,
    pub start_tick: u32,
}

/// Kết quả một người trong `RoundResult`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerResult {
    pub player: u64,
    pub slot: u8,
    pub rank: u8,
    pub alive: bool,
}

/// Kết quả round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomOutcome {
    Draw,
    Winner(u64),
}

/// `RoundResult` (S→C, kind `0x0049`): kết quả round (V1-022).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundResult {
    pub round_id: u64,
    pub outcome: RoomOutcome,
    #[serde(deserialize_with = "de_round_results")]
    pub results: Vec<PlayerResult>,
}

/// Lý do lệnh phòng bị từ chối; mã ổn định theo thứ tự khai báo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomRejectReason {
    PlayerAlreadyInRoom,
    PlayerNotInRoom,
    RoomNotFound,
    RoomFull,
    NotHost,
    CannotKickSelf,
    TargetNotInRoom,
    NotWaiting,
    AlreadyStarted,
    NotEnoughPlayers,
    NotAllReady,
    StaleRequest,
    IdExhausted,
}

impl RoomRejectReason {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// `RoomRejected` (S→C, kind `0x004A`): lệnh phòng bị từ chối, giữ request ID để client bỏ dedup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomRejected {
    pub request: u64,
    pub reason: RoomRejectReason,
}

fn de_room_members<'de, D>(d: D) -> Result<Vec<RoomMember>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, RoomMember, WIRE_MAX_PLAYERS>(d)
}

fn de_round_members<'de, D>(d: D) -> Result<Vec<RoundMember>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, RoundMember, WIRE_MAX_ROUND_MEMBERS>(d)
}

fn de_round_results<'de, D>(d: D) -> Result<Vec<PlayerResult>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, PlayerResult, WIRE_MAX_ROUND_RESULTS>(d)
}

fn de_lobby_rooms<'de, D>(d: D) -> Result<Vec<LobbyRoom>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, LobbyRoom, WIRE_MAX_LOBBY_ROOMS>(d)
}

fn de_name<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    bounded_str::<D, WIRE_MAX_NAME_BYTES>(d)
}

/// Kiểm độ dài một tên hiển thị theo miền phiên (`MAX_NAME_BYTES`).
///
/// Tên hiển thị do server điền (ADR 0009 Addendum B §B.6) nhưng client vẫn kiểm: miền phiên
/// là hợp đồng của **cả hai** đầu, tên vượt trần là lỗi giao thức (không phải lỗi hiển thị).
fn check_name(field: &'static str, name: &str, limits: &ProtocolLimits) -> Result<(), ProtoError> {
    if name.len() > limits.max_name_bytes {
        return Err(ProtoError::Invalid {
            field,
            detail: format!(
                "{} byte vượt trần {} byte",
                name.len(),
                limits.max_name_bytes
            ),
        });
    }
    Ok(())
}

fn de_chat<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    bounded_str::<D, WIRE_MAX_CHAT_BYTES>(d)
}

impl RoomState {
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.members.len() > limits.max_players {
            return Err(ProtoError::CountLimit {
                field: "room.members",
                len: self.members.len(),
                max: limits.max_players,
            });
        }
        let mut slots = Vec::new();
        let mut players = Vec::new();
        for member in &self.members {
            if !slots.iter().all(|s| *s != member.slot) {
                return Err(ProtoError::Invalid {
                    field: "room.members.slot",
                    detail: "slot phải duy nhất".into(),
                });
            }
            slots.push(member.slot);
            if !players.iter().all(|p| *p != member.player) {
                return Err(ProtoError::Invalid {
                    field: "room.members.player",
                    detail: "player phải duy nhất".into(),
                });
            }
            players.push(member.player);
            check_name("room.members.name", &member.name, limits)?;
        }
        if !players.contains(&self.host) {
            return Err(ProtoError::Invalid {
                field: "room.host",
                detail: "host phải là thành viên phòng".into(),
            });
        }
        if let Some(round) = &self.round
            && round.members.len() > limits.max_players
        {
            return Err(ProtoError::CountLimit {
                field: "room.round.members",
                len: round.members.len(),
                max: limits.max_players,
            });
        }
        if let Some(round) = &self.round {
            for member in &round.members {
                check_name("room.round.members.name", &member.name, limits)?;
            }
        }
        Ok(())
    }
}

impl LobbyList {
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.rooms.len() > limits.max_lobby_rooms {
            return Err(ProtoError::CountLimit {
                field: "lobby.rooms",
                len: self.rooms.len(),
                max: limits.max_lobby_rooms,
            });
        }
        for room in &self.rooms {
            if room.name.len() > limits.max_name_bytes {
                return Err(ProtoError::Invalid {
                    field: "lobby.rooms.name",
                    detail: format!(
                        "{} byte vượt trần {} byte",
                        room.name.len(),
                        limits.max_name_bytes
                    ),
                });
            }
            if usize::from(room.member_count) > limits.max_players {
                return Err(ProtoError::Invalid {
                    field: "lobby.rooms.member_count",
                    detail: "member_count vượt số người tối đa".into(),
                });
            }
        }
        Ok(())
    }
}

impl RoomChat {
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.text.len() > limits.max_chat_bytes {
            return Err(ProtoError::Invalid {
                field: "chat.text",
                detail: format!(
                    "{} byte vượt trần {} byte",
                    self.text.len(),
                    limits.max_chat_bytes
                ),
            });
        }
        Ok(())
    }
}

impl RoomChatMessage {
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.text.len() > limits.max_chat_bytes {
            return Err(ProtoError::Invalid {
                field: "chat.text",
                detail: "text vượt trần".into(),
            });
        }
        if self.from_name.len() > limits.max_name_bytes {
            return Err(ProtoError::Invalid {
                field: "chat.from_name",
                detail: "from_name vượt trần".into(),
            });
        }
        Ok(())
    }
}

impl RoundStart {
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.rules_profile_id.len() > limits.max_name_bytes
            || self.map.len() > limits.max_name_bytes
            || self.mode.len() > limits.max_name_bytes
        {
            return Err(ProtoError::Invalid {
                field: "round_start.name",
                detail: "rules_profile_id/map vượt trần tên".into(),
            });
        }
        if self.roster.len() > limits.max_players {
            return Err(ProtoError::CountLimit {
                field: "round_start.roster",
                len: self.roster.len(),
                max: limits.max_players,
            });
        }
        for member in &self.roster {
            check_name("round_start.roster.name", &member.name, limits)?;
        }
        Ok(())
    }
}

impl RoundResult {
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.results.len() > limits.max_players {
            return Err(ProtoError::CountLimit {
                field: "round_result.results",
                len: self.results.len(),
                max: limits.max_players,
            });
        }
        Ok(())
    }
}

macro_rules! impl_message {
    ($ty:ty, $kind:expr) => {
        impl Message for $ty {
            const KIND: MessageKind = $kind;

            fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
                self.check(limits)
            }
        }
    };
}

impl RoomCommand {
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl LobbyListRequest {
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl LoadingAck {
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl ResultAck {
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl RoomRejected {
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl_message!(RoomCommand, MessageKind::RoomCommand);
impl_message!(RoomState, MessageKind::RoomState);
impl_message!(LobbyListRequest, MessageKind::LobbyListRequest);
impl_message!(LobbyList, MessageKind::LobbyList);
impl_message!(RoomChat, MessageKind::RoomChat);
impl_message!(RoomChatMessage, MessageKind::RoomChatMessage);
impl_message!(LoadingAck, MessageKind::LoadingAck);
impl_message!(ResultAck, MessageKind::ResultAck);
impl_message!(RoundResultRequest, MessageKind::RoundResultRequest);
impl_message!(RoundStart, MessageKind::RoundStart);
impl_message!(RoundResult, MessageKind::RoundResult);
impl_message!(RoomRejected, MessageKind::RoomRejected);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{decode_payload, encode_payload};

    fn round_trip<T: Message + PartialEq + std::fmt::Debug>(value: &T) {
        let bytes = encode_payload(value).unwrap();
        let decoded: T = decode_payload(&bytes).unwrap();
        assert_eq!(&decoded, value);
    }

    fn room_state() -> RoomState {
        RoomState {
            room_id: 1,
            host: 7,
            phase: RoomPhase::Waiting,
            members: vec![RoomMember {
                player: 7,
                slot: 0,
                character: RoomCharacter::C00,
                ready: true,
                name: "a".into(),
            }],
            round: None,
        }
    }

    #[test]
    fn room_command_golden_bytes() {
        let cmd = RoomCommand {
            request: 42,
            action: RoomAction::Join {
                room: 3,
                character: RoomCharacter::C01,
            },
        };
        let bytes = encode_payload(&cmd).unwrap();
        assert_eq!(bytes, vec![0x2a, 0x01, 0x03, 0x01]);
        let decoded: RoomCommand = decode_payload(&bytes).unwrap();
        assert_eq!(decoded, cmd);
    }

    /// Golden byte của `RoomState` **có** tên hiển thị (ADR 0009 Addendum B §B.6).
    ///
    /// `RoomState { room_id=1, host=7, phase=Waiting, members=[{player=7, slot=0, C00,
    /// ready=true, name="a"}], round=None }`:
    /// `01` room_id · `07` host · `00` phase · `01` số member · `07` player · `00` slot ·
    /// `00` character · `01` ready · `01` độ dài tên · `61` byte 'a' · `00` round = None.
    #[test]
    fn room_state_golden_bytes_with_display_name() {
        let bytes = encode_payload(&room_state()).unwrap();
        assert_eq!(
            bytes,
            vec![
                0x01, 0x07, 0x00, 0x01, 0x07, 0x00, 0x00, 0x01, 0x01, 0x61, 0x00
            ]
        );
        let decoded: RoomState = decode_payload(&bytes).unwrap();
        assert_eq!(decoded, room_state());
    }

    #[test]
    fn every_room_dto_round_trips() {
        round_trip(&RoomCommand {
            request: 2,
            action: RoomAction::Create {
                character: RoomCharacter::C00,
            },
        });
        round_trip(&room_state());
        round_trip(&LobbyListRequest {});
        round_trip(&LobbyList {
            rooms: vec![LobbyRoom {
                room_id: 1,
                host: 7,
                phase: RoomPhase::Waiting,
                member_count: 1,
                name: "room".into(),
            }],
        });
        round_trip(&RoomChat {
            request: 1,
            text: "hi".into(),
        });
        round_trip(&RoomChatMessage {
            room_id: 1,
            from: 7,
            from_name: "p".into(),
            text: "hi".into(),
        });
        round_trip(&LoadingAck { round_id: 1 });
        round_trip(&ResultAck { round_id: 1 });
        round_trip(&RoundResultRequest { round_id: 300 });
        assert_eq!(
            encode_payload(&RoundResultRequest { round_id: 300 }).unwrap(),
            vec![0xac, 0x02]
        );
        round_trip(&RoundStart {
            round_id: 1,
            seed: 2,
            rules_profile_id: "p".into(),
            rules_hash: 3,
            content_sha256: [0; 32],
            map: "Village00".into(),
            mode: "battle_ffa".into(),
            roster: vec![RoundMember {
                player: 7,
                slot: 0,
                character: RoomCharacter::C00,
                name: "a".into(),
            }],
            start_tick: 0,
        });
        round_trip(&RoundResult {
            round_id: 1,
            outcome: RoomOutcome::Winner(7),
            results: vec![PlayerResult {
                player: 7,
                slot: 0,
                rank: 1,
                alive: true,
            }],
        });
        round_trip(&RoomRejected {
            request: 2,
            reason: RoomRejectReason::StaleRequest,
        });
    }

    #[test]
    fn rejects_over_limit_room_chat_name_and_count() {
        let limits = ProtocolLimits::v1();
        let chat = RoomChat {
            request: 1,
            text: "x".repeat(limits.max_chat_bytes + 1),
        };
        assert!(chat.check(&limits).is_err());

        let list = LobbyList {
            rooms: (0..=limits.max_lobby_rooms)
                .map(|i| LobbyRoom {
                    room_id: i as u64,
                    host: 1,
                    phase: RoomPhase::Waiting,
                    member_count: 1,
                    name: "room".into(),
                })
                .collect(),
        };
        assert!(list.check(&limits).is_err());

        let name = LobbyList {
            rooms: vec![LobbyRoom {
                room_id: 1,
                host: 1,
                phase: RoomPhase::Waiting,
                member_count: 1,
                name: "x".repeat(limits.max_name_bytes + 1),
            }],
        };
        assert!(name.check(&limits).is_err());

        let mut state = room_state();
        state.members = (0..=limits.max_players)
            .map(|slot| RoomMember {
                player: slot as u64,
                slot: slot as u8,
                character: RoomCharacter::C00,
                ready: false,
                name: "a".into(),
            })
            .collect();
        state.host = 0;
        assert!(state.check(&limits).is_err());
    }

    /// Tên hiển thị vượt trần phiên bị từ chối bằng lỗi có kiểu (ADR 0009 Addendum B §B.6).
    ///
    /// Tên quá trần **định dạng** (`WIRE_MAX_NAME_BYTES`) bị chặn lúc decode; tên vượt trần
    /// **phiên** (`MAX_NAME_BYTES`) decode được rồi bị `check` từ chối — cùng cách
    /// `LobbyRoom.name` làm.
    #[test]
    fn ten_hoi_thi_vuot_tran_phien_bi_tu_choi() {
        let limits = ProtocolLimits::v1();

        let mut state = room_state();
        state.members[0].name = "x".repeat(limits.max_name_bytes + 1);
        let error = state
            .check(&limits)
            .expect_err("tên vượt trần phải bị từ chối");
        assert!(
            matches!(&error, ProtoError::Invalid { field, .. } if *field == "room.members.name"),
            "lỗi phải nêu đúng trường, thấy {error:?}"
        );

        let mut state = room_state();
        state.round = Some(RoundInfo {
            id: 1,
            members: vec![RoundMember {
                player: 7,
                slot: 0,
                character: RoomCharacter::C00,
                name: "x".repeat(limits.max_name_bytes + 1),
            }],
        });
        let error = state
            .check(&limits)
            .expect_err("tên roster vượt trần phải bị từ chối");
        assert!(
            matches!(&error, ProtoError::Invalid { field, .. } if *field == "room.round.members.name"),
            "lỗi phải nêu đúng trường, thấy {error:?}"
        );

        let start = RoundStart {
            round_id: 1,
            seed: 2,
            rules_profile_id: "p".into(),
            rules_hash: 3,
            content_sha256: [0; 32],
            map: "Village00".into(),
            mode: "battle_ffa".into(),
            roster: vec![RoundMember {
                player: 7,
                slot: 0,
                character: RoomCharacter::C00,
                name: "x".repeat(limits.max_name_bytes + 1),
            }],
            start_tick: 0,
        };
        let error = start
            .check(&limits)
            .expect_err("tên roster vượt trần phải bị từ chối");
        assert!(
            matches!(&error, ProtoError::Invalid { field, .. } if *field == "round_start.roster.name"),
            "lỗi phải nêu đúng trường, thấy {error:?}"
        );
    }

    /// Tên vượt trần **định dạng** bị chặn lúc decode (không cấp phát theo length prefix).
    #[test]
    fn ten_hoi_thi_vuot_tran_dinh_dang_bi_tu_choi_luc_decode() {
        let mut state = room_state();
        state.members[0].name = "x".repeat(WIRE_MAX_NAME_BYTES + 1);
        let bytes = encode_payload(&state).unwrap();
        assert!(
            decode_payload::<RoomState>(&bytes).is_err(),
            "tên vượt trần định dạng phải bị từ chối lúc decode"
        );
    }

    #[test]
    fn round_roster_may_include_player_who_left_after_start() {
        let mut state = room_state();
        state.round = Some(RoundInfo {
            id: 1,
            members: vec![
                RoundMember {
                    player: 7,
                    slot: 0,
                    character: RoomCharacter::C00,
                    name: "a".into(),
                },
                RoundMember {
                    player: 8,
                    slot: 1,
                    character: RoomCharacter::C01,
                    name: "b".into(),
                },
            ],
        });
        // members hiện tại chỉ còn 7; roster lịch sử giữ 8 đã rời sau Start.
        assert!(state.check(&ProtocolLimits::v1()).is_ok());
    }

    #[test]
    fn room_character_codes_are_stable() {
        assert_eq!(RoomCharacter::C00.code(), 0);
        assert_eq!(RoomCharacter::C01.code(), 1);
        assert_eq!(RoomCharacter::from_code(0), Some(RoomCharacter::C00));
        assert_eq!(RoomCharacter::from_code(1), Some(RoomCharacter::C01));
        assert_eq!(RoomCharacter::from_code(2), None);
    }
}
