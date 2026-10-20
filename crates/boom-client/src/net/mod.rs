//! Module mạng của client V1 (ADR 0005, ADR 0009 §"Tác động tới prediction/reconciliation").
//!
//! Module này **không phụ thuộc macroquad**: nó chỉ nói chuyện với server qua trait
//! `Transport` của `boom-net`, nên test được headless (không cửa sổ, không render).
//!
//! Chia ba lớp:
//!
//! - [`session`]: đường truyền + codec — handshake `Hello`/`Welcome`/`Reject`, auth
//!   `Auth`/`AuthOk`/`AuthFail`, gửi `InputBatch`, nhận `Snapshot`/`RoundEvents`/`InputRejected`,
//!   dedup event theo `(round_id, EventId)`.
//! - [`prediction`]: state confirmed + state dự đoán của chính mình, replay lệnh chưa ack khi
//!   có snapshot (correction).
//! - [`room`]: sảnh/phòng/chat/loading/result (ADR 0009 Addendum B) — lệnh phòng với request
//!   ID tăng dần, kiểm fingerprint của `RoundStart`, kết quả round do server quyết.
//!
//! [`NetClient`] ghép ba lớp trên cho vòng lặp của client: bơm message, dự đoán một bước và
//! gửi cửa sổ lệnh chưa ack.

mod prediction;
mod room;
mod session;

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;

pub use prediction::{
    ALPHA_ONE, BombShadow, PredictError, Predictor, SnapshotOutcome, world_from_snapshot,
};
pub use room::{
    AutoAction, AutoRoomFlow, LocalContent, RoomError, RoomEvent, RoomFlow, RoundExpectation,
    RoundPlan, world_for_round,
};
pub use session::{ClientMessage, ClientSession, NetError};

use std::fmt;
use std::time::{Duration, Instant};

use boom_content::rules::RulesProfile;
use boom_core::{Mode, Phase, PlayerInput, Rules, SimulationState, World, mode_by_id};
use boom_net::{TcpTransport, Transport};
use boom_proto::dto::auth::{Auth, AuthOk, AuthOp};
use boom_proto::{
    Hello, InputBatch, LoadingAck, ResultAck, RoomAction, RoomCharacter, RoundEvent, RoundStart,
    Welcome,
};

/// Thông tin đăng nhập client dùng **sau** `Welcome` (ADR 0009 Addendum A §A.1).
///
/// Mật khẩu đi thô trên dây: V1 chưa có TLS (ADR 0006) nên chỉ dùng cho LAN/máy local — đây là
/// giới hạn đã biết của ADR 0011 §6/§14.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    /// Tên đăng nhập.
    pub username: String,
    /// Mật khẩu thô.
    pub password: String,
    /// Đăng ký tài khoản mới hay đăng nhập tài khoản có sẵn.
    pub op: AuthOp,
}

impl Credentials {
    /// Đăng nhập tài khoản có sẵn.
    pub fn login(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            op: AuthOp::Login,
        }
    }

    /// Đăng ký tài khoản mới.
    pub fn register(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            op: AuthOp::Register,
        }
    }
}

/// `Debug` **không** in mật khẩu: mật khẩu không bao giờ vào log (ADR 0011 §5, §8.1).
impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("op", &self.op)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Lỗi dự đoán/reconciliation báo ra ngoài: client hiện lỗi, không âm thầm bỏ qua state.
impl From<PredictError> for NetError {
    fn from(value: PredictError) -> Self {
        Self::Unexpected(format!("dự đoán: {value}"))
    }
}

/// Khoảng cách tối thiểu giữa hai lần gửi lại lệnh phòng đang chờ phản hồi.
///
/// **Tham số vận hành** (design), không phải luật chơi: mạng có thể mất frame, client gửi lại
/// lệnh phòng (cùng request ID → server dedup) để không kẹt ở sảnh/phòng.
const ROOM_RETRY_INTERVAL: Duration = Duration::from_millis(500);

/// Bước một tick của client: bơm message, dự đoán, gửi cửa sổ lệnh.
pub struct NetClient<W: Transport> {
    session: ClientSession<W>,
    rules: Rules,
    mode: &'static dyn Mode,
    predictor: Option<Predictor>,
    /// Slot người chơi do server cấp trong `Welcome`.
    player_slot: Option<u8>,
    /// `Welcome` nhận được (giữ `server_version`/`content_sha256` của server, ADR 0011 §8.3).
    welcome: Option<Welcome>,
    /// Kết quả `AuthOk`: tên hiển thị và hạn token (token không log, ADR 0011 §6).
    account: Option<AuthOk>,
    /// `snapshot_seq` lớn nhất đã **áp**; chỉ cập nhật khi snapshot được áp, không lùi theo
    /// snapshot cũ (ADR 0009 §4.3).
    last_snapshot_seq: u32,
    /// Chu kỳ gửi snapshot (ms) do server khai trong `Welcome`; presentation dùng để tính
    /// alpha nội suy.
    snapshot_interval_ms: u64,
    /// Event mới (đã dedup), chờ presentation journal lấy.
    events: Vec<RoundEvent>,
    /// Trạng thái luồng phòng: sảnh/phòng/chat/loading/result (ADR 0009 Addendum B).
    room: RoomFlow,
    /// Nội dung cục bộ (map + địa hình) để dựng `World` cho round.
    local_content: Option<LocalContent>,
    /// Luồng tự động (`--auto-ready`): tự vào phòng, ready, start và ack.
    auto: Option<AutoRoomFlow>,
    /// Hash content client đã khai ở handshake (dùng để kiểm `RoundStart`).
    content_sha256: [u8; 32],
    /// Tên profile rules client đã khai ở handshake (dùng để kiểm `RoundStart`).
    profile_id: String,
    /// `World` dựng từ nội dung cục bộ cho round hiện tại.
    ///
    /// `None` = client chưa có content của map đó: `World` dựng từ snapshot (server là
    /// authority về lưới ô) — trạng thái này **không** được coi là đã kiểm content cục bộ.
    round_world: Option<World>,
    /// Lệnh phòng gần nhất của luồng tự động: `(request ID, hành động)`.
    ///
    /// Dùng để gửi lại **cùng** lệnh khi chưa thấy phản hồi (mạng mất frame).
    last_auto: Option<(u64, AutoAction)>,
    /// Mốc gửi lần gần nhất của lệnh phòng trong luồng tự động.
    last_auto_send: Instant,
}

impl<W: Transport> NetClient<W> {
    /// Dựng client mạng trên một transport hai chiều.
    pub fn new(session: ClientSession<W>, rules: Rules, mode: &'static dyn Mode) -> Self {
        Self {
            session,
            rules,
            mode,
            predictor: None,
            player_slot: None,
            welcome: None,
            account: None,
            last_snapshot_seq: 0,
            snapshot_interval_ms: 0,
            events: Vec::new(),
            room: RoomFlow::new(0),
            local_content: None,
            auto: None,
            content_sha256: [0; 32],
            profile_id: String::new(),
            round_world: None,
            last_auto: None,
            last_auto_send: Instant::now(),
        }
    }

    /// Handshake với server.
    pub fn handshake(&mut self, hello: &Hello) -> Result<Welcome, NetError> {
        let welcome = self.session.handshake(hello)?;
        self.player_slot = Some(welcome.player_id);
        self.snapshot_interval_ms = u64::from(welcome.snapshot_interval_ms.max(1));
        self.welcome = Some(welcome.clone());
        // Fingerprint client dùng để kiểm `RoundStart`: đúng những gì client khai ở handshake.
        self.content_sha256 = hello.content_sha256;
        self.profile_id = hello.rules_profile_id.clone();
        // Slot của client do server cấp: cùng id người chơi trong state trận.
        self.room.set_my_slot(welcome.player_id);
        Ok(welcome)
    }

    /// Đăng nhập/đăng ký **sau** `Welcome` (ADR 0009 Addendum A §A.1).
    ///
    /// Server chặn mọi kind khác trước `AuthOk`, nên client phải làm bước này trước khi gửi
    /// input. `AuthFail` được trả về lỗi rõ ràng (không đóng kết nối) để caller thử lại.
    pub fn authenticate(&mut self, credentials: &Credentials) -> Result<AuthOk, NetError> {
        let auth = Auth {
            op: credentials.op,
            username: credentials.username.clone(),
            password: credentials.password.clone(),
        };
        let ok = self.session.authenticate(&auth)?;
        self.account = Some(ok.clone());
        Ok(ok)
    }

    /// Tài khoản đã đăng nhập (sau `AuthOk`).
    pub fn account(&self) -> Option<&AuthOk> {
        self.account.as_ref()
    }

    /// Build id của server (fingerprint ADR 0011 §8.3), lấy từ `Welcome`.
    pub fn server_version(&self) -> Option<&str> {
        self.welcome
            .as_ref()
            .map(|welcome| welcome.server_version.as_str())
    }

    /// Bơm message đang chờ: đọc tới khi hàng đợi rỗng.
    ///
    /// Snapshot mới sẽ áp vào predictor (correction); `InputRejected` sẽ bỏ lệnh đã giải quyết
    /// và giữ nguyên payload các lệnh còn lại để resend; event mới được dedup rồi chờ
    /// presentation.
    pub fn pump(&mut self, timeout: Duration) -> Result<(), NetError> {
        let deadline = std::time::Instant::now() + timeout;
        let mut first = true;
        loop {
            let wait = if first {
                deadline.saturating_duration_since(std::time::Instant::now())
            } else {
                Duration::ZERO
            };
            match self.session.next_message(wait)? {
                Some(message) => {
                    self.handle(message)?;
                    first = false;
                }
                None => break,
            }
        }
        // Luồng tự động: lệnh phòng chưa được server xác nhận thì gửi lại (cùng request ID).
        self.auto_retry()?;
        Ok(())
    }

    /// Dự đoán một bước của mình: sinh lệnh mới (chưa gửi).
    ///
    /// Chưa có snapshot nào thì chưa dự đoán được: lệnh chỉ được tạo khi đã biết state
    /// confirmed (ADR 0005). Lượt đã kết thúc thì **không** sinh lệnh: kết quả do server quyết,
    /// client giữ state confirmed và vẫn nhận message.
    ///
    /// Lệnh được gửi đi bằng [`NetClient::resend`].
    pub fn predict(&mut self, input: PlayerInput) -> Result<(), NetError> {
        let Some(predictor) = self.predictor.as_mut() else {
            return Ok(());
        };
        if !predictor.is_playing() {
            return Ok(());
        }
        predictor.predict(input)?;
        Ok(())
    }

    /// Gửi cửa sổ lệnh chưa ack (bắt đầu từ lệnh chưa ack nhỏ nhất, ADR 0009 §3.2).
    ///
    /// ADR 0009 §3.2: "client gửi lại cùng cửa sổ cho tới khi ack vượt qua lệnh đó; không có
    /// cơ chế gửi một lần rồi thôi". Lệnh đã gửi mà chưa ack vẫn phải được gửi lại **y nguyên
    /// payload** (không đổi `target_tick`) để server nhận được sau khi frame đầu bị mất.
    pub fn resend(&mut self) -> Result<(), NetError> {
        self.send_window()
    }

    /// State dự đoán hiện tại (để render).
    pub fn state(&self) -> Option<&SimulationState> {
        self.predictor.as_ref().and_then(Predictor::predicted_state)
    }

    /// State confirmed (snapshot mới nhất của server).
    pub fn confirmed_state(&self) -> Option<&SimulationState> {
        self.predictor.as_ref().and_then(Predictor::confirmed_state)
    }

    /// Pha lượt theo snapshot mới nhất.
    pub fn phase(&self) -> Option<Phase> {
        self.predictor.as_ref().and_then(Predictor::phase)
    }

    /// Bóng bom chờ xác nhận (presentation; bom thật theo event/snapshot của server).
    pub fn bomb_shadows(&self) -> &[BombShadow] {
        self.predictor.as_ref().map_or(&[], Predictor::bomb_shadows)
    }

    /// Vị trí nội suy của người chơi `id` giữa hai snapshot gần nhất.
    ///
    /// `alpha` tính bằng [`ALPHA_ONE`] là 1.0; lớn hơn được clamp ở snapshot cuối (không ngoại
    /// suy, ADR 0005). Chỉ là **dữ liệu presentation**: state authority vẫn là snapshot.
    pub fn interpolated_position(&self, id: u8, alpha: u32) -> Option<(i32, i32)> {
        self.predictor
            .as_ref()
            .and_then(|predictor| predictor.interpolated_position(id, alpha))
    }

    /// Chu kỳ gửi snapshot (ms) do server khai trong `Welcome`.
    pub fn snapshot_interval_ms(&self) -> u64 {
        self.snapshot_interval_ms
    }

    /// Số lệnh input chưa được server ack (cửa sổ đang chờ resend).
    pub fn pending_len(&self) -> usize {
        self.predictor
            .as_ref()
            .map_or(0, |predictor| predictor.pending().len())
    }

    /// Lấy event mới (đã dedup theo `(round_id, EventId)`).
    pub fn take_events(&mut self) -> Vec<RoundEvent> {
        std::mem::take(&mut self.events)
    }

    /// Trạng thái luồng phòng (sảnh/phòng/chat/loading/result).
    pub fn room(&self) -> &RoomFlow {
        &self.room
    }

    /// Kế hoạch round hiện tại (đã kiểm fingerprint), nếu có.
    ///
    /// Đây là dữ liệu client dùng được: seed/map/mode/rules/content của round mà server đã
    /// chốt. Client không tự sinh kế hoạch này (server là authority).
    pub fn round_plan(&self) -> Option<&RoundPlan> {
        self.room.round()
    }

    /// Lỗi kiểm fingerprint gần nhất của luồng phòng (nếu có).
    ///
    /// Lỗi này là "client không chơi được trận này" (lệch rules/content, thiếu map), **không**
    /// phải lỗi giao thức: client vẫn bơm message, nhưng không ack loading nên loading barrier
    /// của server hủy start thay vì chơi sai nội dung.
    pub fn room_error(&self) -> Option<&RoomError> {
        self.room.error()
    }

    /// Nội dung cục bộ để dựng `World` cho round (map + địa hình).
    pub fn set_local_content(&mut self, content: LocalContent) {
        self.local_content = Some(content);
    }

    /// Bật luồng tự động (`--auto-ready`): client tự xin danh sách phòng, vào phòng, ready,
    /// start (nếu là host) và ack loading/kết quả.
    ///
    /// `min_players` là **tham số kịch bản QA**: số người tối thiểu trong phòng để host start.
    /// Server vẫn kiểm policy thật (`min_players_to_start`) nên client không tự quyết luật.
    ///
    /// Mọi bước là lệnh phòng **thật** trên dây: luồng không giả lập state phía client và không
    /// tự quyết kết quả (kết quả do server quyết trong `RoundResult`).
    pub fn enable_auto_flow(
        &mut self,
        character: RoomCharacter,
        min_players: usize,
    ) -> Result<(), NetError> {
        self.auto = Some(AutoRoomFlow::with_min_players(character, min_players));
        self.request_lobby()
    }

    /// Gửi lệnh phòng với request ID **tăng dần**; trả request ID đã dùng.
    ///
    /// `RoomCommand` không mang player: server bind player từ phiên đã `AuthOk`
    /// (ADR 0005, ADR 0009 Addendum B §B.2).
    pub fn room_command(&mut self, action: RoomAction) -> Result<u64, NetError> {
        let command = self.room.command(action);
        let request = command.request;
        self.session.send_room_command(&command)?;
        Ok(request)
    }

    /// Gửi lại lệnh phòng với **cùng** request ID (mạng mất frame chưa thấy phản hồi).
    ///
    /// Server dedup theo request ID nên lệnh không bị áp hai lần (ADR 0009 Addendum B §B.2).
    pub fn resend_room_command(&mut self, request: u64, action: RoomAction) -> Result<(), NetError> {
        let command = self.room.command_with_request(request, action);
        self.session.send_room_command(&command)
    }

    /// Xin danh sách phòng sảnh.
    pub fn request_lobby(&mut self) -> Result<(), NetError> {
        self.session.send_lobby_list_request()
    }

    /// Gửi chat phòng; trả request ID đã dùng.
    ///
    /// Chat chỉ chuyển text, không chạy slash command (ADR 0009 Addendum B §B.2).
    ///
    /// `#[allow(dead_code)]`: API gửi chat cho tầng UI (#103 chưa merge nên chưa có caller
    /// trong client này); test end-to-end dùng để kiểm chat chỉ tới thành viên phòng.
    #[allow(dead_code)]
    pub fn send_chat(&mut self, text: &str) -> Result<u64, NetError> {
        let chat = self.room.chat_message(text);
        let request = chat.request;
        self.session.send_room_chat(&chat)?;
        Ok(request)
    }

    /// Round hiện tại (từ snapshot gần nhất đã áp).
    pub fn round_id(&self) -> Option<u32> {
        self.current_round()
    }

    /// Slot người chơi do server cấp.
    pub fn player_slot(&self) -> Option<u8> {
        self.player_slot
    }

    /// Round hiện tại: lấy từ predictor (nguồn duy nhất của state đã áp).
    fn current_round(&self) -> Option<u32> {
        self.predictor.as_ref().and_then(Predictor::round_id)
    }

    fn handle(&mut self, message: ClientMessage) -> Result<(), NetError> {
        match message {
            ClientMessage::Snapshot(snapshot) => self.apply_snapshot(snapshot)?,
            ClientMessage::RoundEvents(events) => {
                // Event của round cũ tới trễ sau khi đã sang round mới thì bỏ: presentation
                // chỉ nhận event của round đang chơi (dedup theo `(round_id, EventId)`).
                if self
                    .current_round()
                    .is_none_or(|round| events.round_id >= round)
                {
                    let fresh = self.session.accept_events(events);
                    self.events.extend(fresh);
                }
            }
            ClientMessage::InputRejected(rejected) => {
                // Reject là tín hiệu resync sequence, không phải "hủy cả cửa sổ" (ADR 0009
                // §3.2): bỏ lệnh đã giải quyết, giữ nguyên payload lệnh còn lại để resend, và
                // xin snapshot để chắc state (ADR 0009 §3.4(d)).
                //
                // Chỉ reject **cùng round** mới áp dụng: reject của round cũ không được xóa
                // lệnh của round mới.
                if self.current_round() != Some(rejected.round_id) {
                    return Ok(());
                }
                if let Some(predictor) = self.predictor.as_mut() {
                    predictor.acknowledge(rejected.applied_seq);
                }
                self.session
                    .request_snapshot(rejected.round_id, self.last_snapshot_seq)?;
            }
            ClientMessage::Ping(ping) => {
                // Server đóng phiên đã `AuthOk` nếu quá hạn không nhận `Pong` (ADR 0011 §6.1)
                // nên phải trả lời ngay.
                self.session.send_pong(ping)?;
            }
            ClientMessage::Reject(reject) => return Err(NetError::Rejected(reject)),
            ClientMessage::AuthOk(_) => {
                // `AuthOk` lặp là frame **trùng trên đường** (mạng có thể nhân bản frame):
                // phiên đã xác lập nên bỏ qua, không coi là lỗi giao thức.
            }
            ClientMessage::AuthFail(fail) => {
                // `AuthFail` tới sau khi đã `AuthOk` là mâu thuẫn với phiên đang chạy.
                return Err(NetError::Unexpected(format!(
                    "AuthFail tới sau AuthOk: {:?}",
                    fail.reason
                )));
            }
            // Nhóm phòng/sảnh (ADR 0009 Addendum B): client nhận và giữ trạng thái, không đoán.
            ClientMessage::RoomState(state) => {
                self.room.on_room_state(state);
                self.drive_auto(RoomEvent::RoomState)?;
            }
            ClientMessage::LobbyList(list) => {
                self.room.on_lobby_list(list);
                self.drive_auto(RoomEvent::LobbyList)?;
            }
            ClientMessage::RoomChatMessage(message) => {
                self.room.on_chat(message);
            }
            ClientMessage::RoundStart(start) => {
                self.handle_round_start(start)?;
            }
            ClientMessage::RoundResult(result) => {
                // Kết quả do server quyết (ADR 0005): client giữ lại rồi ack để về phòng.
                let round_id = result.round_id;
                self.room.on_round_result(result);
                self.session.send_result_ack(&ResultAck { round_id })?;
                self.room.mark_result_acked(round_id);
                self.drive_auto(RoomEvent::RoundResult)?;
            }
            ClientMessage::RoomRejected(rejected) => {
                self.room.on_rejected(rejected);
                self.drive_auto(RoomEvent::Rejected)?;
            }
            ClientMessage::UnknownKind(code) => {
                let _ = self.session.close();
                return Err(NetError::Unexpected(format!(
                    "server gửi kind lạ {code:#06x}; đóng kết nối"
                )));
            }
            ClientMessage::Welcome(_) | ClientMessage::Pong(_) => {}
        }
        Ok(())
    }

    /// `RoundStart`: kiểm fingerprint, dựng world cho round rồi ack loading.
    ///
    /// Fingerprint lệch ⇒ client **không** ack loading: loading barrier của server hủy start
    /// (SCOPE_V1 §3) thay vì client chơi sai rules/content. Lỗi được lưu lại để caller hiện
    /// ra ngoài (xem [`NetClient::room_error`]).
    fn handle_round_start(&mut self, start: RoundStart) -> Result<(), NetError> {
        let expectation = RoundExpectation {
            profile_id: self.profile_id.clone(),
            rules_hash: self.rules.profile_hash,
            content_sha256: self.content_sha256,
            mode: self.mode.id().to_owned(),
        };
        let plan = match self.room.on_round_start(start, &expectation) {
            Ok(plan) => plan,
            Err(_) => {
                // Lỗi đã lưu trong luồng phòng: không ack loading.
                return Ok(());
            }
        };
        // Dựng `World` cho round: ưu tiên nội dung cục bộ của client, không có thì để
        // `World` dựng từ snapshot (server là authority về lưới ô).
        match world_for_round(
            &plan,
            self.local_content.as_ref(),
            &self.rules,
            self.mode,
        ) {
            Ok(world) => self.round_world = world,
            Err(error) => {
                self.room.set_error(error);
                return Ok(());
            }
        }
        self.session
            .send_loading_ack(&LoadingAck { round_id: plan.round_id })?;
        self.room.mark_loading_acked(plan.round_id);
        self.drive_auto(RoomEvent::RoundStart)?;
        Ok(())
    }

    /// Chạy một bước của luồng tự động (`--auto-ready`) sau khi có message phòng.
    fn drive_auto(&mut self, event: RoomEvent) -> Result<(), NetError> {
        let action = match self.auto.as_mut() {
            Some(auto) => auto.on_event(event, &self.room),
            None => None,
        };
        let Some(action) = action else {
            return Ok(());
        };
        let request = self.execute_auto(action.clone())?;
        if request.is_some() {
            self.last_auto = request.map(|request| (request, action));
            self.last_auto_send = Instant::now();
        }
        Ok(())
    }

    /// Thực hiện hành động của luồng tự động: đều là lệnh phòng thật trên dây.
    ///
    /// Trả về request ID đã dùng để caller gửi lại **cùng** lệnh khi chưa thấy phản hồi.
    fn execute_auto(&mut self, action: AutoAction) -> Result<Option<u64>, NetError> {
        let Some(character) = self.auto.as_ref().map(AutoRoomFlow::character) else {
            return Ok(None);
        };
        let Some(room_action) = action.room_action(character) else {
            self.request_lobby()?;
            return Ok(None);
        };
        let request = self.room_command(room_action)?;
        Ok(Some(request))
    }

    /// Gửi lại lệnh phòng của luồng tự động khi chưa thấy phản hồi.
    ///
    /// Mạng có thể mất frame: client gửi lại **cùng** request ID nên server dedup, lệnh không
    /// bị áp hai lần (ADR 0009 Addendum B §B.2). Chỉ luồng tự động (`--auto-ready`) tự gửi lại;
    /// client có UI thì người dùng bấm lại.
    fn auto_retry(&mut self) -> Result<(), NetError> {
        let Some(pending) = self.auto.as_ref().and_then(AutoRoomFlow::pending) else {
            return Ok(());
        };
        let Some((request, action)) = self.last_auto.clone() else {
            return Ok(());
        };
        if *pending != action {
            return Ok(());
        }
        if self.last_auto_send.elapsed() < ROOM_RETRY_INTERVAL {
            return Ok(());
        }
        let Some(character) = self.auto.as_ref().map(AutoRoomFlow::character) else {
            return Ok(());
        };
        let Some(room_action) = action.room_action(character) else {
            return Ok(());
        };
        self.resend_room_command(request, room_action)?;
        self.last_auto_send = Instant::now();
        Ok(())
    }

    fn apply_snapshot(&mut self, snapshot: boom_proto::Snapshot) -> Result<(), NetError> {
        // Round mới: dựng lại predictor để `World` khớp content của round đó; snapshot của
        // round cũ tới trễ thì predictor tự bỏ (`SnapshotOutcome::OldRound`).
        let current_round = self.predictor.as_ref().and_then(Predictor::round_id);
        let rebuild = match current_round {
            None => true,
            Some(round) => snapshot.round_id > round,
        };
        if rebuild {
            let Some(player) = self.player_slot else {
                return Err(NetError::Unexpected(
                    "chưa handshake nên chưa biết slot người chơi".into(),
                ));
            };
            // `World` từ nội dung cục bộ nếu có; không có thì dựng từ snapshot (map chưa có
            // bundle nên lấy địa hình theo snapshot).
            let world = match self.round_world.clone() {
                Some(world) => world,
                None => world_from_snapshot(&snapshot, &self.rules, self.mode)
                    .map_err(|error| NetError::Unexpected(format!("dựng world client: {error}")))?,
            };
            self.predictor = Some(Predictor::new(world, player).map_err(|error| {
                NetError::Unexpected(format!("dựng predictor client: {error}"))
            })?);
        }
        let predictor = self.predictor.as_mut().expect("đã dựng predictor ở trên");
        let outcome = predictor.apply_snapshot(&snapshot)?;
        // Metadata chỉ cập nhật khi snapshot **được áp**: snapshot cũ/trùng hoặc của round cũ
        // không được làm lùi `ack_snapshot`.
        if let SnapshotOutcome::Applied { .. } = outcome {
            self.last_snapshot_seq = snapshot.snapshot_seq;
        }
        Ok(())
    }

    /// Gửi cửa sổ lệnh chưa ack (bắt đầu từ lệnh chưa ack nhỏ nhất, ADR 0009 §3.2).
    fn send_window(&mut self) -> Result<(), NetError> {
        let Some(predictor) = self.predictor.as_ref() else {
            return Ok(());
        };
        if !predictor.is_playing() {
            // Lượt đã kết thúc: không gửi lệnh nữa.
            return Ok(());
        }
        if predictor.pending().is_empty() {
            return Ok(());
        }
        let commands = predictor.window(self.session.limits().max_commands);
        if commands.is_empty() {
            return Ok(());
        }
        let (Some(round_id), Some(player)) = (self.current_round(), self.player_slot) else {
            return Ok(());
        };
        let batch = InputBatch {
            round_id,
            player,
            ack_snapshot: self.last_snapshot_seq,
            commands,
        };
        self.session.send_input_batch(&batch)
    }
}

/// Dựng `Hello` cho handshake từ profile rules của client.
///
/// `content_sha256` là hash manifest content: client chưa có bundle (P4) nên caller truyền
/// vào (dev dùng toàn 0, server so với bundle của chính nó — ADR 0009 §1).
pub fn hello_for_profile(
    profile: &RulesProfile,
    rules: &Rules,
    content_sha256: [u8; 32],
    client_build: &str,
) -> Hello {
    Hello {
        wire_schema: boom_proto::limits::WIRE_SCHEMA,
        state_schema: 1,
        rules_profile_id: profile.profile_id.clone(),
        rules_hash: rules.profile_hash,
        content_sha256,
        client_build: client_build.to_owned(),
        capabilities: Vec::new(),
    }
}

/// Nối TCP tới server và dựng client mạng.
///
/// Đọc/ghi tách hai chiều bằng hai socket clone của cùng stream: một luồng đọc chặn, phía ghi
/// gọi trực tiếp (ADR 0006: một kết nối TCP mỗi client, `TCP_NODELAY`).
pub fn connect_tcp(
    addr: &str,
    rules: Rules,
    mode: &'static dyn Mode,
) -> Result<NetClient<TcpTransport>, NetError> {
    let stream = std::net::TcpStream::connect(addr)
        .map_err(|error| NetError::Io(format!("không kết nối được {addr}: {error}")))?;
    let writer_stream = stream
        .try_clone()
        .map_err(|error| NetError::Io(format!("không clone được socket: {error}")))?;
    let reader = TcpTransport::from_stream(stream)
        .map_err(|error| NetError::Io(format!("không bọc được socket đọc: {error}")))?;
    let writer = TcpTransport::from_stream(writer_stream)
        .map_err(|error| NetError::Io(format!("không bọc được socket ghi: {error}")))?;
    Ok(NetClient::new(
        ClientSession::new(reader, writer),
        rules,
        mode,
    ))
}

/// Nối server bằng profile rules dev của client rồi handshake **và** đăng nhập.
///
/// Client chưa có bundle content (P4) nên hash content khai trong `Hello` lấy từ biến môi
/// trường `BOOM_CONTENT_SHA256` (hex 64 ký tự); không có thì khai toàn 0. Server so với bundle
/// **của chính nó** (ADR 0009 §1), nên lệch sẽ bị từ chối rõ ràng chứ không đoán.
///
/// Sau `Welcome` client phải `Auth` (ADR 0009 Addendum A §A.1) vì server chặn mọi kind khác
/// trước `AuthOk`.
pub fn connect(addr: &str, credentials: &Credentials) -> Result<NetClient<TcpTransport>, NetError> {
    let profile = RulesProfile::from_toml(include_str!("../../../../data/rules/v1-dev.toml"))
        .map_err(|error| NetError::Unexpected(format!("profile rules: {error}")))?;
    let rules = Rules::from_profile(&profile)
        .map_err(|error| NetError::Unexpected(format!("rules: {error}")))?;
    let Some(mode) = mode_by_id("battle_ffa") else {
        return Err(NetError::Unexpected("mode battle_ffa chưa đăng ký".into()));
    };
    let hello = hello_for_profile(&profile, &rules, content_sha256_from_env(), "boom-client");
    let mut client = connect_tcp(addr, rules, mode)?;
    client.handshake(&hello)?;
    client.authenticate(credentials)?;
    Ok(client)
}

/// Hash content client khai ở handshake: đọc `BOOM_CONTENT_SHA256` (hex), lỗi thì toàn 0.
pub fn content_sha256_from_env() -> [u8; 32] {
    let Ok(text) = std::env::var("BOOM_CONTENT_SHA256") else {
        return [0; 32];
    };
    let text = text.trim();
    if text.len() != 64 {
        return [0; 32];
    }
    let bytes = text.as_bytes();
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let hi = (bytes[index * 2] as char).to_digit(16);
        let lo = (bytes[index * 2 + 1] as char).to_digit(16);
        let (Some(hi), Some(lo)) = (hi, lo) else {
            return [0; 32];
        };
        *byte = u8::try_from(hi * 16 + lo).expect("digit hex nằm trong 0..=15");
    }
    out
}
