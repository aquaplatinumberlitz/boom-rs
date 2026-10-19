//! Vòng IO mỏng trên `boom-net` (ADR 0006) + handshake V1-002 (ADR 0009 §1).
//!
//! Mỗi kết nối có **một luồng đọc** gọi `recv_frame` chặn và đẩy frame qua channel vào vòng tick
//! của server. `send_frame` được gọi từ vòng tick trên nửa ghi của cùng kết nối; vì vậy test dùng
//! hai đầu `MemoryTransport` cho hai chiều. Không gọi `recv_frame` trong vòng tick.
//!
//! Room/loading/result đi qua wire Addendum B: `LobbyListRequest`, `RoomCommand`, `RoomChat`,
//! `LoadingAck`, `ResultAck`, `RoundStart`, `RoundResult`. Server bind `player` từ session đã auth;
//! auth thật (T5) chưa có, `mark_authenticated_for_test` tạm đánh dấu. Wrapper API authority vẫn
//! giữ cho test nội bộ và caller chưa chuyển wire.

use std::collections::BTreeMap;
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::thread;
use std::time::Duration;

use boom_core::Life;
use boom_net::{NetError, TcpTransport, Transport};
use boom_proto::{
    Hello, InputBatch, InputRejectReason, InputRejected, LoadingAck, LobbyListRequest, Message,
    Ping, Pong, Reject, RejectReason, ResultAck, RoomChat, RoomChatMessage,
    RoomCommand as RoomCommandDto, RoomOutcome, RoomRejected, RoundResult, RoundStart,
    SnapshotRequest, Welcome, decode_frame, encode_frame, limits::WIRE_SCHEMA, read_envelope,
};
use boom_room::{PlayerId, RoomCommand, RoomView, RoundId};
use boom_sync::{
    RoomGate, from_room_command, reject_reason, to_lobby_list, to_room_state, to_round_events,
};

use crate::authority::{
    Authority, InternalRoundResult, InternalServerEvent, MatchOutcome, RoomUpdate, ServerError,
    ServerPhase,
};
use crate::config::HandshakeConfig;

enum Inbound {
    Frame { player: PlayerId, frame: Vec<u8> },
    Closed { player: PlayerId },
    WriteFailed { player: PlayerId },
}

enum Outbound {
    Frame(Vec<u8>),
}

/// Hàng đợi ghi có giới hạn theo kết nối; client không đọc thì bị ngắt thay vì chặn tick.
const OUTBOUND_QUEUE_FRAMES: usize = 32;

/// Hạn chờ writer gửi nốt frame đã xếp (vd. `Reject`) trước khi đóng cưỡng bức socket. Giá trị
/// vận hành (design), không phải luật chơi; không chặn tick vì chờ ở luồng riêng.
const GRACEFUL_CLOSE_MS: u64 = 2_000;

struct Session {
    slot: u8,
    session_id: String,
    handshaken: bool,
    out_tx: SyncSender<Outbound>,
    shutdown: Option<Box<dyn FnOnce() + Send>>,
    /// Writer báo đã gửi hết queue và đóng nửa ghi (dùng khi đóng có flush).
    writer_done: Receiver<()>,
    next_snapshot_ms: u64,
    snapshot_seq: u32,
    /// Cổng kind phòng: false trước `AuthOk` (T5 thay bằng Auth thật).
    auth: RoomGate,
    /// Cửa sổ rate-limit chat 1 giây; server kiểm, DTO chỉ kiểm độ dài.
    chat_window_start_ms: u64,
    chat_count: u32,
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            shutdown();
        }
    }
}

/// Server một phòng: authority + các session transport. Test dùng `MemoryTransport`; production
/// có thể split một `TcpTransport` thành hai nửa đọc/ghi và gọi [`Server::accept`].
pub struct Server {
    authority: Authority,
    handshake: HandshakeConfig,
    tx: Sender<Inbound>,
    rx: Receiver<Inbound>,
    sessions: BTreeMap<PlayerId, Session>,
    now_ms: u64,
    next_session: u64,
}

impl Server {
    pub fn new(authority: Authority, handshake: HandshakeConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            authority,
            handshake,
            tx,
            rx,
            sessions: BTreeMap::new(),
            now_ms: 0,
            next_session: 1,
        }
    }

    pub fn authority(&self) -> &Authority {
        &self.authority
    }

    pub fn phase(&self) -> ServerPhase {
        self.authority.phase()
    }

    pub fn room_view(&self) -> Option<RoomView> {
        self.authority.room_view()
    }

    pub fn last_result(&self) -> Option<&InternalRoundResult> {
        self.authority.last_result()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Tạm đánh dấu session đã `AuthOk` cho test/transition; task auth T5 sẽ thay bằng
    /// `Auth`/`AuthOk` thật (ADR 0009 Addendum A).
    pub fn mark_authenticated_for_test(&mut self, player: PlayerId) -> Result<(), ServerError> {
        let Some(session) = self.sessions.get_mut(&player) else {
            return Err(ServerError::NoSession(player));
        };
        if !session.handshaken {
            return Err(ServerError::NoSession(player));
        }
        session.auth.on_auth_ok();
        Ok(())
    }

    /// Nhận một nửa đọc và một nửa ghi của kết nối; spawn đúng một luồng đọc chặn.
    pub fn accept<R, W>(
        &mut self,
        player: PlayerId,
        slot: u8,
        reader: R,
        writer: W,
    ) -> Result<(), ServerError>
    where
        R: Transport + Send + 'static,
        W: Transport + Send + 'static,
    {
        self.accept_with_shutdown(player, slot, reader, writer, || {})
    }

    /// Như [`Server::accept`] nhưng có handle shutdown độc lập để hủy socket khi session bị loại,
    /// kể cả writer đang kẹt trong `send_frame` blocking.
    pub fn accept_with_shutdown<R, W, S>(
        &mut self,
        player: PlayerId,
        slot: u8,
        mut reader: R,
        writer: W,
        shutdown: S,
    ) -> Result<(), ServerError>
    where
        R: Transport + Send + 'static,
        W: Transport + Send + 'static,
        S: FnOnce() + Send + 'static,
    {
        if self.sessions.contains_key(&player) {
            return Err(ServerError::SessionExists(player));
        }
        let session_id = format!("s{}", self.next_session);
        self.next_session = self.next_session.saturating_add(1);
        let (out_tx, out_rx) = mpsc::sync_channel(OUTBOUND_QUEUE_FRAMES);
        let (done_tx, writer_done) = mpsc::channel();
        self.sessions.insert(
            player,
            Session {
                slot,
                session_id,
                handshaken: false,
                out_tx,
                shutdown: Some(Box::new(shutdown)),
                writer_done,
                next_snapshot_ms: 0,
                snapshot_seq: 0,
                auth: RoomGate::new(),
                chat_window_start_ms: 0,
                chat_count: 0,
            },
        );
        let outbound_tx = self.tx.clone();
        thread::spawn(move || {
            let mut writer = writer;
            while let Ok(out) = out_rx.recv() {
                match out {
                    Outbound::Frame(frame) => {
                        if writer.send_frame(&frame).is_err() {
                            let _ = outbound_tx.send(Inbound::WriteFailed { player });
                            break;
                        }
                    }
                }
            }
            let _ = writer.close();
            let _ = done_tx.send(());
        });
        let tx = self.tx.clone();
        thread::spawn(move || {
            loop {
                match reader.recv_frame() {
                    Ok(Some(frame)) => {
                        if tx.send(Inbound::Frame { player, frame }).is_err() {
                            break;
                        }
                    }
                    Ok(None) | Err(_) => {
                        let _ = tx.send(Inbound::Closed { player });
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    /// Nhận TCP, tách hai nửa đọc/ghi của cùng socket (một luồng đọc chặn).
    pub fn accept_tcp(
        &mut self,
        listener: &TcpListener,
        player: PlayerId,
        slot: u8,
    ) -> Result<(), ServerError> {
        let (stream, _) = listener.accept().map_err(NetError::from)?;
        let writer_stream = stream.try_clone().map_err(NetError::from)?;
        let shutdown_stream = writer_stream.try_clone().map_err(NetError::from)?;
        let reader = TcpTransport::from_stream(stream)?;
        let writer = TcpTransport::from_stream(writer_stream)?;
        let mut shutdown = TcpTransport::from_stream(shutdown_stream)?;
        self.accept_with_shutdown(player, slot, reader, writer, move || {
            let _ = shutdown.close();
        })
    }

    /// Chạy một vòng: nhận frame (chờ tối đa `wait`), xử lý authority, gửi snapshot/event.
    pub fn poll(
        &mut self,
        now_ms: u64,
        wait: Duration,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        if now_ms < self.now_ms {
            return Err(ServerError::TimeWentBackwards {
                last_ms: self.now_ms,
                received_ms: now_ms,
            });
        }
        self.now_ms = now_ms;
        let mut first = None;
        if !wait.is_zero() {
            match self.rx.recv_timeout(wait) {
                Ok(inbound) => first = Some(inbound),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {}
            }
        }
        let mut inbound = Vec::new();
        if let Some(item) = first {
            inbound.push(item);
        }
        while let Ok(item) = self.rx.try_recv() {
            inbound.push(item);
        }
        for item in inbound {
            let player = match &item {
                Inbound::Frame { player, .. }
                | Inbound::Closed { player }
                | Inbound::WriteFailed { player } => *player,
            };
            if let Err(err) = self.handle_inbound(item) {
                // Lỗi protocol/transport của một kết nối không được giết poll/tick.
                if matches!(
                    err,
                    ServerError::Proto(_) | ServerError::Sync(_) | ServerError::Net(_)
                ) {
                    self.close_session(player);
                } else {
                    return Err(err);
                }
            }
        }
        let events = self.authority.advance_to(now_ms)?;
        self.flush_events()?;
        self.flush_lifecycle(&events)?;
        self.flush_snapshots(now_ms)?;
        Ok(events)
    }

    fn handle_inbound(&mut self, inbound: Inbound) -> Result<(), ServerError> {
        match inbound {
            Inbound::Closed { player } | Inbound::WriteFailed { player } => {
                self.close_session(player);
            }
            Inbound::Frame { player, frame } => {
                self.handle_frame(player, &frame)?;
            }
        }
        Ok(())
    }

    fn handle_frame(&mut self, player: PlayerId, frame: &[u8]) -> Result<(), ServerError> {
        let Some(session) = self.sessions.get(&player) else {
            return Ok(());
        };
        if !session.handshaken {
            return self.handle_hello(player, frame);
        }
        let (envelope, _) = read_envelope(frame)?;
        let kind = envelope.kind;
        if kind.is_room()
            && !self
                .sessions
                .get(&player)
                .is_some_and(|session| session.auth.is_authenticated())
        {
            self.send(
                player,
                &Reject {
                    reason: RejectReason::SessionRejected,
                    detail: format!("{kind:?} chỉ hợp lệ sau AuthOk"),
                },
            )?;
            self.close_session_after_flush(player);
            return Ok(());
        }
        let limits = self.authority.config().protocol_limits;
        match kind {
            boom_proto::MessageKind::InputBatch => {
                let batch: InputBatch = decode_frame(frame, &limits)?;
                match self.authority.submit_input(player, &batch, self.now_ms) {
                    Ok(_) => {}
                    Err(err) => self.send_input_rejected(player, &batch, &err)?,
                }
            }
            boom_proto::MessageKind::SnapshotRequest => {
                let _: SnapshotRequest = decode_frame(frame, &limits)?;
                self.send_snapshot(player)?;
            }
            boom_proto::MessageKind::Ping => {
                let ping: Ping = decode_frame(frame, &limits)?;
                self.send(player, &Pong(ping.0))?;
            }
            boom_proto::MessageKind::Pong => {}
            boom_proto::MessageKind::LobbyListRequest => {
                let _: LobbyListRequest = decode_frame(frame, &limits)?;
                self.send_lobby_list(player)?;
            }
            boom_proto::MessageKind::RoomCommand => {
                let dto: RoomCommandDto = decode_frame(frame, &limits)?;
                let command = from_room_command(&dto, player);
                match self.room_command(command, self.now_ms) {
                    Ok(update) => self.after_room_update(&update)?,
                    Err(ServerError::Room(err)) => {
                        self.send(
                            player,
                            &boom_proto::RoomRejected {
                                request: dto.request,
                                reason: reject_reason(&err),
                            },
                        )?;
                    }
                    Err(err) => return Err(err),
                }
            }
            boom_proto::MessageKind::RoomChat => {
                let chat: RoomChat = decode_frame(frame, &limits)?;
                self.handle_room_chat(player, &chat)?;
            }
            boom_proto::MessageKind::LoadingAck => {
                let ack: LoadingAck = decode_frame(frame, &limits)?;
                let _ = ack.round_id;
                match self.authority.ack_loading(player, self.now_ms) {
                    Ok(events) => self.flush_lifecycle(&events)?,
                    Err(_) => self.send_room_rejected(
                        player,
                        0,
                        boom_proto::RoomRejectReason::NotWaiting,
                    )?,
                }
            }
            boom_proto::MessageKind::ResultAck => {
                let ack: ResultAck = decode_frame(frame, &limits)?;
                let _ = ack.round_id;
                match self.authority.ack_result(player, self.now_ms) {
                    Ok(events) => self.flush_lifecycle(&events)?,
                    Err(_) => self.send_room_rejected(
                        player,
                        0,
                        boom_proto::RoomRejectReason::NotWaiting,
                    )?,
                }
            }
            _ => {
                self.close_session(player);
            }
        }
        Ok(())
    }

    fn send_room_rejected(
        &mut self,
        player: PlayerId,
        request: u64,
        reason: boom_proto::RoomRejectReason,
    ) -> Result<(), ServerError> {
        self.send(player, &RoomRejected { request, reason })
    }

    fn after_room_update(&mut self, update: &RoomUpdate) -> Result<(), ServerError> {
        if let Some(view) = self.authority.room_view() {
            self.send_room_state(&view)?;
        }
        self.broadcast_lobby_list()?;
        if let Some(round) = update.round {
            self.send_round_start(round)?;
        }
        Ok(())
    }

    fn send_room_state(&mut self, view: &RoomView) -> Result<(), ServerError> {
        let state = to_room_state(view);
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for player in members {
            if self
                .sessions
                .get(&player)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(player, &state)?;
            }
        }
        Ok(())
    }

    fn send_lobby_list(&mut self, player: PlayerId) -> Result<(), ServerError> {
        let views: Vec<RoomView> = self.authority.room_view().into_iter().collect();
        let list = to_lobby_list(&views);
        self.send(player, &list)
    }

    fn broadcast_lobby_list(&mut self) -> Result<(), ServerError> {
        let views: Vec<RoomView> = self.authority.room_view().into_iter().collect();
        let list = to_lobby_list(&views);
        let players: Vec<PlayerId> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.handshaken && s.auth.is_authenticated())
            .map(|(p, _)| *p)
            .collect();
        for player in players {
            self.send(player, &list)?;
        }
        Ok(())
    }

    fn send_round_start(&mut self, round: RoundId) -> Result<(), ServerError> {
        let Some(view) = self.authority.room_view() else {
            return Ok(());
        };
        if view.round.is_none() {
            return Ok(());
        }
        let Some(roster) = to_room_state(&view).round else {
            return Ok(());
        };
        let Some(seed) = self.authority.round_seed() else {
            return Ok(());
        };
        let message = RoundStart {
            round_id: round.0,
            seed,
            rules_profile_id: self.handshake.rules_profile_id.clone(),
            rules_hash: self.handshake.rules_hash,
            content_sha256: self.handshake.content_sha256,
            map: self.authority.map_id().to_owned(),
            mode: self.authority.mode_id().to_owned(),
            roster: roster.members,
            start_tick: self.authority.tick().unwrap_or(0),
        };
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for player in members {
            if self
                .sessions
                .get(&player)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(player, &message)?;
            }
        }
        Ok(())
    }

    fn handle_room_chat(&mut self, player: PlayerId, chat: &RoomChat) -> Result<(), ServerError> {
        let Some(view) = self.authority.room_view() else {
            return self.send_room_rejected(
                player,
                chat.request,
                boom_proto::RoomRejectReason::PlayerNotInRoom,
            );
        };
        if !view.members.iter().any(|m| m.player == player) {
            return self.send_room_rejected(
                player,
                chat.request,
                boom_proto::RoomRejectReason::PlayerNotInRoom,
            );
        }
        let max_per_s = self.authority.config().protocol_limits.max_chat_per_s;
        let now = self.now_ms;
        let Some(session) = self.sessions.get_mut(&player) else {
            return Ok(());
        };
        if now.saturating_sub(session.chat_window_start_ms) >= 1_000 {
            session.chat_window_start_ms = now;
            session.chat_count = 0;
        }
        if session.chat_count >= max_per_s {
            return Ok(());
        }
        session.chat_count = session.chat_count.saturating_add(1);
        let message = RoomChatMessage {
            room_id: view.id.0,
            from: player.0,
            from_name: format!("player{}", player.0),
            text: chat.text.clone(),
        };
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for member in members {
            if self
                .sessions
                .get(&member)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(member, &message)?;
            }
        }
        Ok(())
    }

    fn send_round_result(&mut self, result: &InternalRoundResult) -> Result<(), ServerError> {
        let Some(view) = self.authority.room_view() else {
            return Ok(());
        };
        let roster = to_room_state(&view)
            .round
            .map(|r| r.members)
            .unwrap_or_default();
        let winner = match result.outcome {
            MatchOutcome::Winner(player) => Some(player),
            MatchOutcome::Draw | MatchOutcome::Aborted => None,
        };
        let draw = matches!(result.outcome, MatchOutcome::Draw | MatchOutcome::Aborted);
        let outcome = winner.map_or(RoomOutcome::Draw, |p| RoomOutcome::Winner(p.0));
        let results = roster
            .into_iter()
            .map(|member| boom_proto::PlayerResult {
                player: member.player,
                slot: member.slot,
                rank: if draw || winner == Some(PlayerId(member.player)) {
                    1
                } else {
                    2
                },
                alive: draw || winner == Some(PlayerId(member.player)),
            })
            .collect();
        let message = RoundResult {
            round_id: result.round.0,
            outcome,
            results,
        };
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for player in members {
            if self
                .sessions
                .get(&player)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(player, &message)?;
            }
        }
        Ok(())
    }

    fn flush_lifecycle(&mut self, events: &[InternalServerEvent]) -> Result<(), ServerError> {
        for event in events {
            match event {
                InternalServerEvent::RoundFinished { result, .. } => {
                    self.send_round_result(result)?;
                }
                InternalServerEvent::ReturnedToWaiting { .. }
                | InternalServerEvent::RoundCancelled { .. } => {
                    if let Some(view) = self.authority.room_view() {
                        self.send_room_state(&view)?;
                    }
                    self.broadcast_lobby_list()?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_hello(&mut self, player: PlayerId, frame: &[u8]) -> Result<(), ServerError> {
        let limits = self.authority.config().protocol_limits;
        let hello: Hello = match decode_frame(frame, &limits) {
            Ok(hello) => hello,
            Err(_) => {
                self.close_session(player);
                return Ok(());
            }
        };
        if let Some(reason) = self.check_hello(&hello) {
            self.send(
                player,
                &Reject {
                    reason,
                    detail: format!("handshake bị từ chối: {reason:?}"),
                },
            )?;
            self.close_session_after_flush(player);
            return Ok(());
        }
        let Some(session) = self.sessions.get(&player) else {
            return Ok(());
        };
        let welcome = Welcome {
            session_id: session.session_id.clone(),
            player_id: session.slot,
            wire_schema: WIRE_SCHEMA,
            state_schema: 1,
            rules_profile_id: self.handshake.rules_profile_id.clone(),
            rules_hash: self.handshake.rules_hash,
            content_epoch: self.handshake.content_epoch,
            snapshot_interval_ms: self.handshake.snapshot_interval_ms.max(1),
            protocol_limits_version: limits.revision,
        };
        self.send(player, &welcome)?;
        if let Some(session) = self.sessions.get_mut(&player) {
            session.handshaken = true;
            session.next_snapshot_ms = self.now_ms;
        }
        Ok(())
    }

    fn check_hello(&self, hello: &Hello) -> Option<RejectReason> {
        if hello.wire_schema != WIRE_SCHEMA {
            return Some(RejectReason::UnsupportedWireSchema);
        }
        if hello.state_schema != WIRE_SCHEMA {
            return Some(RejectReason::UnsupportedStateSchema);
        }
        if hello.rules_profile_id != self.handshake.rules_profile_id
            || hello.rules_hash != self.handshake.rules_hash
        {
            return Some(RejectReason::RulesMismatch);
        }
        if hello.content_sha256 != self.handshake.content_sha256 {
            return Some(RejectReason::ContentMismatch);
        }
        None
    }

    fn send_input_rejected(
        &mut self,
        player: PlayerId,
        batch: &InputBatch,
        err: &crate::InputError,
    ) -> Result<(), ServerError> {
        let reason = match err {
            crate::InputError::OutOfWindow { .. } => InputRejectReason::OutOfWindow,
            crate::InputError::GapTooLarge { .. } => InputRejectReason::GapTooLarge,
            crate::InputError::RateLimited { .. } => InputRejectReason::RateLimited,
            _ => InputRejectReason::BadSequence,
        };
        let first_seq = batch.commands.first().map_or(0, |c| c.seq);
        let applied_seq = self.authority.applied_seq(player).unwrap_or(0);
        let server_tick = self.authority.tick().unwrap_or(0);
        self.send(
            player,
            &InputRejected {
                round_id: batch.round_id,
                reason,
                applied_seq,
                first_seq,
                command_count: u8::try_from(batch.commands.len()).unwrap_or(u8::MAX),
                server_tick,
            },
        )
    }

    fn send<M: Message>(&mut self, player: PlayerId, message: &M) -> Result<(), ServerError> {
        let limits = self.authority.config().protocol_limits;
        let frame = encode_frame(message, &limits)?;
        self.send_raw(player, &frame)
    }

    fn send_raw(&mut self, player: PlayerId, frame: &[u8]) -> Result<(), ServerError> {
        let full = {
            let Some(session) = self.sessions.get(&player) else {
                return Ok(());
            };
            match session.out_tx.try_send(Outbound::Frame(frame.to_vec())) {
                Ok(()) => false,
                Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => true,
            }
        };
        if full {
            // Slow consumer: đóng theo đường disconnect V1-023, không chặn tick.
            self.close_session(player);
        }
        Ok(())
    }

    fn send_snapshot(&mut self, player: PlayerId) -> Result<(), ServerError> {
        if !self.is_member(player) {
            return Ok(());
        }
        let Some(session) = self.sessions.get(&player) else {
            return Ok(());
        };
        let next_seq = session.snapshot_seq.saturating_add(1);
        let Some(snapshot) = self
            .authority
            .snapshot(next_seq, self.handshake.content_epoch)
        else {
            return Ok(());
        };
        let snapshot = snapshot?;
        self.send(player, &snapshot)?;
        if let Some(session) = self.sessions.get_mut(&player) {
            session.snapshot_seq = next_seq;
            session.next_snapshot_ms =
                self.now_ms + u64::from(self.handshake.snapshot_interval_ms.max(1));
        }
        Ok(())
    }

    fn is_member(&self, player: PlayerId) -> bool {
        self.authority
            .room_view()
            .is_some_and(|view| view.members.iter().any(|m| m.player == player))
    }

    fn flush_snapshots(&mut self, now_ms: u64) -> Result<(), ServerError> {
        let due: Vec<PlayerId> = self
            .sessions
            .iter()
            .filter(|(p, s)| s.handshaken && now_ms >= s.next_snapshot_ms && self.is_member(**p))
            .map(|(p, _)| *p)
            .collect();
        for player in due {
            self.send_snapshot(player)?;
        }
        Ok(())
    }

    fn flush_events(&mut self) -> Result<(), ServerError> {
        let events = self.authority.take_events();
        if events.is_empty() {
            return Ok(());
        }
        let Some(round) = self.authority.current_round_id() else {
            return Ok(());
        };
        let message = to_round_events(round, &events);
        let players: Vec<PlayerId> = self
            .sessions
            .iter()
            .filter(|(p, s)| s.handshaken && self.is_member(**p))
            .map(|(p, _)| *p)
            .collect();
        for player in players {
            self.send(player, &message)?;
        }
        Ok(())
    }

    fn close_session(&mut self, player: PlayerId) {
        if self.sessions.remove(&player).is_some() {
            let _ = self.authority.disconnect(player, self.now_ms);
        }
    }

    /// Đóng session nhưng để writer gửi nốt frame đã xếp (vd. `Reject` handshake, ADR 0009 §1).
    /// Bỏ `out_tx` làm writer thoát sau khi rút hết queue; một luồng canh gọi shutdown khi writer
    /// xong hoặc quá `GRACEFUL_CLOSE_MS`, nên writer kẹt vẫn bị thu hồi và tick không phải chờ.
    fn close_session_after_flush(&mut self, player: PlayerId) {
        let Some(mut session) = self.sessions.remove(&player) else {
            return;
        };
        let _ = self.authority.disconnect(player, self.now_ms);
        let shutdown = session.shutdown.take();
        let (dummy_tx, dummy_rx) = mpsc::channel();
        drop(dummy_tx);
        let writer_done = std::mem::replace(&mut session.writer_done, dummy_rx);
        drop(session);
        thread::spawn(move || {
            let _ = writer_done.recv_timeout(Duration::from_millis(GRACEFUL_CLOSE_MS));
            if let Some(shutdown) = shutdown {
                shutdown();
            }
        });
    }

    // --- Điều khiển authority cho test/lớp backend khi chưa có DTO room/loading/result. ---

    pub fn room_command(
        &mut self,
        command: RoomCommand,
        now_ms: u64,
    ) -> Result<RoomUpdate, ServerError> {
        if !self
            .sessions
            .get(&command.player)
            .is_some_and(|session| session.handshaken)
        {
            return Err(ServerError::NoSession(command.player));
        }
        self.authority.room_command(command, now_ms)
    }

    pub fn ack_loading(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.authority.ack_loading(player, now_ms)
    }

    pub fn ack_result(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.authority.ack_result(player, now_ms)
    }

    pub fn disconnect(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.authority.disconnect(player, now_ms)
    }

    pub fn player_life(&self, player: PlayerId) -> Option<Life> {
        self.authority.player_life(player)
    }
}
