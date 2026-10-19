//! Vòng IO mỏng trên `boom-net` (ADR 0006) + handshake V1-002 (ADR 0009 §1).
//!
//! Mỗi kết nối có **một luồng đọc** gọi `recv_frame` chặn và đẩy frame qua channel vào vòng tick
//! của server. `send_frame` được gọi từ vòng tick trên nửa ghi của cùng kết nối; vì vậy test dùng
//! hai đầu `MemoryTransport` cho hai chiều. Không gọi `recv_frame` trong vòng tick.
//!
//! Room/loading/result vẫn đi qua API authority (chưa có DTO `RoomCommand`/`LoadingAck` trong
//! `boom-proto`); `Server` cung cấp wrapper để test và lớp backend sau này gắn message.

use std::collections::BTreeMap;
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use boom_core::Life;
use boom_net::{NetError, TcpTransport, Transport};
use boom_proto::{
    Hello, InputBatch, InputRejectReason, InputRejected, Message, Ping, Pong, Reject, RejectReason,
    SnapshotRequest, Welcome, decode_frame, encode_frame, limits::WIRE_SCHEMA, read_envelope,
};
use boom_room::{PlayerId, RoomCommand, RoomView};
use boom_sync::to_round_events;

use crate::authority::{
    Authority, InternalRoundResult, InternalServerEvent, RoomUpdate, ServerError, ServerPhase,
};
use crate::config::HandshakeConfig;

enum Inbound {
    Frame { player: PlayerId, frame: Vec<u8> },
    Closed { player: PlayerId },
}

struct Session {
    slot: u8,
    session_id: String,
    handshaken: bool,
    writer: Box<dyn Transport + Send>,
    next_snapshot_ms: u64,
    snapshot_seq: u32,
}

impl Session {
    fn close(&mut self) {
        let _ = self.writer.close();
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

    /// Nhận một nửa đọc và một nửa ghi của kết nối; spawn đúng một luồng đọc chặn.
    pub fn accept<R, W>(
        &mut self,
        player: PlayerId,
        slot: u8,
        mut reader: R,
        writer: W,
    ) -> Result<(), ServerError>
    where
        R: Transport + Send + 'static,
        W: Transport + Send + 'static,
    {
        if self.sessions.contains_key(&player) {
            return Err(ServerError::SessionExists(player));
        }
        let session_id = format!("s{}", self.next_session);
        self.next_session = self.next_session.saturating_add(1);
        self.sessions.insert(
            player,
            Session {
                slot,
                session_id,
                handshaken: false,
                writer: Box::new(writer),
                next_snapshot_ms: 0,
                snapshot_seq: 0,
            },
        );
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
        let reader = TcpTransport::from_stream(stream)?;
        let writer = TcpTransport::from_stream(writer_stream)?;
        self.accept(player, slot, reader, writer)
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
            self.handle_inbound(item)?;
        }
        let events = self.authority.advance_to(now_ms)?;
        self.flush_events()?;
        self.flush_snapshots(now_ms)?;
        Ok(events)
    }

    fn handle_inbound(&mut self, inbound: Inbound) -> Result<(), ServerError> {
        match inbound {
            Inbound::Closed { player } => {
                if self.sessions.remove(&player).is_some() {
                    let _ = self.authority.disconnect(player, self.now_ms);
                }
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
        match envelope.kind {
            boom_proto::MessageKind::InputBatch => {
                let batch: InputBatch =
                    decode_frame(frame, &self.authority.config().protocol_limits)?;
                match self.authority.submit_input(player, &batch, self.now_ms) {
                    Ok(_) => {}
                    Err(err) => self.send_input_rejected(player, &batch, &err)?,
                }
            }
            boom_proto::MessageKind::SnapshotRequest => {
                let _: SnapshotRequest =
                    decode_frame(frame, &self.authority.config().protocol_limits)?;
                self.send_snapshot(player)?;
            }
            boom_proto::MessageKind::Ping => {
                let ping: Ping = decode_frame(frame, &self.authority.config().protocol_limits)?;
                self.send(player, &Pong(ping.0))?;
            }
            boom_proto::MessageKind::Pong => {}
            _ => {
                self.close_session(player);
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
            self.close_session(player);
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
        let failed = {
            let Some(session) = self.sessions.get_mut(&player) else {
                return Ok(());
            };
            session.writer.send_frame(frame).is_err()
        };
        if failed {
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
        if let Some(mut session) = self.sessions.remove(&player) {
            session.close();
            let _ = self.authority.disconnect(player, self.now_ms);
        }
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
