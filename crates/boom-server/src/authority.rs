//! Lõi authority thuần: `boom-room` → loading barrier → `boom-core` → kết quả → phòng.
//!
//! Không socket, không wall clock đọc trực tiếp: caller truyền `now_ms` và gọi `advance_to`.
//! Phần 2 sẽ bọc lõi này bằng `boom-proto`/`boom-sync`/`boom-net`; kiểu `Internal*` ở đây chỉ là
//! lệnh nội bộ, không phải DTO wire.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use boom_core::{
    CharacterStats, CoreError, Life, Outcome as CoreOutcome, Phase as CorePhase, PlayerInput,
    PlayerSetup, SimulationEvent, SimulationState, Tick, World,
};
use boom_proto::{InputBatch, Snapshot};
use boom_room::{
    CharacterId, Lobby, Outcome as RoomOutcome, PlayerId, RequestId, RoomAction, RoomCommand,
    RoomError, RoomEvent, RoomId, RoomView, RoundId, RoundMember,
};
use boom_sync::{SnapshotContext, input_ack, to_snapshot};

use crate::config::{CharacterStatsTable, InputTicks, ReplaySetup, ServerConfig};
use crate::input::{InputAck, InputError, InputTrack};

/// Kết quả trận do server quyết, một lần cho mỗi round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchOutcome {
    Winner(PlayerId),
    Draw,
    /// Tất cả rời/mất kết nối: hủy round, không tạo người thắng vắng mặt (SCOPE_V1 §3).
    Aborted,
}

/// Kết quả round kèm event của round. Đây là dữ liệu nội bộ; phần 2 map sang `RoundEvents` DTO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InternalRoundResult {
    pub round: RoundId,
    pub outcome: MatchOutcome,
    pub tick: Tick,
    pub seed: u64,
    pub events: Vec<SimulationEvent>,
}

/// Event authority phát cho lớp IO/log. Không phải DTO wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InternalServerEvent {
    LoadingAcked {
        room: RoomId,
        round: RoundId,
        player: PlayerId,
    },
    MatchStarted {
        room: RoomId,
        round: RoundId,
    },
    RoundFinished {
        room: RoomId,
        result: InternalRoundResult,
    },
    ReturnedToWaiting {
        room: RoomId,
        round: RoundId,
    },
    RoundCancelled {
        room: RoomId,
        round: Option<RoundId>,
    },
    PlayerDisconnected {
        room: RoomId,
        player: PlayerId,
        at_ms: u64,
    },
    PlayerEliminated {
        room: RoomId,
        player: PlayerId,
        at_ms: u64,
    },
    InputRetired {
        room: RoomId,
        round: RoundId,
        player: PlayerId,
        seq: u32,
    },
}

/// Pha vòng đời do authority quản lý. `boom-room` chỉ biết waiting/started; các pha còn lại ở đây.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerPhase {
    Waiting,
    Loading,
    Playing,
    Results,
    Closed,
}

impl ServerPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Loading => "loading",
            Self::Playing => "playing",
            Self::Results => "results",
            Self::Closed => "closed",
        }
    }
}

/// Kết quả một lệnh phòng. `duplicate` nghĩa request đã xử lý thành công trước đó.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomUpdate {
    pub events: Vec<RoomEvent>,
    pub duplicate: bool,
    pub round: Option<RoundId>,
}

#[derive(Debug)]
pub enum ServerError {
    Room(RoomError),
    Core(CoreError),
    Input(InputError),
    WrongPhase { phase: ServerPhase },
    NoRoom,
    RoomExists { room: RoomId },
    UnknownPlayer(PlayerId),
    Proto(boom_proto::ProtoError),
    Sync(boom_sync::SyncError),
    Net(boom_net::NetError),
    NoSession(PlayerId),
    SessionExists(PlayerId),
    TimeWentBackwards { last_ms: u64, received_ms: u64 },
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Room(e) => write!(f, "lỗi phòng: {e}"),
            Self::Core(e) => write!(f, "lỗi core: {e}"),
            Self::Input(e) => write!(f, "lỗi scheduler input: {e}"),
            Self::Proto(e) => write!(f, "lỗi giao thức: {e}"),
            Self::Sync(e) => write!(f, "lỗi đồng bộ DTO: {e}"),
            Self::Net(e) => write!(f, "lỗi transport: {e}"),
            Self::NoSession(player) => write!(f, "không có session cho {player:?}"),
            Self::SessionExists(player) => write!(f, "session {player:?} đã tồn tại"),
            Self::WrongPhase { phase } => {
                write!(f, "lệnh không hợp lệ ở pha {}", phase.label())
            }
            Self::NoRoom => write!(f, "chưa có phòng"),
            Self::RoomExists { room } => {
                write!(
                    f,
                    "authority đã quản lý phòng {room:?}; không tạo phòng thứ hai"
                )
            }
            Self::UnknownPlayer(p) => write!(f, "không biết người chơi {p:?}"),
            Self::TimeWentBackwards {
                last_ms,
                received_ms,
            } => {
                write!(f, "thời gian lùi: {received_ms} < {last_ms}")
            }
        }
    }
}

impl std::error::Error for ServerError {}

impl From<RoomError> for ServerError {
    fn from(value: RoomError) -> Self {
        Self::Room(value)
    }
}

impl From<CoreError> for ServerError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

impl From<InputError> for ServerError {
    fn from(value: InputError) -> Self {
        Self::Input(value)
    }
}

impl From<boom_proto::ProtoError> for ServerError {
    fn from(value: boom_proto::ProtoError) -> Self {
        Self::Proto(value)
    }
}

impl From<boom_sync::SyncError> for ServerError {
    fn from(value: boom_sync::SyncError) -> Self {
        Self::Sync(value)
    }
}

impl From<boom_net::NetError> for ServerError {
    fn from(value: boom_net::NetError) -> Self {
        Self::Net(value)
    }
}

/// Dữ liệu bất biến để dựng một trận: world (terrain/rules/mode) và chỉ số C00/C01.
#[derive(Clone, Debug)]
pub struct MatchSetup {
    pub world: World,
    pub stats: CharacterStatsTable,
    /// Metadata để ghi replay; `None` dù `replay_dir` bật thì không ghi.
    pub replay: Option<ReplaySetup>,
}

struct Match {
    room: RoomId,
    round: RoundId,
    seed: u64,
    members: Vec<RoundMember>,
    slot_to_player: BTreeMap<u8, PlayerId>,
    state: SimulationState,
    inputs: BTreeMap<PlayerId, InputTrack>,
    disconnected_at: BTreeMap<PlayerId, u64>,
    eliminated: BTreeSet<PlayerId>,
    events: Vec<SimulationEvent>,
    event_cursor: usize,
    result: Option<InternalRoundResult>,
}

impl Match {
    fn player_for_slot(&self, slot: u8) -> Option<PlayerId> {
        self.slot_to_player.get(&slot).copied()
    }
}

#[derive(Clone)]
enum Lifecycle {
    Waiting,
    Loading {
        started_ms: u64,
        acked: BTreeSet<PlayerId>,
    },
    Playing,
    Results {
        at_ms: u64,
        acked: BTreeSet<PlayerId>,
    },
    Closed,
}

/// Authority một phòng. V1 một kênh; có thể mở rộng nhiều phòng bằng nhiều `Authority`.
pub struct Authority {
    config: ServerConfig,
    setup: MatchSetup,
    input_limits: InputTicks,
    lobby: Lobby,
    public_room: Option<RoomId>,
    internal_room: Option<RoomId>,
    phase: Lifecycle,
    active: Option<Match>,
    replay: Option<crate::replay_log::ReplayLog>,
    last_result: Option<InternalRoundResult>,
    now_ms: u64,
    next_tick_ms: u64,
    tick_origin_ms: u64,
    ticks_elapsed: u64,
    tick_hz: u64,
    next_seed: u64,
    next_round: u64,
    current_public_round: Option<RoundId>,
    next_internal_request: u64,
    last_external_request: BTreeMap<PlayerId, RequestId>,
}

impl Authority {
    pub fn new(config: ServerConfig, setup: MatchSetup) -> Self {
        let input_limits = InputTicks::from_limits(config.input, setup.world.rules());
        let tick_hz = u64::from(setup.world.rules().ticks_per_second.max(1));
        Self {
            lobby: Lobby::new(config.policy.room_policy),
            config,
            setup,
            input_limits,
            public_room: None,
            internal_room: None,
            phase: Lifecycle::Waiting,
            active: None,
            replay: None,
            last_result: None,
            now_ms: 0,
            next_tick_ms: 0,
            tick_origin_ms: 0,
            ticks_elapsed: 0,
            tick_hz,
            next_seed: 1,
            next_round: 1,
            current_public_round: None,
            next_internal_request: 1,
            last_external_request: BTreeMap::new(),
        }
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn phase(&self) -> ServerPhase {
        match self.phase {
            Lifecycle::Waiting => ServerPhase::Waiting,
            Lifecycle::Loading { .. } => ServerPhase::Loading,
            Lifecycle::Playing => ServerPhase::Playing,
            Lifecycle::Results { .. } => ServerPhase::Results,
            Lifecycle::Closed => ServerPhase::Closed,
        }
    }

    pub fn room_id(&self) -> Option<RoomId> {
        self.public_room
    }

    /// Snapshot phòng; `id` là ID public ổn định qua các round, không phải ID nội bộ của lobby.
    pub fn room_view(&self) -> Option<RoomView> {
        let internal = self.internal_room?;
        let mut view = self.lobby.room(internal)?;
        if let Some(public) = self.public_room {
            view.id = public;
        }
        if let (Some(round), Some(public_round)) = (view.round.as_mut(), self.current_public_round)
        {
            round.id = public_round;
        }
        Some(view)
    }

    pub fn last_result(&self) -> Option<&InternalRoundResult> {
        self.last_result.as_ref()
    }

    pub fn tick(&self) -> Option<Tick> {
        self.active.as_ref().map(|m| m.state.tick)
    }

    pub fn rules_hash(&self) -> u64 {
        self.setup.world.rules().profile_hash
    }

    pub fn current_round_id(&self) -> Option<u32> {
        self.active
            .as_ref()
            .and_then(|m| u32::try_from(m.round.0).ok())
    }

    pub fn state_hash(&self) -> Option<u64> {
        self.active.as_ref().map(|m| m.state.hash())
    }

    pub fn player_life(&self, player: PlayerId) -> Option<Life> {
        let active = self.active.as_ref()?;
        let slot = self
            .room_view()?
            .members
            .iter()
            .find(|m| m.player == player)?
            .slot
            .0;
        active
            .state
            .players
            .iter()
            .find(|p| p.id == slot)
            .map(|p| p.life)
    }

    /// Vị trí hiện tại của người chơi trong trận (mpx) — dùng cho test/part 2.
    pub fn player_position(&self, player: PlayerId) -> Option<(i32, i32)> {
        let active = self.active.as_ref()?;
        let slot = self
            .room_view()?
            .members
            .iter()
            .find(|m| m.player == player)?
            .slot
            .0;
        active
            .state
            .players
            .iter()
            .find(|p| p.id == slot)
            .map(|p| (p.x.0, p.y.0))
    }

    /// Sequence input đã áp dụng tới của một người (ack hiện tại).
    pub fn applied_seq(&self, player: PlayerId) -> Option<u32> {
        self.active
            .as_ref()
            .and_then(|m| m.inputs.get(&player))
            .map(crate::input::InputTrack::applied_seq)
    }

    /// Snapshot DTO đầy đủ của state hiện tại; `None` khi không có trận.
    /// Chuyển đổi nằm ở `boom-sync`, server không tự dựng DTO state.
    pub fn snapshot(
        &self,
        snapshot_seq: u32,
        content_epoch: u32,
    ) -> Option<Result<Snapshot, boom_sync::SyncError>> {
        let active = self.active.as_ref()?;
        let round_id = u32::try_from(active.round.0).ok()?;
        let mut acks = Vec::new();
        for member in &active.members {
            let Some(track) = active.inputs.get(&member.player) else {
                continue;
            };
            acks.push(input_ack(
                member.slot.0,
                track.applied_seq(),
                active.state.tick,
            ));
        }
        let ctx = SnapshotContext {
            snapshot_seq,
            round_id,
            state_schema: 1,
            rules_hash: self.setup.world.rules().profile_hash,
            content_epoch,
            acks,
        };
        Some(to_snapshot(&active.state, &ctx))
    }

    /// Event mới phát sinh từ lần gọi trước; dùng cho `RoundEvents` DTO.
    pub fn take_events(&mut self) -> Vec<SimulationEvent> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        let out = active.events[active.event_cursor..].to_vec();
        active.event_cursor = active.events.len();
        out
    }

    /// Áp một lệnh phòng đã xác thực danh tính. Chỉ hợp lệ khi đang chờ.
    pub fn room_command(
        &mut self,
        command: RoomCommand,
        now_ms: u64,
    ) -> Result<RoomUpdate, ServerError> {
        self.advance_clock(now_ms)?;
        if !matches!(self.phase, Lifecycle::Waiting | Lifecycle::Closed) {
            return Err(ServerError::WrongPhase {
                phase: self.phase(),
            });
        }

        if let Some(&last) = self.last_external_request.get(&command.player) {
            if command.request == last {
                return Ok(RoomUpdate {
                    events: Vec::new(),
                    duplicate: true,
                    round: None,
                });
            }
            if command.request < last {
                return Err(ServerError::Room(RoomError::StaleRequest {
                    player: command.player,
                    last,
                    received: command.request,
                }));
            }
        }

        // Authority một phòng: không cho Create thứ hai ghi đè active room.
        if matches!(command.action, RoomAction::Create { .. })
            && let Some(room) = self.public_room
        {
            return Err(ServerError::RoomExists { room });
        }

        // Join dùng ID public; lobby dùng ID nội bộ.
        let mut action = command.action.clone();
        if let RoomAction::Join { room, character } = action {
            let internal = self.internal_room.ok_or(ServerError::NoRoom)?;
            if room != self.public_room.unwrap_or(internal) {
                return Err(ServerError::Room(RoomError::RoomNotFound { room }));
            }
            action = RoomAction::Join {
                room: internal,
                character,
            };
        }

        let internal_request = RequestId(self.next_internal_request);
        self.next_internal_request = self.next_internal_request.saturating_add(1);
        let outcome = self.lobby.apply(RoomCommand {
            player: command.player,
            request: internal_request,
            action,
        })?;
        let RoomOutcome::Applied(mut events) = outcome else {
            return Ok(RoomUpdate {
                events: Vec::new(),
                duplicate: true,
                round: None,
            });
        };
        self.last_external_request
            .insert(command.player, command.request);
        // ID round do Lobby cấp lại từ 1 sau mỗi lần dựng lại waiting; authority giữ ID public
        // tăng dần để result/replay không trùng round.
        let mut public_started = None;
        for event in &mut events {
            if let RoomEvent::RoundStarted { round, .. } = event {
                let public = RoundId(self.next_round);
                self.next_round = self.next_round.saturating_add(1);
                *round = public;
                self.current_public_round = Some(public);
                public_started = Some(public);
            }
        }

        // Ghi nhận phòng public khi tạo; Join không đổi.
        if let Some(RoomEvent::RoomCreated { room, .. }) = events.first() {
            self.public_room = Some(*room);
            self.internal_room = Some(*room);
            self.phase = Lifecycle::Waiting;
        }
        // Phòng bị xóa khi người cuối rời: cho phép tạo phòng mới.
        if self.lobby.rooms().is_empty() {
            self.public_room = None;
            self.internal_room = None;
            self.phase = Lifecycle::Waiting;
        }

        if let Some(RoomEvent::RoundStarted { room, members, .. }) = events
            .iter()
            .find(|e| matches!(e, RoomEvent::RoundStarted { .. }))
        {
            let (room, members) = (*room, members.clone());
            let round = public_started.expect("round started");
            if let Err(err) = self.begin_loading(room, round, members.clone(), now_ms) {
                // Lobby đã chuyển Started; đưa về waiting để không kẹt phòng.
                self.phase = Lifecycle::Waiting;
                self.current_public_round = None;
                self.rebuild_waiting(&members);
                return Err(err);
            }
            return Ok(RoomUpdate {
                events,
                duplicate: false,
                round: Some(round),
            });
        }

        Ok(RoomUpdate {
            events,
            duplicate: false,
            round: None,
        })
    }

    fn begin_loading(
        &mut self,
        room: RoomId,
        round: RoundId,
        members: Vec<RoundMember>,
        now_ms: u64,
    ) -> Result<(), ServerError> {
        let mut roster = Vec::with_capacity(members.len());
        let mut slot_to_player = BTreeMap::new();
        for member in &members {
            let stats = self.stats_for(member.character);
            roster.push(PlayerSetup {
                id: member.slot.0,
                stats,
            });
            slot_to_player.insert(member.slot.0, member.player);
        }
        let seed = self.next_seed;
        self.next_seed = self.next_seed.saturating_add(1);
        let state = SimulationState::new_round(&self.setup.world, &roster, seed)?;
        let inputs = members
            .iter()
            .map(|m| (m.player, InputTrack::new()))
            .collect();
        self.last_result = None;
        self.active = Some(Match {
            room,
            round,
            seed,
            members: members.clone(),
            slot_to_player,
            state,
            inputs,
            disconnected_at: BTreeMap::new(),
            eliminated: BTreeSet::new(),
            events: Vec::new(),
            event_cursor: 0,
            result: None,
        });
        self.phase = Lifecycle::Loading {
            started_ms: now_ms,
            acked: BTreeSet::new(),
        };
        self.replay = None;
        if let (Some(dir), Some(replay_setup)) = (&self.config.replay_dir, &self.setup.replay) {
            match crate::replay_log::ReplayLog::new(
                dir,
                round.0,
                seed,
                replay_setup,
                &self.setup,
                &members,
                self.setup.world.terrain(),
            ) {
                Ok(log) => self.replay = Some(log),
                Err(err) => {
                    eprintln!("replay_log: không mở được {}: {err}", dir.display());
                }
            }
        }
        Ok(())
    }

    fn stats_for(&self, character: CharacterId) -> CharacterStats {
        match character {
            CharacterId::C00 => self.setup.stats.c00,
            CharacterId::C01 => self.setup.stats.c01,
        }
    }

    /// Client báo đã load xong. Đủ mọi người thì vào Playing.
    pub fn ack_loading(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.advance_clock(now_ms)?;
        let Lifecycle::Loading { started_ms, acked } = &mut self.phase else {
            return Err(ServerError::WrongPhase {
                phase: self.phase(),
            });
        };
        let active = self.active.as_ref().ok_or(ServerError::NoRoom)?;
        if !active.members.iter().any(|m| m.player == player) {
            return Err(ServerError::UnknownPlayer(player));
        }
        let (room, round, members) = (active.room, active.round, active.members.clone());
        if now_ms.saturating_sub(*started_ms) >= u64::from(self.config.policy.loading_timeout_ms) {
            // Frame tới trước timer không được vượt loading barrier (SCOPE_V1 §3).
            self.cancel_round(&members);
            return Ok(vec![InternalServerEvent::RoundCancelled {
                room,
                round: Some(round),
            }]);
        }
        acked.insert(player);
        let mut events = vec![InternalServerEvent::LoadingAcked {
            room: active.room,
            round: active.round,
            player,
        }];
        if acked.len() == active.members.len() {
            let (room, round) = (active.room, active.round);
            self.phase = Lifecycle::Playing;
            self.start_play_clock(now_ms);
            events.push(InternalServerEvent::MatchStarted { room, round });
        }
        Ok(events)
    }

    /// Nhận `InputBatch` DTO từ một phiên đã bind. Server bind player theo session, không tin
    /// trường `player` trên dây; scheduler giữ nguyên ngữ nghĩa ADR 0009.
    pub fn submit_input(
        &mut self,
        player: PlayerId,
        batch: &InputBatch,
        now_ms: u64,
    ) -> Result<InputAck, InputError> {
        if !matches!(self.phase, Lifecycle::Playing) {
            return Err(InputError::NotPlaying);
        }
        batch
            .check(&self.config.protocol_limits)
            .map_err(InputError::Proto)?;
        let active = self.active.as_mut().expect("playing có match");
        let round = u32::try_from(active.round.0).map_err(|_| InputError::RoundIdOutOfRange {
            round: active.round,
        })?;
        if batch.round_id != round {
            return Err(InputError::WrongRound {
                expected: active.round,
                received: RoundId(u64::from(batch.round_id)),
            });
        }
        let Some((&slot, _)) = active
            .slot_to_player
            .iter()
            .find(|(_, bound)| **bound == player)
        else {
            return Err(InputError::UnknownPlayer(player));
        };
        if batch.player != slot {
            return Err(InputError::PlayerMismatch {
                expected: slot,
                received: batch.player,
            });
        }
        if active.disconnected_at.contains_key(&player) || active.eliminated.contains(&player) {
            return Err(InputError::UnknownPlayer(player));
        }
        let track = active
            .inputs
            .get_mut(&player)
            .ok_or(InputError::UnknownPlayer(player))?;
        let ack = track.submit(
            &batch.commands,
            active.state.tick,
            now_ms.max(self.now_ms),
            self.input_limits,
        )?;
        Ok(ack)
    }

    /// Cho thời gian trôi; chạy các tick đã đến hạn, kiểm loading/disconnect/result timeout.
    pub fn advance_to(&mut self, now_ms: u64) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.advance_clock(now_ms)?;
        let mut events = Vec::new();
        loop {
            match &self.phase {
                Lifecycle::Loading { started_ms, acked } => {
                    let deadline = u64::from(self.config.policy.loading_timeout_ms);
                    let active = self.active.as_ref().ok_or(ServerError::NoRoom)?;
                    if acked.len() == active.members.len() {
                        let (room, round) = (active.room, active.round);
                        self.phase = Lifecycle::Playing;
                        self.start_play_clock(now_ms);
                        events.push(InternalServerEvent::MatchStarted { room, round });
                        continue;
                    }
                    if now_ms.saturating_sub(*started_ms) >= deadline {
                        let (room, round) = (active.room, active.round);
                        let members = active.members.clone();
                        self.cancel_round(&members);
                        events.push(InternalServerEvent::RoundCancelled {
                            room,
                            round: Some(round),
                        });
                        continue;
                    }
                    break;
                }
                Lifecycle::Playing => {
                    if now_ms < self.next_tick_ms {
                        break;
                    }
                    let mut finished = false;
                    self.eliminate_disconnected(now_ms, &mut events);
                    if self.active.as_ref().is_some_and(|m| m.result.is_some()) {
                        finished = true;
                    } else {
                        self.run_tick(&mut events)?;
                        if self.active.as_ref().is_some_and(|m| m.result.is_some()) {
                            finished = true;
                        }
                    }
                    if finished {
                        let active = self.active.as_ref().expect("match");
                        let result = active.result.clone().expect("result");
                        let room = active.room;
                        self.phase = Lifecycle::Results {
                            at_ms: now_ms,
                            acked: BTreeSet::new(),
                        };
                        self.last_result = Some(result.clone());
                        self.finish_replay();
                        events.push(InternalServerEvent::RoundFinished { room, result });
                    }
                    self.ticks_elapsed = self.ticks_elapsed.saturating_add(1);
                    self.next_tick_ms = self
                        .tick_origin_ms
                        .saturating_add(self.ticks_elapsed.saturating_mul(1_000) / self.tick_hz);
                }
                Lifecycle::Results { at_ms, .. } => {
                    let deadline = u64::from(self.config.policy.results_timeout_ms);
                    if now_ms.saturating_sub(*at_ms) >= deadline {
                        self.return_to_waiting(&mut events);
                        continue;
                    }
                    break;
                }
                Lifecycle::Waiting | Lifecycle::Closed => break,
            }
        }
        Ok(events)
    }

    fn start_play_clock(&mut self, now_ms: u64) {
        self.tick_origin_ms = now_ms;
        self.ticks_elapsed = 0;
        self.next_tick_ms = now_ms;
    }

    fn run_tick(&mut self, out: &mut Vec<InternalServerEvent>) -> Result<(), ServerError> {
        let active = self.active.as_mut().ok_or(ServerError::NoRoom)?;
        let tick = active.state.tick;
        let player_ids: Vec<PlayerId> = active
            .state
            .players
            .iter()
            .map(|player| active.slot_to_player[&player.id])
            .collect();
        let mut inputs = Vec::with_capacity(player_ids.len());
        for player_id in player_ids {
            if active.disconnected_at.contains_key(&player_id)
                || active.eliminated.contains(&player_id)
            {
                inputs.push(PlayerInput::default());
                continue;
            }
            let (input, retired) = active
                .inputs
                .get_mut(&player_id)
                .expect("input track")
                .apply_tick(tick, self.input_limits)?;
            for seq in retired {
                out.push(InternalServerEvent::InputRetired {
                    room: active.room,
                    round: active.round,
                    player: player_id,
                    seq,
                });
            }
            inputs.push(input);
        }
        let slots: Vec<u8> = active.state.players.iter().map(|p| p.id).collect();
        if let Some(replay) = &mut self.replay {
            replay.record_inputs(tick, &inputs, &slots);
        }
        let mut events = Vec::new();
        active.state.step(&self.setup.world, &inputs, &mut events)?;
        if let Some(replay) = &mut self.replay {
            replay.record_checkpoint(active.state.tick, active.state.hash());
        }
        active.events.extend(events);
        if active.state.phase != CorePhase::Playing {
            let outcome = match active.state.phase {
                CorePhase::Ended(CoreOutcome::Winner(slot)) => active
                    .player_for_slot(slot)
                    .map_or(MatchOutcome::Aborted, MatchOutcome::Winner),
                CorePhase::Ended(CoreOutcome::Draw) => {
                    if active.eliminated.len() >= active.members.len() {
                        MatchOutcome::Aborted
                    } else {
                        MatchOutcome::Draw
                    }
                }
                CorePhase::Playing => unreachable!(),
            };
            active.result = Some(InternalRoundResult {
                round: active.round,
                outcome,
                tick: active.state.tick,
                seed: active.seed,
                events: active.events.clone(),
            });
        }
        Ok(())
    }

    fn finish_replay(&mut self) {
        let Some(replay) = self.replay.take() else {
            return;
        };
        let Some(active) = self.active.as_ref() else {
            return;
        };
        let Some(result) = active.result.as_ref() else {
            return;
        };
        let outcome = crate::replay_log::outcome_label(result.outcome, &active.members);
        if let Err(err) = replay.finish(
            active.state.tick,
            active.state.hash(),
            boom_core::event_digest(&active.events),
            active.events.len(),
            &outcome,
        ) {
            eprintln!("replay_log: không ghi được replay: {err}");
        }
    }

    fn eliminate_disconnected(&mut self, now_ms: u64, out: &mut Vec<InternalServerEvent>) {
        let timeout = u64::from(self.config.policy.disconnect_timeout_ms);
        let Some(active) = self.active.as_mut() else {
            return;
        };
        let due: Vec<PlayerId> = active
            .disconnected_at
            .iter()
            .filter(|(p, at)| !active.eliminated.contains(p) && now_ms >= **at + timeout)
            .map(|(p, _)| *p)
            .collect();
        for player in due {
            active.eliminated.insert(player);
            if let Some(slot) = active
                .members
                .iter()
                .find(|m| m.player == player)
                .map(|m| m.slot.0)
                && let Some(p) = active.state.players.iter_mut().find(|p| p.id == slot)
                && p.life != Life::Dead
            {
                p.life = Life::Dead;
            }
            out.push(InternalServerEvent::PlayerEliminated {
                room: active.room,
                player,
                at_ms: now_ms,
            });
        }
        if active.eliminated.len() >= active.members.len() {
            active.result = Some(InternalRoundResult {
                round: active.round,
                outcome: MatchOutcome::Aborted,
                tick: active.state.tick,
                seed: active.seed,
                events: active.events.clone(),
            });
        }
    }

    /// Client ack kết quả; đủ mọi người thì về phòng.
    pub fn ack_result(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.advance_clock(now_ms)?;
        let Lifecycle::Results { acked, .. } = &mut self.phase else {
            return Err(ServerError::WrongPhase {
                phase: self.phase(),
            });
        };
        let active = self.active.as_ref().ok_or(ServerError::NoRoom)?;
        if !active.members.iter().any(|m| m.player == player) {
            return Err(ServerError::UnknownPlayer(player));
        }
        acked.insert(player);
        if acked.len() == active.members.len() {
            let mut events = Vec::new();
            self.return_to_waiting(&mut events);
            return Ok(events);
        }
        Ok(Vec::new())
    }

    /// Mất kết nối: trong trận input thành neutral ngay, quá timeout loại một lần;
    /// trong loading hủy start và reset ready (SCOPE_V1 §3).
    pub fn disconnect(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.advance_clock(now_ms)?;
        let mut events = Vec::new();
        match &self.phase {
            Lifecycle::Loading { .. } => {
                let active = self.active.as_ref().ok_or(ServerError::NoRoom)?;
                if !active.members.iter().any(|m| m.player == player) {
                    return Err(ServerError::UnknownPlayer(player));
                }
                let (room, round) = (active.room, active.round);
                let members: Vec<RoundMember> = active
                    .members
                    .iter()
                    .filter(|m| m.player != player)
                    .cloned()
                    .collect();
                self.cancel_round(&members);
                events.push(InternalServerEvent::RoundCancelled {
                    room,
                    round: Some(round),
                });
            }
            Lifecycle::Playing => {
                let active = self.active.as_mut().ok_or(ServerError::NoRoom)?;
                if !active.members.iter().any(|m| m.player == player) {
                    return Err(ServerError::UnknownPlayer(player));
                }
                active.disconnected_at.entry(player).or_insert(now_ms);
                events.push(InternalServerEvent::PlayerDisconnected {
                    room: active.room,
                    player,
                    at_ms: now_ms,
                });
            }
            Lifecycle::Results { .. } => {
                let active = self.active.as_mut().ok_or(ServerError::NoRoom)?;
                if !active.members.iter().any(|m| m.player == player) {
                    return Err(ServerError::UnknownPlayer(player));
                }
                active.disconnected_at.entry(player).or_insert(now_ms);
                events.push(InternalServerEvent::PlayerDisconnected {
                    room: active.room,
                    player,
                    at_ms: now_ms,
                });
            }
            Lifecycle::Waiting => {
                // Ở waiting, disconnect tương đương rời phòng để không giữ slot chết.
                self.internal_room.ok_or(ServerError::NoRoom)?;
                let _ = self.lobby.apply(RoomCommand {
                    player,
                    request: RequestId(self.next_internal_request),
                    action: RoomAction::Leave,
                });
                self.next_internal_request = self.next_internal_request.saturating_add(1);
                if self.lobby.rooms().is_empty() {
                    self.internal_room = None;
                    self.public_room = None;
                }
            }
            Lifecycle::Closed => return Err(ServerError::NoRoom),
        }
        Ok(events)
    }

    fn cancel_round(&mut self, members: &[RoundMember]) {
        self.active = None;
        self.phase = Lifecycle::Waiting;
        self.current_public_round = None;
        self.rebuild_waiting(members);
    }

    fn return_to_waiting(&mut self, out: &mut Vec<InternalServerEvent>) {
        let Some(active) = self.active.take() else {
            return;
        };
        let (room, round) = (active.room, active.round);
        let remaining: Vec<RoundMember> = active
            .members
            .iter()
            .filter(|m| {
                !active.disconnected_at.contains_key(&m.player)
                    && !active.eliminated.contains(&m.player)
            })
            .cloned()
            .collect();
        self.phase = Lifecycle::Waiting;
        self.current_public_round = None;
        self.rebuild_waiting(&remaining);
        out.push(InternalServerEvent::ReturnedToWaiting { room, round });
    }

    /// Dựng lại lobby waiting từ roster đã chốt, giữ ID public; ready reset hết.
    fn rebuild_waiting(&mut self, members: &[RoundMember]) {
        if members.is_empty() {
            self.phase = Lifecycle::Closed;
            self.lobby = Lobby::new(self.config.policy.room_policy);
            self.internal_room = None;
            self.public_room = None;
            return;
        }
        let host = self
            .internal_room
            .and_then(|room| self.lobby.room(room))
            .map_or(members[0].player, |view| view.host);
        let mut ordered: Vec<RoundMember> = members.to_vec();
        ordered.sort_by_key(|m| if m.player == host { 0 } else { 1 + m.slot.0 });
        self.lobby = Lobby::new(self.config.policy.room_policy);
        self.internal_room = None;
        let mut created = None;
        for (index, member) in ordered.iter().enumerate() {
            let action = if index == 0 {
                RoomAction::Create {
                    character: member.character,
                }
            } else {
                RoomAction::Join {
                    room: created.expect("đã create"),
                    character: member.character,
                }
            };
            let request = RequestId(self.next_internal_request);
            self.next_internal_request = self.next_internal_request.saturating_add(1);
            if let Ok(RoomOutcome::Applied(events)) = self.lobby.apply(RoomCommand {
                player: member.player,
                request,
                action,
            }) && let Some(RoomEvent::RoomCreated { room, .. }) = events.first()
            {
                created = Some(*room);
                self.internal_room = Some(*room);
            }
        }
        if self.public_room.is_none() {
            self.public_room = created;
        }
    }

    fn advance_clock(&mut self, now_ms: u64) -> Result<(), ServerError> {
        if now_ms < self.now_ms {
            return Err(ServerError::TimeWentBackwards {
                last_ms: self.now_ms,
                received_ms: now_ms,
            });
        }
        self.now_ms = now_ms;
        Ok(())
    }
}

impl fmt::Debug for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Authority")
            .field("phase", &self.phase())
            .field("room", &self.public_room)
            .field("tick", &self.tick())
            .finish()
    }
}
