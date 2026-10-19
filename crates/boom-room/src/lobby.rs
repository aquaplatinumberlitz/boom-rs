//! Máy trạng thái phòng thuần: create/join/leave/kick, chọn nhân vật, ready và start.
//!
//! Một [`Lobby`] giữ nhiều phòng, bảo đảm một người chỉ ở một phòng và ID phòng/lượt duy nhất.
//! Mọi lệnh đi qua [`Lobby::apply`] tuần tự nên join là atomic ở mức state machine; server P7
//! chịu trách nhiệm tuần tự hoá request đến và xác thực danh tính trước khi bind [`PlayerId`].

use std::collections::BTreeMap;

use crate::model::{
    CharacterId, MemberView, Outcome, PlayerId, ReadyResetReason, RequestId, RoomAction,
    RoomCommand, RoomError, RoomEvent, RoomId, RoomPhase, RoomPolicy, RoomView, RoundId,
    RoundMember, RoundView, SlotId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Member {
    player: PlayerId,
    slot: SlotId,
    character: CharacterId,
    ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartedRound {
    id: RoundId,
    members: Vec<RoundMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Room {
    id: RoomId,
    host: PlayerId,
    phase: RoomPhase,
    /// Thứ tự vào phòng, dùng để chọn host kế nhiệm (người vào sớm nhất còn lại).
    members: Vec<Member>,
    round: Option<StartedRound>,
}

impl Room {
    fn member_mut(&mut self, player: PlayerId) -> Option<&mut Member> {
        self.members.iter_mut().find(|m| m.player == player)
    }

    fn free_slot(&self, max_players: u8) -> Option<SlotId> {
        (0..max_players)
            .map(SlotId)
            .find(|slot| self.members.iter().all(|m| m.slot != *slot))
    }

    /// Hủy ready của mọi thành viên; trả event chỉ khi có ít nhất một ready bị hủy.
    /// `cleared` sắp theo slot để event ổn định.
    fn reset_ready(&mut self, reason: ReadyResetReason) -> Option<RoomEvent> {
        let mut cleared: Vec<(SlotId, PlayerId)> = self
            .members
            .iter()
            .filter(|m| m.ready)
            .map(|m| (m.slot, m.player))
            .collect();
        if cleared.is_empty() {
            return None;
        }
        cleared.sort_by_key(|(slot, _)| *slot);
        let cleared: Vec<PlayerId> = cleared.into_iter().map(|(_, player)| player).collect();
        for member in &mut self.members {
            member.ready = false;
        }
        Some(RoomEvent::ReadyReset {
            room: self.id,
            reason,
            cleared,
        })
    }

    fn sorted_members(&self) -> Vec<MemberView> {
        let mut members: Vec<MemberView> = self
            .members
            .iter()
            .map(|m| MemberView {
                player: m.player,
                slot: m.slot,
                character: m.character,
                ready: m.ready,
            })
            .collect();
        members.sort_by_key(|m| m.slot);
        members
    }

    fn view(&self) -> RoomView {
        RoomView {
            id: self.id,
            host: self.host,
            phase: self.phase,
            members: self.sorted_members(),
            round: self.round.as_ref().map(|r| RoundView {
                id: r.id,
                members: r.members.clone(),
            }),
        }
    }
}

/// Lobby nhiều phòng; entry point duy nhất cho lệnh phòng.
#[derive(Clone, Debug)]
pub struct Lobby {
    policy: RoomPolicy,
    next_room: u64,
    next_round: u64,
    rooms: BTreeMap<RoomId, Room>,
    /// Một người chỉ có tối đa một phòng tại một thời điểm.
    seat: BTreeMap<PlayerId, RoomId>,
    /// Request ID thành công gần nhất của mỗi người, để dedup lệnh gửi lại.
    last_request: BTreeMap<PlayerId, RequestId>,
}

impl Lobby {
    pub fn new(policy: RoomPolicy) -> Self {
        Self {
            policy,
            next_room: 1,
            next_round: 1,
            rooms: BTreeMap::new(),
            seat: BTreeMap::new(),
            last_request: BTreeMap::new(),
        }
    }

    pub const fn policy(&self) -> RoomPolicy {
        self.policy
    }

    /// Xử lý một lệnh đã xác thực. Tuần tự nên hai join cạnh tranh slot cuối không thể cùng thắng.
    ///
    /// Dedup: cùng `request` với lệnh thành công gần nhất của người đó trả [`Outcome::Duplicate`]
    /// và không đổi state; request nhỏ hơn là [`RoomError::StaleRequest`]. Lệnh lỗi không ghi nhận
    /// request ID vì không đổi state, nên có thể thử lại cùng ID sau khi sửa điều kiện.
    pub fn apply(&mut self, command: RoomCommand) -> Result<Outcome, RoomError> {
        if let Some(&last) = self.last_request.get(&command.player) {
            if command.request == last {
                return Ok(Outcome::Duplicate);
            }
            if command.request < last {
                return Err(RoomError::StaleRequest {
                    player: command.player,
                    last,
                    received: command.request,
                });
            }
        }
        let events = self.dispatch(command.player, command.action)?;
        self.last_request.insert(command.player, command.request);
        Ok(Outcome::Applied(events))
    }

    fn dispatch(
        &mut self,
        player: PlayerId,
        action: RoomAction,
    ) -> Result<Vec<RoomEvent>, RoomError> {
        match action {
            RoomAction::Create { character } => self.create(player, character),
            RoomAction::Join { room, character } => self.join(player, room, character),
            RoomAction::Leave => self.leave(player),
            RoomAction::Kick { target } => self.kick(player, target),
            RoomAction::SelectCharacter { character } => self.select_character(player, character),
            RoomAction::SetReady { ready } => self.set_ready(player, ready),
            RoomAction::Start => self.start(player),
        }
    }

    fn alloc_room(&mut self) -> Result<RoomId, RoomError> {
        let id = self.next_room;
        self.next_room = self
            .next_room
            .checked_add(1)
            .ok_or(RoomError::IdExhausted)?;
        Ok(RoomId(id))
    }

    fn alloc_round(&mut self) -> Result<RoundId, RoomError> {
        let id = self.next_round;
        self.next_round = self
            .next_round
            .checked_add(1)
            .ok_or(RoomError::IdExhausted)?;
        Ok(RoundId(id))
    }

    fn room_of(&self, player: PlayerId) -> Result<RoomId, RoomError> {
        self.seat
            .get(&player)
            .copied()
            .ok_or(RoomError::PlayerNotInRoom { player })
    }

    fn create(
        &mut self,
        player: PlayerId,
        character: CharacterId,
    ) -> Result<Vec<RoomEvent>, RoomError> {
        if let Some(&room) = self.seat.get(&player) {
            return Err(RoomError::PlayerAlreadyInRoom { player, room });
        }
        let room_id = self.alloc_room()?;
        let room = Room {
            id: room_id,
            host: player,
            phase: RoomPhase::Waiting,
            members: vec![Member {
                player,
                slot: SlotId(0),
                character,
                ready: false,
            }],
            round: None,
        };
        self.rooms.insert(room_id, room);
        self.seat.insert(player, room_id);
        Ok(vec![RoomEvent::RoomCreated {
            room: room_id,
            host: player,
            character,
        }])
    }

    fn join(
        &mut self,
        player: PlayerId,
        room_id: RoomId,
        character: CharacterId,
    ) -> Result<Vec<RoomEvent>, RoomError> {
        if let Some(&room) = self.seat.get(&player) {
            return Err(RoomError::PlayerAlreadyInRoom { player, room });
        }
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        if room.phase != RoomPhase::Waiting {
            return Err(RoomError::NotWaiting {
                room: room_id,
                phase: room.phase,
            });
        }
        if room.members.len() >= usize::from(self.policy.max_players()) {
            return Err(RoomError::RoomFull {
                room: room_id,
                capacity: self.policy.max_players(),
            });
        }
        let slot = room
            .free_slot(self.policy.max_players())
            .ok_or(RoomError::RoomFull {
                room: room_id,
                capacity: self.policy.max_players(),
            })?;
        room.members.push(Member {
            player,
            slot,
            character,
            ready: false,
        });
        let mut events = vec![RoomEvent::PlayerJoined {
            room: room_id,
            player,
            slot,
            character,
        }];
        if let Some(reset) = room.reset_ready(ReadyResetReason::RosterChanged) {
            events.push(reset);
        }
        self.seat.insert(player, room_id);
        Ok(events)
    }

    fn leave(&mut self, player: PlayerId) -> Result<Vec<RoomEvent>, RoomError> {
        let room_id = self.room_of(player)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        let index = room
            .members
            .iter()
            .position(|m| m.player == player)
            .ok_or(RoomError::PlayerNotInRoom { player })?;
        room.members.remove(index);
        let mut events = vec![RoomEvent::PlayerLeft {
            room: room_id,
            player,
        }];
        self.seat.remove(&player);

        if room.members.is_empty() {
            self.rooms.remove(&room_id);
            return Ok(events);
        }
        if room.host == player {
            let previous = room.host;
            // Danh sách giữ thứ tự vào phòng, nên phần tử đầu là người vào sớm nhất còn lại.
            room.host = room.members[0].player;
            events.push(RoomEvent::HostChanged {
                room: room_id,
                host: room.host,
                previous,
            });
        }
        if room.phase == RoomPhase::Waiting
            && let Some(reset) = room.reset_ready(ReadyResetReason::RosterChanged)
        {
            events.push(reset);
        }
        Ok(events)
    }

    fn kick(&mut self, actor: PlayerId, target: PlayerId) -> Result<Vec<RoomEvent>, RoomError> {
        let room_id = self.room_of(actor)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        if room.phase != RoomPhase::Waiting {
            return Err(RoomError::NotWaiting {
                room: room_id,
                phase: room.phase,
            });
        }
        if room.host != actor {
            return Err(RoomError::NotHost {
                player: actor,
                room: room_id,
            });
        }
        if target == actor {
            return Err(RoomError::CannotKickSelf { player: actor });
        }
        let index = room.members.iter().position(|m| m.player == target).ok_or(
            RoomError::TargetNotInRoom {
                target,
                room: room_id,
            },
        )?;
        room.members.remove(index);
        let mut events = vec![RoomEvent::PlayerKicked {
            room: room_id,
            player: target,
            by: actor,
        }];
        self.seat.remove(&target);
        if let Some(reset) = room.reset_ready(ReadyResetReason::RosterChanged) {
            events.push(reset);
        }
        Ok(events)
    }

    fn select_character(
        &mut self,
        player: PlayerId,
        character: CharacterId,
    ) -> Result<Vec<RoomEvent>, RoomError> {
        let room_id = self.room_of(player)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        if room.phase != RoomPhase::Waiting {
            return Err(RoomError::NotWaiting {
                room: room_id,
                phase: room.phase,
            });
        }
        let member = room
            .member_mut(player)
            .ok_or(RoomError::PlayerNotInRoom { player })?;
        if member.character == character {
            return Ok(Vec::new());
        }
        member.character = character;
        let mut events = vec![RoomEvent::CharacterSelected {
            room: room_id,
            player,
            character,
        }];
        if let Some(reset) = room.reset_ready(ReadyResetReason::SelectionChanged) {
            events.push(reset);
        }
        Ok(events)
    }

    fn set_ready(&mut self, player: PlayerId, ready: bool) -> Result<Vec<RoomEvent>, RoomError> {
        let room_id = self.room_of(player)?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        if room.phase != RoomPhase::Waiting {
            return Err(RoomError::NotWaiting {
                room: room_id,
                phase: room.phase,
            });
        }
        let member = room
            .member_mut(player)
            .ok_or(RoomError::PlayerNotInRoom { player })?;
        if member.ready == ready {
            return Ok(Vec::new());
        }
        member.ready = ready;
        Ok(vec![RoomEvent::ReadyChanged {
            room: room_id,
            player,
            ready,
        }])
    }

    fn start(&mut self, player: PlayerId) -> Result<Vec<RoomEvent>, RoomError> {
        let room_id = self.room_of(player)?;
        let room = self
            .rooms
            .get(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        if room.phase == RoomPhase::Started {
            let round = room.round.as_ref().map(|r| r.id);
            return Err(match round {
                Some(round) => RoomError::AlreadyStarted {
                    room: room_id,
                    round,
                },
                None => RoomError::NotWaiting {
                    room: room_id,
                    phase: room.phase,
                },
            });
        }
        if room.host != player {
            return Err(RoomError::NotHost {
                player,
                room: room_id,
            });
        }
        let present = room.members.len();
        if present < usize::from(self.policy.min_players_to_start()) {
            return Err(RoomError::NotEnoughPlayers {
                room: room_id,
                required: self.policy.min_players_to_start(),
                present: u8::try_from(present).unwrap_or(u8::MAX),
            });
        }
        let mut waiting: Vec<(SlotId, PlayerId)> = room
            .members
            .iter()
            .filter(|m| !m.ready)
            .map(|m| (m.slot, m.player))
            .collect();
        waiting.sort_by_key(|(slot, _)| *slot);
        let waiting: Vec<PlayerId> = waiting.into_iter().map(|(_, player)| player).collect();
        if !waiting.is_empty() {
            return Err(RoomError::NotAllReady {
                room: room_id,
                waiting,
            });
        }

        let mut members: Vec<RoundMember> = room
            .members
            .iter()
            .map(|m| RoundMember {
                player: m.player,
                slot: m.slot,
                character: m.character,
            })
            .collect();
        members.sort_by_key(|m| m.slot);
        let round_id = self.alloc_round()?;
        let room = self
            .rooms
            .get_mut(&room_id)
            .ok_or(RoomError::RoomNotFound { room: room_id })?;
        room.phase = RoomPhase::Started;
        room.round = Some(StartedRound {
            id: round_id,
            members: members.clone(),
        });
        Ok(vec![RoomEvent::RoundStarted {
            room: room_id,
            round: round_id,
            members,
        }])
    }

    /// Snapshot một phòng; `None` nếu phòng đã bị xóa (hết thành viên).
    pub fn room(&self, room: RoomId) -> Option<RoomView> {
        self.rooms.get(&room).map(Room::view)
    }

    /// Toàn bộ phòng theo ID tăng dần; server tự lọc pha khi hiện danh sách sảnh.
    pub fn rooms(&self) -> Vec<RoomView> {
        self.rooms.values().map(Room::view).collect()
    }

    /// Phòng hiện tại của một người, nếu có.
    pub fn player_room(&self, player: PlayerId) -> Option<RoomId> {
        self.seat.get(&player).copied()
    }
}
