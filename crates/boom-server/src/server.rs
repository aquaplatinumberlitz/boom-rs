//! Vòng IO mỏng trên `boom-net` (ADR 0006) + handshake V1-002 (ADR 0009 §1).
//!
//! Mỗi kết nối có **một luồng đọc** gọi `recv_frame` chặn và đẩy frame qua channel vào vòng tick
//! của server. `send_frame` được gọi từ vòng tick trên nửa ghi của cùng kết nối; vì vậy test dùng
//! hai đầu `MemoryTransport` cho hai chiều. Không gọi `recv_frame` trong vòng tick.
//!
//! Auth T5 đi qua wire Addendum A: `Hello` → `Welcome` (fingerprint) → `Auth` →
//! `AuthOk`/`AuthFail`; `Logout` revoke + đóng; Ping/Pong liveness. Room/loading/result đi qua
//! Addendum B sau `AuthOk`. Server bind `player`/account từ session, không tin payload.
//!
//! Snapshot của lượt chỉ đi **sau** loading barrier (ADR 0009 §4.3): lúc phòng còn ở loading,
//! state trận chưa chạy (`tick == 0`) nên trên dây không có `Snapshot` nào.

use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, TcpListener};
use std::sync::Arc;
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
use boom_room::{PlayerId, RoomCommand, RoomEvent, RoomView, RoundId};
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
use crate::observability::{
    DisconnectReason, LogFields, Logger, RoomContext, events, phases, round_fields,
};

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

/// Nhãn kết quả round ghi vào log (`winner`/`draw`/`aborted`).
fn outcome_label(outcome: &MatchOutcome) -> &'static str {
    match outcome {
        MatchOutcome::Winner(_) => "winner",
        MatchOutcome::Draw => "draw",
        MatchOutcome::Aborted => "aborted",
    }
}

/// Ngữ cảnh phòng/round của một tài khoản theo **một ảnh phòng** (ADR 0011 §8.1).
///
/// - `room_id` chỉ có khi tài khoản **đang là thành viên** phòng.
/// - `round_id` chỉ có khi tài khoản nằm trong **roster của round** (roster chốt lúc start).
///
/// Tài khoản đứng sảnh, connection chưa auth, người đã rời phòng ⇒ không có ngữ cảnh nào
/// (null) ⇒ log không gắn ID phòng/trận của người khác.
fn room_context(view: Option<&RoomView>, account: PlayerId) -> RoomContext {
    let Some(view) = view else {
        return RoomContext::NONE;
    };
    if !view.members.iter().any(|member| member.player == account) {
        return RoomContext::NONE;
    }
    let round_id = view
        .round
        .as_ref()
        .filter(|round| round.members.iter().any(|member| member.player == account))
        .and_then(|round| u32::try_from(round.id.0).ok());
    RoomContext {
        room_id: Some(view.id.0),
        round_id,
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

/// Tên hiển thị **fallback** khi account store không còn giữ tài khoản.
///
/// Thành viên đã mất phiên dùng `player{account_id}` — giống `RoomChatMessage.from_name` — để
/// UI không hiện dòng rỗng (ADR 0009 Addendum B §B.6).
pub(crate) fn fallback_display_name(account: PlayerId) -> String {
    format!("player{}", account.0)
}

#[derive(Clone, Debug)]
struct AuthSession {
    /// ID tài khoản: **định danh người chơi trong phòng/round** (ADR 0009 Addendum B §B.2).
    ///
    /// Mọi trường `player` trên dây (`RoomState`, `RoundStart.roster`, `RoomChatMessage`,
    /// `RoundResult`) mang giá trị này; server bind từ phiên `AuthOk`, không tin payload.
    account_id: u64,
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
    Barrier(Sender<()>),
}

/// Hàng đợi ghi có giới hạn theo kết nối; client không đọc thì bị ngắt thay vì chặn tick.
const OUTBOUND_QUEUE_FRAMES: usize = 32;

/// Hạn chờ writer gửi nốt frame đã xếp (vd. `Reject`) trước khi đóng cưỡng bức socket. Giá trị
/// vận hành (design), không phải luật chơi; không chặn tick vì chờ ở luồng riêng.
const GRACEFUL_CLOSE_MS: u64 = 2_000;

struct Session {
    /// Định danh **kết nối** trong phiên (đưa vào `Welcome.player_id`).
    ///
    /// **Không phải** slot trận: slot trận lấy từ roster round theo `account_id`
    /// (ADR 0009 Addendum B §B.2). Số thứ tự kết nối TCP không được dùng làm slot.
    connection_id: u8,
    session_id: String,
    /// IP nguồn của kết nối (không lấy port) — bucket rate limit theo nguồn ADR 0011 §5.
    source_ip: IpAddr,
    /// Mốc accept (ms) để áp hạn handshake/auth.
    connected_at_ms: u64,
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
    /// Kết nối đang mở, khóa theo **định danh kết nối** (`PlayerId` cấp lúc accept).
    sessions: BTreeMap<PlayerId, Session>,
    /// Ánh xạ **định danh người chơi trong phòng** (`account_id`) → kết nối đang giữ nó.
    ///
    /// Một tài khoản chỉ có một phiên (ADR 0011 §6) nên ánh xạ là 1-1; dùng để gửi tin
    /// phòng/round về đúng kết nối mà không tin `player` do client khai.
    by_account: BTreeMap<PlayerId, PlayerId>,
    /// Số batch input bị **bỏ** vì client khai slot khác slot server bind từ phiên.
    input_slot_mismatch: u64,
    /// `InputRetired` phát sinh ngay trong `InputTrack::submit` (retire đầu hàng khi nhận batch).
    ///
    /// `Authority::submit_input` trả các sequence này trong `InputAck.retired` nhưng chỉ
    /// `apply_tick` phát `InternalServerEvent::InputRetired`. `poll` chuyển chúng thành event
    /// cùng room/round/account và trả cho caller, để retire của nhánh submit không bị bỏ và
    /// caller đo `applied_seq` thấy đúng tập đã giải quyết.
    pending_input_retired: Vec<InternalServerEvent>,
    now_ms: u64,
    next_session: u64,
    /// Auth + store thật; `None` chỉ dành cho test handshake thuần.
    auth: Option<AuthRuntime>,
    /// Log JSON Lines (ADR 0011 §8.1); `None` khi không gắn (test).
    logger: Option<Arc<Logger>>,
    server_version: String,
    ping_interval_ms: u64,
    liveness_timeout_ms: u64,
    /// Hạn hoàn tất handshake+auth cho kết nối chưa `AuthOk`.
    auth_deadline_ms: u64,
    /// Đã ghi `player.disconnect` lý do `server_shutdown` cho phiên của instance này.
    ///
    /// `shutdown_sessions` được gọi cả lúc dừng sạch lẫn trong `Drop` ⇒ cờ này giúp log
    /// `player.disconnect` chỉ xuất hiện **một lần** cho mỗi phiên.
    shutdown_logged: bool,
    /// Kết quả public bất biến, gắn với phòng (ADR 0009 §B.7).
    committed_result: Option<(u64, RoundResult)>,
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
            by_account: BTreeMap::new(),
            input_slot_mismatch: 0,
            pending_input_retired: Vec::new(),
            now_ms: 0,
            next_session: 1,
            auth: None,
            logger: None,
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
            ping_interval_ms: 15_000,
            liveness_timeout_ms: 30_000,
            auth_deadline_ms: 10_000,
            shutdown_logged: false,
            committed_result: None,
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

    /// Gắn logger JSON Lines (ADR 0011 §8.1). Không bắt buộc: test không cần log.
    pub fn set_logger(&mut self, logger: Arc<Logger>) {
        self.logger = Some(logger);
    }

    /// Đặt hạn handshake/auth cho kết nối chưa `AuthOk` (mặc định 10 s).
    pub fn set_auth_deadline_ms(&mut self, deadline_ms: u64) {
        self.auth_deadline_ms = deadline_ms.max(1);
    }

    /// Số batch input bị bỏ vì client khai slot khác slot server bind từ phiên.
    pub fn input_slot_mismatch_count(&self) -> u64 {
        self.input_slot_mismatch
    }

    /// Định danh người chơi trong phòng/round của một kết nối (`account_id` sau `AuthOk`).
    ///
    /// `None` khi kết nối chưa `AuthOk` ⇒ chưa có định danh để bind lệnh phòng.
    fn room_player(&self, connection: PlayerId) -> Option<PlayerId> {
        self.sessions
            .get(&connection)
            .and_then(|session| session.account.as_ref())
            .map(|account| PlayerId(account.account_id))
    }

    /// Kết nối đang giữ một định danh người chơi trong phòng.
    fn connection_of(&self, account: PlayerId) -> Option<PlayerId> {
        self.by_account.get(&account).copied()
    }

    /// Tên hiển thị của một định danh người chơi (`account_id`), lấy từ account store.
    ///
    /// Tên hiển thị do **server** điền (ADR 0009 Addendum B §B.6): client không khai tên.
    /// Thành viên đã mất phiên (không còn session giữ `account_id`) dùng fallback
    /// [`fallback_display_name`] — giống `RoomChatMessage.from_name` — để UI không hiện dòng
    /// rỗng.
    fn display_name_of(&self, account: PlayerId) -> String {
        self.connection_of(account)
            .and_then(|connection| self.sessions.get(&connection))
            .and_then(|session| session.account.as_ref())
            .map(|account| account.display_name.clone())
            .unwrap_or_else(|| fallback_display_name(account))
    }

    /// Định danh người chơi trong phòng mà một kết nối được bind.
    ///
    /// Sau `AuthOk` đó là `account_id` (ADR 0009 Addendum B §B.2). Kết nối **chưa** có
    /// account thì dùng chính định danh kết nối: trên dây, kind phòng trước `AuthOk` đã bị
    /// `RoomGate` chặn, nên nhánh này chỉ cho lớp backend/test chưa bật auth.
    fn bound_player(&self, connection: PlayerId) -> PlayerId {
        self.room_player(connection).unwrap_or(connection)
    }

    /// Thu hồi **mọi phiên của instance này** trong store (ADR 0011 §6.1).
    ///
    /// Dùng khi server dừng có trật tự (Ctrl-C/SIGTERM): phiên bị đánh dấu thu hồi nên
    /// người chơi đăng nhập lại được, còn `Server::drop` gọi lại hàm này (thao tác idempotent).
    pub fn shutdown_sessions(&mut self, now_ms: u64) {
        if self.shutdown_logged {
            return;
        }
        self.shutdown_logged = true;
        if let Some(auth) = self.auth.as_mut() {
            auth.shutdown_all(i64::try_from(now_ms).unwrap_or(i64::MAX));
        }
        // Mỗi người chơi đang có phiên: ghi `player.disconnect` lý do `server_shutdown` để
        // vận hành biết ai rớt khi server dừng (ADR 0011 §8.1).
        let accounts: Vec<(PlayerId, Option<SessionId>)> = self
            .sessions
            .values()
            .filter_map(|session| {
                session
                    .account
                    .as_ref()
                    .map(|account| (PlayerId(account.account_id), Some(account.session_id)))
            })
            .collect();
        for (account, session_id) in accounts {
            let context = self.member_context(Some(account));
            self.log_player_disconnect(
                Some(account),
                DisconnectReason::ServerShutdown,
                RevokeReason::ServerShutdown,
                session_id,
                context,
            );
        }
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

    /// Chờ writer gửi hết các frame đã xếp, phục vụ điều phối IO/fixture.
    /// Không poll, không tiến đồng hồ authority; `timeout` chỉ là trần an toàn.
    pub fn flush_outbound(&self, timeout: Duration) -> Result<(), String> {
        let mut barriers = Vec::new();
        for session in self.sessions.values() {
            let (tx, rx) = mpsc::channel();
            session
                .out_tx
                .try_send(Outbound::Barrier(tx))
                .map_err(|err| err.to_string())?;
            barriers.push(rx);
        }
        for barrier in barriers {
            barrier
                .recv_timeout(timeout)
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    /// Nhận một nửa đọc và một nửa ghi của kết nối; spawn đúng một luồng đọc chặn.
    ///
    /// `connection_id` là **định danh kết nối** (đưa vào `Welcome.player_id`), không phải
    /// slot trận: slot trận lấy từ roster round theo `account_id` (ADR 0009 Addendum B §B.2).
    pub fn accept<R, W>(
        &mut self,
        player: PlayerId,
        connection_id: u8,
        source_ip: IpAddr,
        reader: R,
        writer: W,
    ) -> Result<(), ServerError>
    where
        R: Transport + Send + 'static,
        W: Transport + Send + 'static,
    {
        self.accept_with_shutdown(player, connection_id, source_ip, reader, writer, || {})
    }

    /// Như [`Server::accept`] nhưng có handle shutdown độc lập để hủy socket khi session bị loại,
    /// kể cả writer đang kẹt trong `send_frame` blocking.
    pub fn accept_with_shutdown<R, W, S>(
        &mut self,
        player: PlayerId,
        connection_id: u8,
        source_ip: IpAddr,
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
                connection_id,
                session_id,
                source_ip,
                connected_at_ms: self.now_ms,
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
                    Outbound::Barrier(done) => {
                        let _ = done.send(());
                    }
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
        connection_id: u8,
    ) -> Result<(), ServerError> {
        let (stream, peer) = listener.accept().map_err(NetError::from)?;
        let source_ip = peer.ip();
        let writer_stream = stream.try_clone().map_err(NetError::from)?;
        let shutdown_stream = writer_stream.try_clone().map_err(NetError::from)?;
        let reader = TcpTransport::from_stream(stream)?;
        let writer = TcpTransport::from_stream(writer_stream)?;
        let mut shutdown = TcpTransport::from_stream(shutdown_stream)?;
        self.accept_with_shutdown(
            player,
            connection_id,
            source_ip,
            reader,
            writer,
            move || {
                let _ = shutdown.close();
            },
        )
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
                if matches!(err, ServerError::Proto(_) | ServerError::Sync(_)) {
                    self.close_session_with_reason(
                        player,
                        RevokeReason::Disconnect,
                        DisconnectReason::ProtocolError,
                    );
                } else if matches!(err, ServerError::Net(_)) {
                    self.close_session(player);
                } else {
                    return Err(err);
                }
            }
        }
        self.flush_liveness(now_ms)?;
        let tick_events = self.authority.advance_to(now_ms)?;
        // Retire của nhánh `submit` xảy ra khi xử lý frame ở trên, trước tick; trả cùng lượt
        // gọi này để caller thấy retire trước khi đo `applied_seq`.
        let mut events = std::mem::take(&mut self.pending_input_retired);
        events.extend(tick_events);
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
        let authenticated = self
            .sessions
            .get(&player)
            .is_some_and(|session| session.account.is_some());
        if kind == boom_proto::MessageKind::Hello {
            // Trước AuthOk: Hello lặp được trả lại Welcome (dùng cho test mạng lossy); không
            // đổi account/RoomGate. Sau AuthOk: Addendum A không nhận Hello → lỗi giao thức.
            if authenticated {
                self.send(
                    player,
                    &Reject {
                        reason: RejectReason::SessionRejected,
                        detail: "Hello sau AuthOk là lỗi giao thức".to_owned(),
                    },
                )?;
                self.close_session_with_reason(
                    player,
                    RevokeReason::Disconnect,
                    DisconnectReason::ProtocolError,
                );
                return Ok(());
            }
            return self.handle_hello(player, frame);
        }
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
            self.close_session_with_reason(
                player,
                RevokeReason::Disconnect,
                DisconnectReason::ProtocolError,
            );
            return Ok(());
        }
        if kind == boom_proto::MessageKind::Logout {
            let _: Logout = decode_frame(frame, &self.authority.config().protocol_limits)?;
            self.close_session_after_flush_with_reason(
                player,
                RevokeReason::Logout,
                DisconnectReason::ClientClosed,
            );
            return Ok(());
        }
        // Đã `AuthOk` ⇒ định danh người chơi trong phòng/round là `account_id`
        // (ADR 0009 Addendum B §B.2). Mọi lệnh phòng/round bind theo định danh này.
        let Some(room_player) = self.room_player(player) else {
            self.close_session_with_reason(
                player,
                RevokeReason::Disconnect,
                DisconnectReason::ProtocolError,
            );
            return Ok(());
        };
        let limits = self.authority.config().protocol_limits;
        match kind {
            boom_proto::MessageKind::InputBatch => {
                let batch: InputBatch = decode_frame(frame, &limits)?;
                // Server bind slot của input từ phiên: định danh người chơi = `account_id`,
                // slot lấy từ roster round (ADR 0009 Addendum B §B.2). `batch.player` do
                // client khai chỉ để **đối chiếu**, không phải nguồn sự thật.
                match self
                    .authority
                    .submit_input(room_player, &batch, self.now_ms)
                {
                    Ok(ack) => self.queue_input_retired(room_player, ack.retired),
                    Err(crate::InputError::PlayerMismatch { expected, received }) => {
                        // Client khai slot khác slot server bind ⇒ **bỏ** batch + đếm,
                        // không áp dụng và không đổi state (không tin client khai — ADR 0005).
                        self.input_slot_mismatch = self.input_slot_mismatch.saturating_add(1);
                        self.log_input_slot_mismatch(room_player, expected, received);
                    }
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
                // Bind định danh người chơi từ phiên (`account_id`) rồi mới dựng lệnh phòng:
                // client không được khai định danh (ADR 0009 Addendum B §B.2).
                let command = from_room_command(&dto, room_player);
                // Ảnh phòng **trước** khi áp lệnh: người bị kick bị gỡ khỏi roster ngay
                // khi lệnh áp dụng ⇒ sau đó không tra lại được ngữ cảnh phòng/round.
                let before = self.authority.room_view();
                match self.authority.room_command(command, self.now_ms) {
                    Ok(update) => self.after_room_update(&update, before)?,
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
                self.handle_room_chat(player, room_player, &chat)?;
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
                    match self.authority.ack_loading(room_player, self.now_ms) {
                        Ok(events) => self.flush_lifecycle(&events)?,
                        Err(_) => self.send_room_rejected(
                            player,
                            ack.round_id,
                            boom_proto::RoomRejectReason::NotWaiting,
                        )?,
                    }
                }
            }
            boom_proto::MessageKind::RoundResultRequest => {
                let request: boom_proto::RoundResultRequest = decode_frame(frame, &limits)?;
                let result = self
                    .committed_result
                    .as_ref()
                    .filter(|(room, result)| {
                        self.authority
                            .room_view()
                            .is_some_and(|view| view.id.0 == *room)
                            && result.round_id == request.round_id
                            && result
                                .results
                                .iter()
                                .any(|entry| entry.player == room_player.0)
                    })
                    .map(|(_, result)| result.clone());
                if let Some(result) = result {
                    self.send(player, &result)?;
                } else {
                    self.send_room_rejected(
                        player,
                        request.round_id,
                        boom_proto::RoomRejectReason::StaleRequest,
                    )?;
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
                    match self.authority.ack_result(room_player, self.now_ms) {
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
                // Kind lạ sau `AuthOk` là lỗi giao thức (ADR 0009 Addendum A).
                self.close_session_with_reason(
                    player,
                    RevokeReason::Disconnect,
                    DisconnectReason::ProtocolError,
                );
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
        let source = session.source_ip.to_string();
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
                        account_id,
                        display_name,
                        session_id,
                        last_pong_ms: self.now_ms,
                        next_ping_ms: self.now_ms.saturating_add(self.ping_interval_ms),
                    });
                }
                // Định danh người chơi trong phòng = `account_id`; mọi lệnh phòng/round bind
                // theo ánh xạ này (ADR 0009 Addendum B §B.2).
                self.by_account.insert(PlayerId(account_id), player);
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

    /// Xử lý kết quả lệnh phòng: gửi trạng thái phòng cho roster, ghi log vòng đời round
    /// và log `player.disconnect` cho người bị kick.
    ///
    /// `before` là ảnh phòng **trước** khi lệnh áp dụng: người bị kick đã bị gỡ khỏi
    /// roster ⇒ không tra lại được ngữ cảnh phòng/round của họ sau khi lệnh áp dụng.
    fn after_room_update(
        &mut self,
        update: &RoomUpdate,
        before: Option<RoomView>,
    ) -> Result<(), ServerError> {
        if let Some(view) = self.authority.room_view() {
            self.send_room_state(&view)?;
        }
        self.broadcast_lobby_list()?;
        // Người bị kick: ghi lý do `kicked` (V1: kick chỉ rời phòng, kết nối TCP còn sống).
        for event in &update.events {
            if let RoomEvent::PlayerKicked { player, by, .. } = event {
                let context = room_context(before.as_ref(), *player);
                self.log_kicked(*player, *by, context);
            }
        }
        if self.authority.room_view().is_none() {
            self.committed_result = None;
        }
        if let Some(round) = update.round {
            self.committed_result = None;
            // Round mới: ghi vòng đời round trước khi gửi `RoundStart` cho roster.
            self.log_round_start(round);
            self.send_round_start(round)?;
            return Ok(());
        }
        // Start lặp: không tạo round mới, nhưng client chưa nhận `RoundStart` thì gửi lại.
        let repeated_round = if update.duplicate && self.authority.phase() == ServerPhase::Loading {
            self.authority
                .room_view()
                .and_then(|view| view.round)
                .map(|round| round.id)
        } else {
            None
        };
        if let Some(round) = repeated_round {
            self.send_round_start(round)?;
        }
        Ok(())
    }

    fn send_room_state(&mut self, view: &RoomView) -> Result<(), ServerError> {
        let mut state = to_room_state(view);
        // Tên hiển thị: **server** điền từ account store sau adapter (adapter không suy diễn
        // tên — ADR 0009 Addendum B §B.6).
        for member in &mut state.members {
            member.name = self.display_name_of(PlayerId(member.player));
        }
        if let Some(round) = state.round.as_mut() {
            for member in &mut round.members {
                member.name = self.display_name_of(PlayerId(member.player));
            }
        }
        // `view.members` mang **định danh người chơi** (`account_id`); gửi về kết nối đang giữ nó.
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for account in members {
            let Some(connection) = self.connection_of(account) else {
                continue;
            };
            if self
                .sessions
                .get(&connection)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(connection, &state)?;
            }
        }
        Ok(())
    }

    fn lobby_list(&self) -> LobbyList {
        let views: Vec<RoomView> = self.authority.room_view().into_iter().collect();
        let mut list = to_lobby_list(&views);
        for room in &mut list.rooms {
            // `LobbyRoom.name` là **tên hiển thị của chủ phòng** (ADR 0009 Addendum B §B.6):
            // `host` là định danh người chơi (`account_id`) ⇒ tên lấy từ account store.
            room.name = self.display_name_of(PlayerId(room.host));
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
        let Some(mut roster) = to_room_state(&view).round else {
            return Ok(());
        };
        // Tên hiển thị trong roster: server điền từ account store (ADR 0009 Addendum B §B.6).
        // Roster là bản **lịch sử**: người đã rời phòng sau `Start` vẫn có tên.
        for member in &mut roster.members {
            member.name = self.display_name_of(PlayerId(member.player));
        }
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
        for account in members {
            let Some(connection) = self.connection_of(account) else {
                continue;
            };
            if self
                .sessions
                .get(&connection)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(connection, &message)?;
            }
        }
        Ok(())
    }

    fn handle_room_chat(
        &mut self,
        connection: PlayerId,
        room_player: PlayerId,
        chat: &RoomChat,
    ) -> Result<(), ServerError> {
        let Some(view) = self.authority.room_view() else {
            return self.send_room_rejected(
                connection,
                chat.request,
                boom_proto::RoomRejectReason::PlayerNotInRoom,
            );
        };
        if !view.members.iter().any(|m| m.player == room_player) {
            return self.send_room_rejected(
                connection,
                chat.request,
                boom_proto::RoomRejectReason::PlayerNotInRoom,
            );
        }
        let max_per_s = self.authority.config().protocol_limits.max_chat_per_s;
        let now = self.now_ms;
        let Some(session) = self.sessions.get_mut(&connection) else {
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
            .get(&connection)
            .and_then(|session| session.account.as_ref())
            .map(|account| account.display_name.clone())
            .unwrap_or_else(|| fallback_display_name(room_player));
        let message = RoomChatMessage {
            room_id: view.id.0,
            // `from` là định danh người chơi (`account_id`).
            from: room_player.0,
            from_name,
            text: chat.text.clone(),
        };
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for account in members {
            let Some(member_connection) = self.connection_of(account) else {
                continue;
            };
            if self
                .sessions
                .get(&member_connection)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(member_connection, &message)?;
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
        self.committed_result = Some((view.id.0, message.clone()));
        let members: Vec<PlayerId> = view.members.iter().map(|m| m.player).collect();
        for account in members {
            let Some(connection) = self.connection_of(account) else {
                continue;
            };
            if self
                .sessions
                .get(&connection)
                .is_some_and(|s| s.handshaken && s.auth.is_authenticated())
            {
                self.send(connection, &message)?;
            }
        }
        Ok(())
    }

    fn flush_lifecycle(&mut self, events: &[InternalServerEvent]) -> Result<(), ServerError> {
        for event in events {
            match event {
                InternalServerEvent::LoadingAcked { round, player, .. } => {
                    // Một người đã báo xong loading; vẫn ở pha loading.
                    let mut fields = LogFields::new();
                    fields.insert("player_id", serde_json::json!(player.0));
                    self.log_round_event(events::ROUND_LOADING, *round, phases::LOADING, fields);
                }
                InternalServerEvent::MatchStarted { round, .. } => {
                    // Loading barrier đóng ⇒ trận bắt đầu chạy.
                    self.log_round_event(
                        events::ROUND_PLAYING,
                        *round,
                        phases::PLAYING,
                        LogFields::new(),
                    );
                }
                InternalServerEvent::RoundFinished { result, .. } => {
                    // Kết quả round đã chốt.
                    let mut fields = LogFields::new();
                    fields.insert("outcome", serde_json::json!(outcome_label(&result.outcome)));
                    if let MatchOutcome::Winner(player) = result.outcome {
                        fields.insert("winner", serde_json::json!(player.0));
                    }
                    fields.insert("tick", serde_json::json!(result.tick));
                    self.log_round_event(
                        events::ROUND_RESULT,
                        result.round,
                        phases::RESULTS,
                        fields,
                    );
                    self.send_round_result(result)?;
                }
                InternalServerEvent::ReturnedToWaiting { round, .. } => {
                    // Round kết thúc bình thường: về phòng chờ.
                    let mut fields = LogFields::new();
                    fields.insert("outcome", serde_json::json!("completed"));
                    self.log_round_event(events::ROUND_END, *round, phases::WAITING, fields);
                    if let Some(view) = self.authority.room_view() {
                        self.send_room_state(&view)?;
                    }
                    self.broadcast_lobby_list()?;
                }
                InternalServerEvent::RoundCancelled { round, .. } => {
                    // Round bị hủy (hết hạn loading, người chơi rời lúc loading).
                    if let Some(round) = round {
                        let mut fields = LogFields::new();
                        fields.insert("outcome", serde_json::json!("cancelled"));
                        self.log_round_event(events::ROUND_END, *round, phases::WAITING, fields);
                    }
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
                self.close_session_with_reason(
                    player,
                    RevokeReason::Disconnect,
                    DisconnectReason::ProtocolError,
                );
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
            // Định danh kết nối trong phiên — **không phải** slot trận (ADR 0009 Addendum B §B.2).
            player_id: session.connection_id,
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
        // Ack lấy theo **định danh người chơi** (`account_id`) của phiên.
        let applied_seq = self
            .room_player(player)
            .and_then(|room_player| self.authority.applied_seq(room_player))
            .unwrap_or(0);
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

    /// Ghi sự kiện "batch input bị bỏ vì client khai slot khác slot server bind".
    ///
    /// Không gửi `InputRejected`: lý do `BadSequence` khiến client bỏ lệnh của mình
    /// (ADR 0009 §3.4(d)) trong khi lệnh hoàn toàn hợp lệ — chỉ là client chưa biết
    /// slot của mình. Client tự resync qua snapshot.
    fn log_input_slot_mismatch(&self, room_player: PlayerId, expected: u8, received: u8) {
        let Some(logger) = self.logger.as_ref() else {
            return;
        };
        let mut fields = LogFields::new();
        fields.insert("player_id", serde_json::json!(room_player.0));
        fields.insert("expected_slot", serde_json::json!(expected));
        fields.insert("received_slot", serde_json::json!(received));
        fields.insert(
            "dropped_batches",
            serde_json::json!(self.input_slot_mismatch),
        );
        logger.warn(events::INPUT_SLOT_MISMATCH, fields);
    }

    /// Đưa các sequence `InputAck.retired` của nhánh `submit` vào event trả cho caller `poll`.
    ///
    /// Retire này đã xảy ra trong `InputTrack::submit` (ADR 0009 §3.3 bước 5), trước mọi tick;
    /// event chỉ quan sát nên không đổi state/hash/applied_seq. `submit_input` chỉ chạy ở pha
    /// `Playing` nên room và round public luôn có.
    fn queue_input_retired(&mut self, player: PlayerId, retired: Vec<u32>) {
        if retired.is_empty() {
            return;
        }
        let room = self.authority.room_id().expect("pha Playing luôn có phòng");
        let round = self
            .authority
            .current_round_id()
            .map(|round| RoundId(u64::from(round)))
            .expect("pha Playing luôn có round public");
        self.pending_input_retired
            .extend(
                retired
                    .into_iter()
                    .map(|seq| InternalServerEvent::InputRetired {
                        room,
                        round,
                        player,
                        seq,
                    }),
            );
    }

    /// ID phòng **public** (giống `RoomState.room_id` mà client thấy).
    fn public_room_id(&self) -> Option<u64> {
        self.authority.room_view().map(|view| view.id.0)
    }

    /// Ghi sự kiện vòng đời round (ADR 0011 §8.1).
    ///
    /// Mọi sự kiện `round.*` mang `round_id` **thật** (ID public trên dây) để đối chiếu log
    /// với `RoundStart`/`RoundResult` mà client nhận được.
    fn log_round_event(&self, event: &str, round: RoundId, phase: &str, extra: LogFields) {
        let Some(logger) = self.logger.as_ref() else {
            return;
        };
        let mut fields = round_fields(round.0, self.public_room_id(), phase);
        for (key, value) in extra {
            fields.insert(key, value);
        }
        logger.info(event, fields);
    }

    /// Ghi sự kiện `player.disconnect` (ADR 0011 §8.1): lý do + định danh, không lộ bí mật.
    ///
    /// `reason` là mã lý do disconnect; `revoke_reason` là mã lý do **thu hồi phiên** trong DB
    /// (ADR 0011 §6.1) — hai trường khác nhau: bị kick thì `reason = kicked` nhưng phiên
    /// vẫn sống ⇒ `revoke_reason` để trống (null).
    ///
    /// `context` là ngữ cảnh phòng/round **của người đó** (xem [`room_context`]): chỉ
    /// truyền ngữ cảnh mà tài khoản thực sự thuộc roster, không gắn ID trận của người khác.
    fn log_player_disconnect(
        &self,
        account: Option<PlayerId>,
        reason: DisconnectReason,
        revoke: RevokeReason,
        session_id: Option<SessionId>,
        context: RoomContext,
    ) {
        let Some(logger) = self.logger.as_ref() else {
            return;
        };
        let mut fields = self.disconnect_identity_fields(account, session_id, context);
        fields.insert("reason", serde_json::json!(reason.as_str()));
        fields.insert("revoke_reason", serde_json::json!(revoke.as_str()));
        logger.info(events::PLAYER_DISCONNECT, fields);
    }

    /// Trường định danh chung cho sự kiện `player.disconnect` (ADR 0011 §8.1).
    fn disconnect_identity_fields(
        &self,
        account: Option<PlayerId>,
        session_id: Option<SessionId>,
        context: RoomContext,
    ) -> LogFields {
        let mut fields = LogFields::new();
        if let Some(account) = account {
            fields.insert(
                "account_id",
                serde_json::json!(i64::try_from(account.0).unwrap_or(i64::MAX)),
            );
            fields.insert("player_id", serde_json::json!(account.0));
        }
        if let Some(session_id) = session_id {
            fields.insert("session_id", serde_json::json!(session_id.0));
        }
        context.apply(&mut fields);
        fields
    }

    /// Ngữ cảnh phòng/round của tài khoản theo trạng thái authority **hiện tại**.
    ///
    /// Dùng cho các đường đóng session (EOF/reset, timeout, shutdown): tại thời điểm ghi
    /// log, tài khoản vẫn còn trong roster phòng ⇒ giữ đúng ngữ cảnh của thành viên thật;
    /// tài khoản ngoài phòng (đứng sảnh) hoặc connection chưa auth ⇒ không có ngữ cảnh.
    fn member_context(&self, account: Option<PlayerId>) -> RoomContext {
        let Some(account) = account else {
            return RoomContext::NONE;
        };
        room_context(self.authority.room_view().as_ref(), account)
    }

    /// Ghi `round.start`: round vừa được tạo, loading barrier mở.
    fn log_round_start(&self, round: RoundId) {
        let mut fields = LogFields::new();
        let players = self
            .authority
            .room_view()
            .map(|view| view.members.len())
            .unwrap_or(0);
        fields.insert("players", serde_json::json!(players));
        if let Some(seed) = self.authority.round_seed() {
            fields.insert("seed", serde_json::json!(seed));
        }
        self.log_round_event(events::ROUND_START, round, phases::LOADING, fields);
    }

    /// Ghi `player.disconnect` lý do `kicked` cho người bị host kick khỏi phòng.
    ///
    /// V1: kick chỉ **rời phòng** (kết nối TCP còn sống, phiên không bị thu hồi) — log ghi
    /// lý do `kicked` để vận hành biết vì sao người chơi rời phòng giữa trận.
    ///
    /// `context` là ngữ cảnh phòng/round lấy từ ảnh phòng **trước** khi lệnh kick áp dụng:
    /// người bị kick bị gỡ khỏi roster ngay lập tức ⇒ sau đó không tra lại được ngữ cảnh.
    fn log_kicked(&self, player: PlayerId, by: PlayerId, context: RoomContext) {
        let Some(logger) = self.logger.as_ref() else {
            return;
        };
        let mut fields = self.disconnect_identity_fields(Some(player), None, context);
        fields.insert("kicked_by", serde_json::json!(by.0));
        fields.insert(
            "reason",
            serde_json::json!(DisconnectReason::Kicked.as_str()),
        );
        logger.info(events::PLAYER_DISCONNECT, fields);
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

    /// Gửi snapshot của lượt cho một kết nối.
    ///
    /// ADR 0009 §4.3: server chỉ gửi snapshot **sau khi round bắt đầu** — tức sau loading
    /// barrier (SCOPE_V1 §3). Lúc phòng còn ở loading, state trận chưa chạy (`tick == 0`)
    /// nên snapshot chỉ mô tả phòng đang loading: không gửi. Nhịp gửi định kỳ không bị
    /// dịch vì vậy: `next_snapshot_ms` chỉ đổi khi snapshot thật sự đi.
    fn send_snapshot(&mut self, player: PlayerId) -> Result<(), ServerError> {
        if !self.authority.snapshot_allowed() {
            return Ok(());
        }
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

    /// Kết nối này có đang là thành viên phòng không (định danh = `account_id`).
    fn is_member(&self, connection: PlayerId) -> bool {
        let Some(account) = self.room_player(connection) else {
            return false;
        };
        self.authority
            .room_view()
            .is_some_and(|view| view.members.iter().any(|m| m.player == account))
    }

    fn flush_liveness(&mut self, now_ms: u64) -> Result<(), ServerError> {
        let deadline = self.auth_deadline_ms.max(1);
        let auth_expired: Vec<PlayerId> = self
            .sessions
            .iter()
            .filter(|(_, session)| {
                session.account.is_none()
                    && now_ms.saturating_sub(session.connected_at_ms) >= deadline
            })
            .map(|(player, _)| *player)
            .collect();
        for player in auth_expired {
            // Không hoàn tất handshake/auth trong hạn ⇒ lý do `auth_failed`.
            self.close_session_with_reason(
                player,
                RevokeReason::Disconnect,
                DisconnectReason::AuthFailed,
            );
        }
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
            // Im lặng quá hạn liveness (không có `Pong`) ⇒ `timeout_no_input`.
            self.close_session_with_reason(
                player,
                RevokeReason::TimeoutNoInput,
                DisconnectReason::TimeoutNoInput,
            );
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

    /// Đóng session vì client biến mất (EOF/RST) hoặc vì writer không còn đọc được.
    fn close_session(&mut self, player: PlayerId) {
        self.close_session_with_reason(
            player,
            RevokeReason::Disconnect,
            DisconnectReason::ClientClosed,
        );
    }

    /// Đóng session, thu hồi phiên và ghi `player.disconnect` với lý do.
    fn close_session_with_reason(
        &mut self,
        player: PlayerId,
        revoke: RevokeReason,
        reason: DisconnectReason,
    ) {
        let Some(session) = self.sessions.remove(&player) else {
            return;
        };
        let account = session
            .account
            .as_ref()
            .map(|account| PlayerId(account.account_id));
        let session_id = session.account.as_ref().map(|account| account.session_id);
        self.revoke_account(&session, revoke);
        self.forget_account(account);
        let context = self.member_context(account);
        self.log_player_disconnect(account, reason, revoke, session_id, context);
        self.notify_disconnect(account);
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

    /// Gỡ ánh xạ account → kết nối khi phiên đóng.
    fn forget_account(&mut self, account: Option<PlayerId>) {
        if let Some(account) = account {
            self.by_account.remove(&account);
        }
    }

    /// Báo cho authority biết người chơi rời phòng (định danh = `account_id`).
    fn notify_disconnect(&mut self, account: Option<PlayerId>) {
        if let Some(account) = account
            && let Ok(events) = self.authority.disconnect(account, self.now_ms)
        {
            let _ = self.flush_lifecycle(&events);
        }
        if let Some(view) = self.authority.room_view() {
            let _ = self.send_room_state(&view);
        }
        if self.authority.room_view().is_none() {
            self.committed_result = None;
        }
        let _ = self.broadcast_lobby_list();
    }

    /// Đóng session nhưng để writer gửi nốt frame đã xếp (vd. `Reject` handshake, ADR 0009 §1).
    /// Bỏ `out_tx` làm writer thoát sau khi rút hết queue; một luồng canh gọi shutdown khi writer
    /// xong hoặc quá `GRACEFUL_CLOSE_MS`, nên writer kẹt vẫn bị thu hồi và tick không phải chờ.
    fn close_session_after_flush(&mut self, player: PlayerId) {
        self.close_session_after_flush_with_reason(
            player,
            RevokeReason::Disconnect,
            DisconnectReason::ProtocolError,
        );
    }

    fn close_session_after_flush_with_reason(
        &mut self,
        player: PlayerId,
        revoke: RevokeReason,
        reason: DisconnectReason,
    ) {
        let Some(mut session) = self.sessions.remove(&player) else {
            return;
        };
        let account = session
            .account
            .as_ref()
            .map(|account| PlayerId(account.account_id));
        let session_id = session.account.as_ref().map(|account| account.session_id);
        self.revoke_account(&session, revoke);
        self.forget_account(account);
        let context = self.member_context(account);
        self.log_player_disconnect(account, reason, revoke, session_id, context);
        self.notify_disconnect(account);
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
    //
    // `player` ở các hàm này là **định danh kết nối** (khoá session): server bind định danh
    // người chơi trong phòng (`account_id`) từ phiên đó, đúng như lệnh phòng trên dây
    // (ADR 0009 Addendum B §B.2).

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
        let command = RoomCommand {
            player: self.bound_player(command.player),
            ..command
        };
        self.authority.room_command(command, now_ms)
    }
    pub fn ack_loading(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.authority
            .ack_loading(self.bound_player(player), now_ms)
    }

    pub fn ack_result(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.authority.ack_result(self.bound_player(player), now_ms)
    }

    pub fn disconnect(
        &mut self,
        player: PlayerId,
        now_ms: u64,
    ) -> Result<Vec<InternalServerEvent>, ServerError> {
        self.authority.disconnect(self.bound_player(player), now_ms)
    }

    pub fn player_life(&self, player: PlayerId) -> Option<Life> {
        self.authority.player_life(self.bound_player(player))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let now_ms = self.now_ms;
        self.shutdown_sessions(now_ms);
    }
}
