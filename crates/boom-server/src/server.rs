//! Vòng IO mỏng trên `boom-net` (ADR 0006) + handshake V1-002 (ADR 0009 §1).
//!
//! Mỗi kết nối có **một luồng đọc** gọi `recv_frame` chặn và đẩy frame qua channel vào vòng tick
//! của server. `send_frame` được gọi từ vòng tick trên nửa ghi của cùng kết nối; vì vậy test dùng
//! hai đầu `MemoryTransport` cho hai chiều. Không gọi `recv_frame` trong vòng tick.
//!
//! Auth T5 đi qua wire Addendum A: `Hello` → `Welcome` (fingerprint) → `Auth` →
//! `AuthOk`/`AuthFail`; `Logout` revoke + đóng; Ping/Pong liveness. Room/loading/result đi qua
//! Addendum B sau `AuthOk`. Server bind `player`/account từ session, không tin payload.

use std::collections::BTreeMap;
use std::fmt;
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
use std::thread;
use std::time::Duration;

use boom_core::Life;
use boom_net::{NetError, TcpTransport, Transport};
use boom_proto::dto::auth::{Auth, AuthFail, AuthFailReason, AuthOk, AuthOp, Logout};
use boom_proto::{
    Hello, InputBatch, InputRejectReason, InputRejected, LoadingAck, LobbyList, LobbyListRequest,
    Message, Ping, Pong, Reject, RejectReason, ResultAck, RoomChat, RoomChatMessage,
    RoomCommand as RoomCommandDto, RoomOutcome, RoomRejected, RoundResult, RoundStart,
    SnapshotRequest, Welcome, decode_frame, decode_payload, encode_frame, limits::WIRE_SCHEMA,
    read_envelope,
};
use boom_room::{PlayerId, RoomCommand, RoomView, RoundId};
use boom_sync::{
    RoomGate, from_room_command, reject_reason, to_lobby_list, to_room_state, to_round_events,
};

use crate::auth::{AuthError, AuthOutcome, AuthService};
use crate::authority::{
    Authority, InternalRoundResult, InternalServerEvent, MatchOutcome, RoomUpdate, ServerError,
    ServerPhase,
};
use crate::config::HandshakeConfig;
use crate::identity::{AccountStore, RevokeReason, SessionId, SessionStore};

fn room_command_reject_reason(err: &ServerError) -> Option<boom_proto::RoomRejectReason> {
    use boom_proto::RoomRejectReason as Reason;
    match err {
        ServerError::Room(e) => Some(reject_reason(e)),
        ServerError::WrongPhase { .. } => Some(Reason::NotWaiting),
        ServerError::RoomExists { .. } => Some(Reason::AlreadyStarted),
        ServerError::NoRoom => Some(Reason::RoomNotFound),
        ServerError::NoSession(_) | ServerError::UnknownPlayer(_) => Some(Reason::PlayerNotInRoom),
        _ => None,
    }
}

/// Cổng store mà auth cần: tài khoản + vòng đời phiên. Trait object để `Server` giữ
/// `SqliteAccountStore` thật mà không phụ thuộc driver.
pub trait AuthStore: AccountStore + SessionStore {}
impl<T: AccountStore + SessionStore> AuthStore for T {}

/// Lỗi khởi tạo auth của [`Server`] (boot cleanup store).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthInitError(pub String);
impl fmt::Display for AuthInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AuthInitError {}

struct AuthRuntime {
    service: AuthService,
    store: Box<dyn AuthStore>,
}

impl AuthRuntime {
    fn boot_cleanup(&mut self, now_ms: i64) -> Result<usize, crate::identity::StoreError> {
        self.service
            .sessions()
            .boot_cleanup(self.store.as_mut(), now_ms)
    }

    fn login(
        &mut self,
        op: AuthOp,
        username: &str,
        password: &str,
        source: &str,
        connection_id: &str,
        now_ms: i64,
    ) -> Result<AuthOutcome, AuthError> {
        match op {
            AuthOp::Login => self.service.login(
                self.store.as_mut(),
                username,
                password,
                source,
                connection_id,
                now_ms,
            ),
            AuthOp::Register => self.service.register(
                self.store.as_mut(),
                username,
                password,
                source,
                connection_id,
                now_ms,
            ),
        }
    }

    fn revoke(
        &mut self,
        session_id: SessionId,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<bool, crate::identity::StoreError> {
        match reason {
            RevokeReason::Logout => {
                self.service
                    .sessions()
                    .logout(self.store.as_mut(), session_id, now_ms)
            }
            RevokeReason::TimeoutNoInput => {
                self.service
                    .sessions()
                    .on_liveness_timeout(self.store.as_mut(), session_id, now_ms)
            }
            _ => self
                .service
                .sessions()
                .on_disconnect(self.store.as_mut(), session_id, now_ms),
        }
    }

    fn touch(&mut self, session_id: SessionId, now_ms: i64) {
        let _ = self
            .service
            .sessions()
            .touch(self.store.as_mut(), session_id, now_ms);
    }

    fn shutdown_all(&mut self, now_ms: i64) {
        let _ = self
            .service
            .sessions()
            .shutdown(self.store.as_mut(), now_ms);
    }
}

#[derive(Clone, Debug)]
struct AuthSession {
    display_name: String,
    session_id: SessionId,
    last_pong_ms: u64,
    next_ping_ms: u64,
}

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
    /// Cổng kind phòng: false trước `AuthOk`.
    auth: RoomGate,
    /// Phiên tài khoản đã `AuthOk`; `None` trước auth.
    account: Option<AuthSession>,
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
    /// Auth + store thật; `None` chỉ dành cho test handshake thuần.
    auth: Option<AuthRuntime>,
    server_version: String,
    ping_interval_ms: u64,
    liveness_timeout_ms: u64,
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
            auth: None,
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
            ping_interval_ms: 15_000,
            liveness_timeout_ms: 30_000,
        }
    }

    /// Tạo server có auth/store thật và chạy boot cleanup theo `instance_id` của
    /// `AuthService` trước khi nhận kết nối (ADR 0011 §6.1).
    pub fn with_auth(
        authority: Authority,
        handshake: HandshakeConfig,
        auth: AuthService,
        store: Box<dyn AuthStore>,
        now_ms: u64,
    ) -> Result<Self, AuthInitError> {
        Self::with_auth_and_timers(authority, handshake, auth, store, now_ms, 15_000, 30_000)
    }

    /// Như [`Self::with_auth`] nhưng đặt chu kỳ Ping/liveness (test dùng đồng hồ tiêm).
    pub fn with_auth_and_timers(
        authority: Authority,
        handshake: HandshakeConfig,
        auth: AuthService,
        store: Box<dyn AuthStore>,
        now_ms: u64,
        ping_interval_ms: u64,
        liveness_timeout_ms: u64,
    ) -> Result<Self, AuthInitError> {
        let now_i64 = i64::try_from(now_ms).unwrap_or(i64::MAX);
        let mut runtime = AuthRuntime {
            service: auth,
            store,
        };
        runtime
            .boot_cleanup(now_i64)
            .map_err(|err| AuthInitError(format!("boot cleanup auth thất bại: {err}")))?;
        let mut server = Self::new(authority, handshake);
        server.auth = Some(runtime);
        server.now_ms = now_ms;
        server.ping_interval_ms = ping_interval_ms.max(1);
        server.liveness_timeout_ms = liveness_timeout_ms;
        Ok(server)
    }

    /// Build id của server trong `Welcome`/fingerprint.
    pub fn set_server_version(&mut self, version: impl Into<String>) {
        self.server_version = version.into();
    }

    pub fn authority(&self) -> &Authority {
        &self.authority
    }

    pub fn phase(&self) -> ServerPhase {
        self.authority.phase()
    }

    /// Đồng hồ server hiện tại (ms) do caller tiêm qua [`Server::poll`].
    pub fn now_ms(&self) -> u64 {
        self.now_ms
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
                account: None,
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
        self.flush_liveness(now_ms)?;
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
            let (envelope, _) = read_envelope(frame)?;
            if envelope.kind == boom_proto::MessageKind::Hello {
                return self.handle_hello(player, frame);
            }
            self.reject_session_before_welcome(player, envelope.kind);
            return Ok(());
        }
        let (envelope, _) = read_envelope(frame)?;
        let kind = envelope.kind;
        // Hello lặp (vd. test mạng lossy) được trả lại Welcome thay vì đóng; TCP thật không
        // sinh duplicate nhưng đây là idempotent an toàn cho handshake.
        if kind == boom_proto::MessageKind::Hello {
            return self.handle_hello(player, frame);
        }
        let authenticated = self
            .sessions
            .get(&player)
            .is_some_and(|session| session.account.is_some());
        if !authenticated {
            return match kind {
                boom_proto::MessageKind::Auth => self.handle_auth(player, frame),
                boom_proto::MessageKind::Ping => {
                    let ping: Ping = decode_frame(frame, &self.authority.config().protocol_limits)?;
                    self.send(player, &Pong(ping.0))
                }
                boom_proto::MessageKind::Pong => {
                    let _: Pong = decode_frame(frame, &self.authority.config().protocol_limits)?;
                    Ok(())
                }
                _ => {
                    self.reject_session_before_auth(player, kind);
                    Ok(())
                }
            };
        }
        if kind == boom_proto::MessageKind::Auth {
            // Auth chỉ hợp lệ một lần thành công mỗi kết nối.
            self.send(
                player,
                &Reject {
                    reason: RejectReason::SessionRejected,
                    detail: "Auth lần hai sau AuthOk là lỗi giao thức".to_owned(),
                },
            )?;
            self.close_session(player);
            return Ok(());
        }
        if kind == boom_proto::MessageKind::Logout {
            let _: Logout = decode_frame(frame, &self.authority.config().protocol_limits)?;
            self.close_session_after_flush_with_reason(player, RevokeReason::Logout);
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
            boom_proto::MessageKind::Pong => {
                let _: Pong = decode_frame(frame, &limits)?;
                self.handle_pong(player);
            }
            boom_proto::MessageKind::LobbyListRequest => {
                let _: LobbyListRequest = decode_frame(frame, &limits)?;
                self.send_lobby_list(player)?;
            }
            boom_proto::MessageKind::RoomCommand => {
                let dto: RoomCommandDto = decode_frame(frame, &limits)?;
                let command = from_room_command(&dto, player);
                match self.room_command(command, self.now_ms) {
                    Ok(update) => self.after_room_update(&update)?,
                    Err(err) => {
                        if let Some(reason) = room_command_reject_reason(&err) {
                            self.send_room_rejected(player, dto.request, reason)?;
                        } else {
                            return Err(err);
                        }
                    }
                }
            }
            boom_proto::MessageKind::RoomChat => {
                let chat: RoomChat = decode_frame(frame, &limits)?;
                self.handle_room_chat(player, &chat)?;
            }
            boom_proto::MessageKind::LoadingAck => {
                let ack: LoadingAck = decode_frame(frame, &limits)?;
                if !self.ack_round_matches(ack.round_id) {
                    self.send_room_rejected(
                        player,
                        ack.round_id,
                        boom_proto::RoomRejectReason::NotWaiting,
                    )?;
                } else if self.authority.phase() == ServerPhase::Playing {
                    // Ack lặp sau barrier: idempotent, không đổi phase.
                } else {
                    match self.authority.ack_loading(player, self.now_ms) {
                        Ok(events) => self.flush_lifecycle(&events)?,
                        Err(_) => self.send_room_rejected(
                            player,
                            ack.round_id,
                            boom_proto::RoomRejectReason::NotWaiting,
                        )?,
                    }
                }
            }
            boom_proto::MessageKind::ResultAck => {
                let ack: ResultAck = decode_frame(frame, &limits)?;
                if !self.ack_round_matches(ack.round_id) {
                    self.send_room_rejected(
                        player,
                        ack.round_id,
                        boom_proto::RoomRejectReason::NotWaiting,
                    )?;
                } else if self.authority.phase() == ServerPhase::Waiting {
                    // ResultAck lặp sau khi đã về phòng: idempotent.
                } else {
                    match self.authority.ack_result(player, self.now_ms) {
                        Ok(events) => self.flush_lifecycle(&events)?,
                        Err(_) => self.send_room_rejected(
                            player,
                            ack.round_id,
                            boom_proto::RoomRejectReason::NotWaiting,
                        )?,
                    }
                }
            }
            _ => {
                self.close_session(player);
            }
        }
        Ok(())
    }

    fn reject_session_before_welcome(&mut self, player: PlayerId, kind: boom_proto::MessageKind) {
        let _ = self.send(
            player,
            &Reject {
                reason: RejectReason::SessionRejected,
                detail: format!("{kind:?} chỉ hợp lệ sau Hello/Welcome"),
            },
        );
        self.close_session_after_flush(player);
    }

    fn reject_session_before_auth(&mut self, player: PlayerId, kind: boom_proto::MessageKind) {
        let _ = self.send(
            player,
            &Reject {
                reason: RejectReason::SessionRejected,
                detail: format!("{kind:?} chỉ hợp lệ sau AuthOk"),
            },
        );
        self.close_session_after_flush(player);
    }

    fn handle_auth(&mut self, player: PlayerId, frame: &[u8]) -> Result<(), ServerError> {
        let limits = self.authority.config().protocol_limits;
        let (_, payload) = read_envelope(frame)?;
        let auth: Auth = match decode_payload(payload) {
            Ok(auth) => auth,
            Err(_) => {
                self.send(
                    player,
                    &AuthFail {
                        reason: AuthFailReason::InvalidInput,
                    },
                )?;
                return Ok(());
            }
        };
        if auth.limit_violation(&limits).is_some() {
            self.send(
                player,
                &AuthFail {
                    reason: AuthFailReason::InvalidInput,
                },
            )?;
            return Ok(());
        }
        let Some(session) = self.sessions.get(&player) else {
            return Ok(());
        };
        let connection_id = session.session_id.clone();
        let source = format!("connection:{connection_id}");
        let now = i64::try_from(self.now_ms).unwrap_or(i64::MAX);
        let Some(runtime) = self.auth.as_mut() else {
            self.send(
                player,
                &AuthFail {
                    reason: AuthFailReason::Unavailable,
                },
            )?;
            return Ok(());
        };
        match runtime.login(
            auth.op,
            &auth.username,
            &auth.password,
            &source,
            &connection_id,
            now,
        ) {
            Ok(outcome) => {
                let success = outcome.success();
                let account_id = u64::try_from(success.account_id.0).unwrap_or(0);
                let expires_at_ms = u64::try_from(success.expires_at_ms).unwrap_or(u64::MAX);
                let session_id = success.session_id;
                let display_name = success.display_name.clone();
                let token = success.token.expose().to_owned();
                self.send(
                    player,
                    &AuthOk {
                        account_id,
                        display_name: display_name.clone(),
                        token,
                        expires_at_ms,
                    },
                )?;
                if let Some(session) = self.sessions.get_mut(&player) {
                    session.auth.on_auth_ok();
                    session.account = Some(AuthSession {
                        display_name,
                        session_id,
                        last_pong_ms: self.now_ms,
                        next_ping_ms: self.now_ms.saturating_add(self.ping_interval_ms),
                    });
                }
            }
            Err(err) => {
                self.send(
                    player,
                    &AuthFail {
                        reason: err.wire_reason(),
                    },
                )?;
            }
        }
        Ok(())
    }

    fn handle_pong(&mut self, player: PlayerId) {
        let now = self.now_ms;
        let session_id = if let Some(session) = self.sessions.get_mut(&player) {
            if let Some(account) = session.account.as_mut() {
                account.last_pong_ms = now;
                account.next_ping_ms = now.saturating_add(self.ping_interval_ms);
                Some(account.session_id)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(session_id) = session_id
            && let Some(auth) = self.auth.as_mut()
        {
            auth.touch(session_id, i64::try_from(now).unwrap_or(i64::MAX));
        }
    }

    fn ack_round_matches(&self, round: u64) -> bool {
        self.authority
            .current_round_id()
            .is_some_and(|current| u64::from(current) == round)
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
        let round = update.round.or_else(|| {
            if update.duplicate && self.authority.phase() == ServerPhase::Loading {
                self.authority
                    .room_view()
                    .and_then(|view| view.round)
                    .map(|r| r.id)
            } else {
                None
            }
        });
        if let Some(round) = round {
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

    fn lobby_list(&self) -> LobbyList {
        let views: Vec<RoomView> = self.authority.room_view().into_iter().collect();
        let mut list = to_lobby_list(&views);
        for room in &mut list.rooms {
            if let Some(account) = self
                .sessions
                .get(&PlayerId(room.host))
                .and_then(|session| session.account.as_ref())
            {
                room.name = account.display_name.clone();
            }
        }
        list
    }

    fn send_lobby_list(&mut self, player: PlayerId) -> Result<(), ServerError> {
        let list = self.lobby_list();
        self.send(player, &list)
    }

    fn broadcast_lobby_list(&mut self) -> Result<(), ServerError> {
        let list = self.lobby_list();
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
        let public_round = view.round.as_ref().map_or(round.0, |r| r.id.0);
        let message = RoundStart {
            round_id: public_round,
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
        let from_name = self
            .sessions
            .get(&player)
            .and_then(|session| session.account.as_ref())
            .map(|account| account.display_name.clone())
            .unwrap_or_else(|| format!("player{}", player.0));
        let message = RoomChatMessage {
            room_id: view.id.0,
            from: player.0,
            from_name,
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
        let outcome = winner.map_or(RoomOutcome::Draw, |p| RoomOutcome::Winner(p.0));
        let results = roster
            .into_iter()
            .map(|member| {
                let player = PlayerId(member.player);
                let alive = self
                    .authority
                    .round_player_life(player)
                    .is_some_and(|life| life != Life::Dead);
                boom_proto::PlayerResult {
                    player: member.player,
                    slot: member.slot,
                    // Chưa có spec/design thứ hạng V1-022; để 0 (unknown), không tự đặt policy.
                    rank: 0,
                    alive,
                }
            })
            .collect();
        let public_round = view.round.as_ref().map_or(result.round.0, |r| r.id.0);
        let message = RoundResult {
            round_id: public_round,
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
            server_version: self.server_version.clone(),
            rules_profile_id: self.handshake.rules_profile_id.clone(),
            rules_hash: self.handshake.rules_hash,
            content_sha256: self.handshake.content_sha256,
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

    fn flush_liveness(&mut self, now_ms: u64) -> Result<(), ServerError> {
        let timeout = self.liveness_timeout_ms.max(1);
        let timed_out: Vec<PlayerId> =
            self.sessions
                .iter()
                .filter(|(_, session)| {
                    session.account.as_ref().is_some_and(|account| {
                        now_ms.saturating_sub(account.last_pong_ms) >= timeout
                    })
                })
                .map(|(player, _)| *player)
                .collect();
        for player in timed_out {
            self.close_session_with_reason(player, RevokeReason::TimeoutNoInput);
        }
        let due: Vec<PlayerId> = self
            .sessions
            .iter()
            .filter(|(_, session)| {
                session
                    .account
                    .as_ref()
                    .is_some_and(|account| now_ms >= account.next_ping_ms)
            })
            .map(|(player, _)| *player)
            .collect();
        for player in due {
            self.send(player, &Ping(now_ms))?;
            if let Some(session) = self.sessions.get_mut(&player)
                && let Some(account) = session.account.as_mut()
            {
                account.next_ping_ms = now_ms.saturating_add(self.ping_interval_ms);
            }
        }
        Ok(())
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
        self.close_session_with_reason(player, RevokeReason::Disconnect);
    }

    fn close_session_with_reason(&mut self, player: PlayerId, reason: RevokeReason) {
        let Some(session) = self.sessions.remove(&player) else {
            return;
        };
        self.revoke_account(&session, reason);
        self.notify_disconnect(player);
    }

    fn revoke_account(&mut self, session: &Session, reason: RevokeReason) {
        let Some(account) = session.account.as_ref() else {
            return;
        };
        if let Some(auth) = self.auth.as_mut() {
            let _ = auth.revoke(
                account.session_id,
                reason,
                i64::try_from(self.now_ms).unwrap_or(i64::MAX),
            );
        }
    }

    fn notify_disconnect(&mut self, player: PlayerId) {
        if let Ok(events) = self.authority.disconnect(player, self.now_ms) {
            let _ = self.flush_lifecycle(&events);
        }
        if let Some(view) = self.authority.room_view() {
            let _ = self.send_room_state(&view);
        }
        let _ = self.broadcast_lobby_list();
    }

    /// Đóng session nhưng để writer gửi nốt frame đã xếp (vd. `Reject` handshake, ADR 0009 §1).
    /// Bỏ `out_tx` làm writer thoát sau khi rút hết queue; một luồng canh gọi shutdown khi writer
    /// xong hoặc quá `GRACEFUL_CLOSE_MS`, nên writer kẹt vẫn bị thu hồi và tick không phải chờ.
    fn close_session_after_flush(&mut self, player: PlayerId) {
        self.close_session_after_flush_with_reason(player, RevokeReason::Disconnect);
    }

    fn close_session_after_flush_with_reason(&mut self, player: PlayerId, reason: RevokeReason) {
        let Some(mut session) = self.sessions.remove(&player) else {
            return;
        };
        self.revoke_account(&session, reason);
        self.notify_disconnect(player);
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

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(auth) = self.auth.as_mut() {
            auth.shutdown_all(i64::try_from(self.now_ms).unwrap_or(i64::MAX));
        }
    }
}
