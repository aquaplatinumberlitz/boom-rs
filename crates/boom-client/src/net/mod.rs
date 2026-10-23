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

/// Backend UI nối server thật (ADR 0009 Addendum B §B.6).
///
/// **Chưa có caller trong binary**: luồng `--connect` hiện vẫn lấy thông tin đăng nhập từ
/// cờ `--user`/`--password` (xem `main.rs`), nên module này chưa được nối vào CLI. Vì vậy
/// `dead_code` tắt cho tới khi màn login UI được nối vào luồng `--connect`.
#[allow(dead_code)]
pub mod ui_backend;

pub use prediction::{
    ALPHA_ONE, BombShadow, PredictError, Predictor, SnapshotOutcome, world_from_snapshot,
};
pub use room::{
    AutoAction, AutoRoomFlow, LocalContent, PendingAck, RoomError, RoomEvent, RoomFlow,
    RoundExpectation, RoundPlan, RoundStartOutcome, world_for_round,
};
pub use session::{ClientMessage, ClientSession, NetError};

use std::fmt;
use std::time::{Duration, Instant};

use boom_content::rules::RulesProfile;
use boom_core::{
    Cell, EntityId, EventId as CoreEventId, EventKind, ItemKind, Mode, Outcome, Phase, PlayerInput,
    Rules, SimulationEvent, SimulationState, World, mode_by_id,
};
use boom_net::{TcpTransport, Transport};
use boom_proto::dto::auth::{Auth, AuthOk, AuthOp};
use boom_proto::{
    Hello, InputBatch, LoadingAck, ProtocolLimits, ResultAck, RoomAction, RoomCharacter,
    RoundEvent, RoundStart, Welcome,
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
///
/// Khoảng này dùng cho **hai** nhịp gửi lại — lệnh phòng và ack phòng — nhưng mỗi nhịp có
/// **đồng hồ riêng** ([`NetClient::last_auto_send`] và [`NetClient::last_ack_send`]): nếu
/// chung một đồng hồ thì nhịp của lệnh phòng sẽ "nuốt" nhịp của ack, và frame ack mất sẽ
/// không được gửi lại ⇒ client phải chờ timeout phía server.
const ROOM_RETRY_INTERVAL: Duration = Duration::from_millis(500);

/// Chu kỳ client tự gửi `Ping` để đo RTT của đường input (ms).
///
/// **Tham số vận hành** (design), không phải luật chơi: `Ping`/`Pong` là message hai chiều có
/// sẵn trong giao thức (ADR 0009 §2), server trả nguyên payload. RTT đo được dùng để ước lượng
/// tick authority lúc batch input tới server — không lấy độ trễ biết trước của fixture.
const INPUT_RTT_PING_INTERVAL: Duration = Duration::from_secs(1);

/// Hạn chờ `Pong` trước khi coi mẫu RTT bị mất và gửi `Ping` mới.
///
/// Mạng mất frame là bình thường (SCOPE_V1 §7.6); mẫu cũ không được chặn nhịp đo mãi.
const INPUT_RTT_PING_TIMEOUT: Duration = Duration::from_secs(2);

/// Bước một tick của client: bơm message, dự đoán, gửi cửa sổ lệnh.
pub struct NetClient<W: Transport> {
    session: ClientSession<W>,
    rules: Rules,
    mode: &'static dyn Mode,
    predictor: Option<Predictor>,
    /// Slot trận của client: lấy từ mục `RoundStart.roster` có `player == account_id`.
    ///
    /// **Không** lấy từ `Welcome.player_id`: đó là định danh **kết nối**, không phải slot
    /// trận (ADR 0009 Addendum B §B.2).
    player_slot: Option<u8>,
    /// Định danh kết nối trong phiên (`Welcome.player_id`).
    ///
    /// Chỉ là thông tin nhận diện phiên: **không** dùng làm slot trận, không dùng làm định
    /// danh người chơi trong phòng (đó là `AuthOk.account_id`).
    connection_id: Option<u8>,
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
    /// Mốc **local** nhận snapshot đã áp gần nhất.
    ///
    /// Cùng với tick của snapshot, đây là gốc để ước lượng tick authority hiện tại khi gửi
    /// input. Chỉ cập nhật khi snapshot thực sự được áp (`SnapshotOutcome::Applied`).
    snapshot_received_at: Option<Instant>,
    /// RTT gần nhất của đường input, đo bằng `Ping`/`Pong` của client.
    ///
    /// Không lấy `one_way_delay_ms` của fixture: client thật không biết trước độ trễ mạng.
    input_rtt: Option<Duration>,
    /// Target `tick` mà client dự đoán batch input mới tới server.
    ///
    /// `None` trên đường sản phẩm: dùng ước lượng từ snapshot + RTT đo được. Chỉ có
    /// fixture/test gán giá trị để **điều khiển mốc thời gian tường minh** (đồng hồ ảo của
    /// fixture không đo bằng `Instant` thật): giữ ngoài đường production.
    input_target_override: Option<u32>,
    /// Nonce `Ping` kế tiếp (khác 0 để không trùng marker test).
    input_ping_nonce: u64,
    /// `Ping` đang chờ `Pong`: `(nonce, mốc gửi local)`.
    input_ping_pending: Option<(u64, Instant)>,
    /// Mốc sớm nhất được gửi `Ping` đo RTT kế tiếp.
    input_ping_at: Instant,
    /// Event mới (đã dedup), chờ presentation lấy: `(round, event)`.
    ///
    /// **Round** đi kèm event vì khoá dedup của consumer là `(round_id, EventId)`
    /// (ADR 0005, R-089): event của lượt cũ tới trễ phải giữ được lượt của nó, không được
    /// ghi nhầm vào lượt mới.
    events: Vec<(u32, RoundEvent)>,
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
    /// Mốc gửi lần gần nhất của **ack phòng** (`LoadingAck`/`ResultAck`).
    ///
    /// **Tách khỏi** [`NetClient::last_auto_send`]: hai nhịp gửi lại phải độc lập, nếu chung
    /// một đồng hồ thì lệnh phòng gửi lại liên tục sẽ chặn ack gửi lại, và frame ack mất
    /// sẽ không bao giờ được gửi lại (client chỉ chờ timeout phía server).
    last_ack_send: Instant,
    /// Khoảng cách tối thiểu giữa hai lần gửi lại lệnh phòng đang chờ phản hồi.
    ///
    /// Mặc định [`ROOM_RETRY_INTERVAL`]. Fixture đặt [`Duration::ZERO`] để gửi lại **mỗi** vòng
    /// bơm: mạng ảo chạy theo đồng hồ ảo nên thời gian thật không được quyết định kết quả test.
    ///
    /// Khoảng này dùng cho **cả hai** nhịp gửi lại (lệnh phòng và ack phòng); mỗi nhịp có
    /// đồng hồ riêng nên chúng không chen nhau.
    room_retry_interval: Duration,
    /// Nhịp pump giao thức chờ kết quả; backoff 4, 8, … tối đa 64 nhịp.
    result_retry: Option<(u64, u32, u32)>,
    result_unavailable: Option<u64>,
}

/// Cấu hình recovery theo nhịp pump giao thức (design, ADR 0009 §B.7).
const RESULT_RETRY_INITIAL_PUMPS: u32 = 4;
const RESULT_RETRY_MAX_PUMPS: u32 = 64;

impl<W: Transport> NetClient<W> {
    /// Dựng client mạng trên một transport hai chiều.
    pub fn new(session: ClientSession<W>, rules: Rules, mode: &'static dyn Mode) -> Self {
        Self::with_room_retry_interval(session, rules, mode, ROOM_RETRY_INTERVAL)
    }

    /// Dựng client mạng với khoảng cách gửi lại lệnh phòng tuỳ chọn.
    ///
    /// `interval` là **tham số vận hành** (design), không phải luật chơi: khoảng cách giữa hai
    /// lần gửi lại lệnh phòng đang chờ phản hồi. Fixture đặt [`Duration::ZERO`] để gửi lại
    /// **mỗi** vòng bơm: mạng ảo chạy theo đồng hồ ảo nên thời gian thật không được quyết định
    /// kết quả test.
    pub fn with_room_retry_interval(
        session: ClientSession<W>,
        rules: Rules,
        mode: &'static dyn Mode,
        interval: Duration,
    ) -> Self {
        Self {
            session,
            rules,
            mode,
            predictor: None,
            player_slot: None,
            connection_id: None,
            welcome: None,
            account: None,
            last_snapshot_seq: 0,
            snapshot_interval_ms: 0,
            snapshot_received_at: None,
            input_rtt: None,
            input_target_override: None,
            input_ping_nonce: 0,
            input_ping_pending: None,
            input_ping_at: Instant::now(),
            events: Vec::new(),
            room: RoomFlow::new(),
            local_content: None,
            auto: None,
            content_sha256: [0; 32],
            profile_id: String::new(),
            round_world: None,
            last_auto: None,
            last_auto_send: Instant::now(),
            last_ack_send: Instant::now(),
            room_retry_interval: interval,
            result_retry: None,
            result_unavailable: None,
        }
    }

    /// Handshake với server.
    pub fn handshake(&mut self, hello: &Hello) -> Result<Welcome, NetError> {
        let welcome = self.session.handshake(hello)?;
        // `Welcome.player_id` là **định danh kết nối** trong phiên, không phải slot trận:
        // slot trận lấy từ `RoundStart.roster` theo `AuthOk.account_id`
        // (ADR 0009 Addendum B §B.2).
        self.connection_id = Some(welcome.player_id);
        self.snapshot_interval_ms = u64::from(welcome.snapshot_interval_ms.max(1));
        self.welcome = Some(welcome.clone());
        // Fingerprint client dùng để kiểm `RoundStart`: đúng những gì client khai ở handshake.
        self.content_sha256 = hello.content_sha256;
        self.profile_id = hello.rules_profile_id.clone();
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
        // Định danh người chơi trong phòng/round: `account_id` (server bind mọi trường `player`
        // trên dây từ giá trị này — ADR 0009 Addendum B §B.2).
        self.room.set_my_player(ok.account_id);
        Ok(ok)
    }

    /// Tài khoản đã đăng nhập (sau `AuthOk`).
    pub fn account(&self) -> Option<&AuthOk> {
        self.account.as_ref()
    }

    /// Miền giới hạn giao thức của phiên hiện tại.
    ///
    /// Client dùng để hiển thị giới hạn của phiên (vd. số người tối đa trong sảnh) thay vì
    /// hard-code — giá trị nằm trong `ProtocolLimits` có revision (ADR 0009 §3.5).
    pub fn limits(&self) -> &ProtocolLimits {
        self.session.limits()
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
        // Ack phòng chưa có bằng chứng server đã nhận thì gửi lại (idempotent).
        self.ack_retry()?;
        self.result_retry()?;
        Ok(())
    }

    /// Dự đoán một bước của mình: sinh lệnh mới (chưa gửi).
    ///
    /// `target_tick` của lệnh mới lấy từ ước lượng tick authority hiện tại
    /// ([`NetClient::input_target_tick`]), không phải tick của state dự đoán đã cũ so với
    /// server. Nếu lệnh phải chờ cửa sổ gửi, [`NetClient::send_window`] gán lại `target_tick`
    /// tại lần đầu vào batch (ADR 0009 §3.2); lệnh đã gửi thì payload bất biến.
    ///
    /// Chưa có snapshot nào thì chưa dự đoán được: lệnh chỉ được tạo khi đã biết state
    /// confirmed (ADR 0005). Lượt đã kết thúc thì **không** sinh lệnh: kết quả do server quyết,
    /// client giữ state confirmed và vẫn nhận message.
    ///
    /// Lệnh được gửi đi bằng [`NetClient::resend`].
    pub fn predict(&mut self, input: PlayerInput) -> Result<(), NetError> {
        let target_tick = self.input_target_tick();
        let Some(predictor) = self.predictor.as_mut() else {
            return Ok(());
        };
        if !predictor.is_playing() {
            return Ok(());
        }
        predictor.predict_at(input, target_tick)?;
        Ok(())
    }

    /// Gửi cửa sổ lệnh chưa ack (bắt đầu từ lệnh chưa ack nhỏ nhất, ADR 0009 §3.2).
    ///
    /// ADR 0009 §3.2: "client gửi lại cùng cửa sổ cho tới khi ack vượt qua lệnh đó; không có
    /// cơ chế gửi một lần rồi thôi". Lệnh đã gửi mà chưa ack vẫn phải được gửi lại **y nguyên
    /// payload** (không đổi `target_tick`) để server nhận được sau khi frame đầu bị mất.
    pub fn resend(&mut self) -> Result<(), NetError> {
        // Đo RTT đường input bằng Ping/Pong của chính client (server trả nguyên payload).
        // Chỉ gửi khi thực sự gửi cửa sổ lệnh, không làm nhiễu lưu lượng phòng/loading.
        self.input_ping_retry()?;
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

    /// Ước lượng tick authority lúc batch input mới tới server.
    ///
    /// Gốc là snapshot đã áp: `tick(snapshot) + số tick trôi từ lúc nhận + RTT đo được`.
    /// Cộng cả RTT (không phải nửa RTT) vì `target_tick` là tick **lúc batch tới server**, còn
    /// snapshot đã cũ hơn một chiều khi vừa nhận. Không dùng độ trễ biết trước của fixture:
    /// RTT lấy từ `Ping`/`Pong` của chính client ([`NetClient::input_ping_retry`]).
    ///
    /// ADR 0009 §3.2 cho phép tính/chỉnh `target_tick` cho lệnh **chưa gửi**; caller
    /// ([`NetClient::predict`]) dùng ước lượng này lúc tạo lệnh, và
    /// [`NetClient::send_window`] gán lại khi lệnh lần đầu vào batch.
    /// Test hook: đặt target tick tường minh cho fixture có đồng hồ ảo.
    ///
    /// Không dùng trên đường sản phẩm; production luôn `None` để ước lượng từ snapshot/RTT.
    #[cfg(test)]
    pub(crate) fn set_input_target_override(&mut self, target: Option<u32>) {
        self.input_target_override = target;
    }

    fn input_target_tick(&self) -> u32 {
        // Chỉ có fixture/test gán override để điều khiển mốc thời gian tường minh; đường
        // sản phẩm luôn tham chiếu target_tick None ⇒ dùng ước lượng bên dưới.
        if let Some(target_tick) = self.input_target_override {
            return target_tick;
        }
        self.input_target_tick_at(Instant::now())
    }

    /// Như [`NetClient::input_target_tick`] nhưng nhận mốc `now` để test điều khiển đồng hồ.
    fn input_target_tick_at(&self, now: Instant) -> u32 {
        let Some(confirmed) = self.predictor.as_ref().and_then(Predictor::confirmed_state) else {
            return 0;
        };
        let Some(received_at) = self.snapshot_received_at else {
            return confirmed.tick;
        };
        let hz = self.rules.ticks_per_second.max(1);
        let elapsed_ms = u32::try_from(now.saturating_duration_since(received_at).as_millis())
            .unwrap_or(u32::MAX);
        let rtt_ms = self
            .input_rtt
            .map_or(0, |rtt| u32::try_from(rtt.as_millis()).unwrap_or(u32::MAX));
        // Chuyển ms → tick bằng tay thay vì `ms_to_ticks`: `elapsed` có thể lớn (quá hạn
        // `Pong`), mà `ms_to_ticks` panic khi kết quả vượt `Tick`; ở đây kẹp tại `u32::MAX`
        // rồi để `saturating_add` trên `target_tick` xử lý phần còn lại.
        let ticks = u64::from(elapsed_ms.saturating_add(rtt_ms)) * u64::from(hz);
        let ticks = ticks.div_ceil(1_000);
        confirmed
            .tick
            .saturating_add(u32::try_from(ticks).unwrap_or(u32::MAX))
    }

    /// Gửi `Ping` đo RTT nếu đã tới nhịp và không có mẫu nào đang chờ.
    ///
    /// Chỉ chạy sau `AuthOk`: trước đó server vẫn echo `Ping`, nhưng không cần đo RTT khi chưa
    /// vào trận. Mẫu quá hạn bị bỏ để một `Pong` mất không chặn nhịp đo.
    fn input_ping_retry(&mut self) -> Result<(), NetError> {
        if self.account.is_none() {
            return Ok(());
        }
        // Chỉ đo RTT khi đang trong trận: Ping trong sảnh/loading không cần cho input và
        // làm nhiễu các test đang kiểm frame room.
        if !self
            .predictor
            .as_ref()
            .is_some_and(|predictor| predictor.is_playing())
        {
            return Ok(());
        }
        let now = Instant::now();
        if let Some((_, sent_at)) = self.input_ping_pending {
            if now.saturating_duration_since(sent_at) < INPUT_RTT_PING_TIMEOUT {
                return Ok(());
            }
            // Mẫu cũ coi như mất; cho phép gửi mẫu mới ngay.
            self.input_ping_pending = None;
            self.input_ping_at = now;
        }
        if now < self.input_ping_at {
            return Ok(());
        }
        self.input_ping_nonce = self.input_ping_nonce.wrapping_add(1).max(1);
        let nonce = self.input_ping_nonce;
        self.session.send_ping(nonce)?;
        self.input_ping_pending = Some((nonce, now));
        self.input_ping_at = now + INPUT_RTT_PING_INTERVAL;
        Ok(())
    }

    /// Ghi nhận `Pong` khớp nonce đang chờ và làm mượt RTT (EWMA 1/4).
    ///
    /// `Pong` lạ (không khớp mẫu đang chờ) bị bỏ: server có thể gửi `Pong` của phiên khác
    /// hoặc frame trùng tới muộn, không được làm sai ước lượng.
    fn record_input_pong(&mut self, nonce: u64) {
        let Some((pending, sent_at)) = self.input_ping_pending else {
            return;
        };
        if pending != nonce {
            return;
        }
        let sample = sent_at.elapsed();
        self.input_rtt = Some(match self.input_rtt {
            None => sample,
            Some(previous) => {
                let nanos = previous
                    .as_nanos()
                    .saturating_mul(3)
                    .saturating_add(sample.as_nanos())
                    / 4;
                Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
            }
        });
        self.input_ping_pending = None;
    }

    /// Lấy event mới (đã dedup theo `(round_id, EventId)`) — dạng **thô** trên dây.
    ///
    /// Hàm này và [`NetClient::take_core_events`] rút cùng một hàng đợi: gọi một trong hai
    /// là hàng đợi rỗng. Đường trình diễn dùng [`NetClient::take_core_events`]; hàm này
    /// để test đọc event thô.
    #[allow(dead_code)] // đường trình diễn dùng `take_core_events`; hàm này cho test đọc event thô.
    pub fn take_events(&mut self) -> Vec<RoundEvent> {
        self.events.drain(..).map(|(_, event)| event).collect()
    }

    /// Lấy event mới (đã dedup theo `(round_id, EventId)`) đã **dịch** về event core.
    ///
    /// Kèm **round của event**: consumer trình diễn dùng round đó làm khoá dedup
    /// `(round_id, EventId)` nên event của lượt cũ tới trễ không tạo mục ở lượt mới.
    ///
    /// Event không dịch được (tên loại lạ, thiếu trường) thì **bỏ**: consumer không đoán
    /// giá trị của event lạ (ADR 0009 §2).
    pub fn take_core_events(&mut self) -> Vec<(u32, SimulationEvent)> {
        self.events
            .drain(..)
            .filter_map(|(round, event)| decode_event(&event).map(|event| (round, event)))
            .collect()
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
    pub fn resend_room_command(
        &mut self,
        request: u64,
        action: RoomAction,
    ) -> Result<(), NetError> {
        let command = self.room.command_with_request(request, action);
        self.session.send_room_command(&command)
    }

    /// Xin danh sách phòng sảnh.
    pub fn request_lobby(&mut self) -> Result<(), NetError> {
        self.session.send_lobby_list_request()
    }

    /// Xin server gửi snapshot đầy đủ kế tiếp để **đồng bộ state** (ADR 0009 §4.3).
    ///
    /// Client dùng khi state confirmed có thể đã cũ hơn state server: vừa vào lượt (server
    /// đã chạy qua vài tick trước khi client nhận `RoundStart`), vừa đổi phòng, hoặc nghi
    /// ngờ lệch. Server trả snapshot của state hiện tại; client áp nó làm gốc dự đoán mới.
    ///
    /// `Err` khi client chưa biết round nào để xin (chưa có `RoundStart` lẫn snapshot).
    ///
    /// `#[allow(dead_code)]`: tầng UI chưa có nút resync; test end-to-end dùng để đồng bộ
    /// gốc dự đoán của client với state hiện tại của server trước khi chơi.
    #[allow(dead_code)]
    pub fn request_snapshot(&mut self) -> Result<(), NetError> {
        let round_id = self.current_round().or_else(|| {
            self.room
                .round()
                .and_then(|plan| u32::try_from(plan.round_id).ok())
        });
        let Some(round_id) = round_id else {
            return Err(NetError::Unexpected(
                "chưa biết round nào để xin snapshot".into(),
            ));
        };
        self.session
            .request_snapshot(round_id, self.last_snapshot_seq)
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

    /// Slot trận của client (mục `RoundStart.roster` có `player == account_id`).
    ///
    /// `None` = client chưa nhận `RoundStart` nên chưa biết slot của mình.
    pub fn player_slot(&self) -> Option<u8> {
        self.player_slot
    }

    /// Định danh kết nối trong phiên (`Welcome.player_id`).
    ///
    /// **Không** phải slot trận: slot trận lấy từ roster theo `account_id`
    /// (ADR 0009 Addendum B §B.2).
    ///
    /// `#[allow(dead_code)]`: accessor này chỉ dùng để **chứng minh** client không lấy slot
    /// từ `Welcome` (test end-to-end); tầng UI chưa cần hiển thị định danh kết nối.
    #[allow(dead_code)]
    pub fn connection_id(&self) -> Option<u8> {
        self.connection_id
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
                let round_id = events.round_id;
                if self
                    .current_round()
                    .is_none_or(|round| events.round_id >= round)
                {
                    let fresh = self.session.accept_events(events);
                    if self
                        .room
                        .round()
                        .is_some_and(|round| round.round_id == u64::from(round_id))
                        && fresh.iter().any(|event| event.name == "round_ended")
                    {
                        self.result_retry.get_or_insert((
                            u64::from(round_id),
                            0,
                            RESULT_RETRY_INITIAL_PUMPS,
                        ));
                    }
                    self.events
                        .extend(fresh.into_iter().map(|event| (round_id, event)));
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
                if self.room.on_round_result(result) {
                    self.result_retry = None;
                    self.send_result_ack(round_id)?;
                    self.drive_auto(RoomEvent::RoundResult)?;
                }
            }
            ClientMessage::RoomRejected(rejected) => {
                if rejected.reason == boom_proto::RoomRejectReason::StaleRequest
                    && self
                        .result_retry
                        .is_some_and(|retry| retry.0 == rejected.request)
                {
                    self.result_unavailable = Some(rejected.request);
                    self.result_retry = None;
                    return Ok(());
                }
                self.room.on_rejected(rejected);
                self.drive_auto(RoomEvent::Rejected)?;
            }
            ClientMessage::UnknownKind(code) => {
                let _ = self.session.close();
                return Err(NetError::Unexpected(format!(
                    "server gửi kind lạ {code:#06x}; đóng kết nối"
                )));
            }
            ClientMessage::Pong(pong) => self.record_input_pong(pong.0),
            ClientMessage::Welcome(_) => {}
        }
        Ok(())
    }

    /// `RoundStart`: kiểm fingerprint, dựng world cho round rồi ack loading.
    ///
    /// Fingerprint lệch ⇒ client **không** ack loading: loading barrier của server hủy start
    /// (SCOPE_V1 §3) thay vì client chơi sai rules/content. Lỗi được lưu lại để caller hiện
    /// ra ngoài (xem [`NetClient::room_error`]).
    ///
    /// `RoundStart` **cũ** hoặc **trùng** round đang chơi thì bỏ: không đổi kế hoạch round,
    /// không đổi slot trận, không ack và không chạy luồng tự động (xem
    /// [`RoomFlow::on_round_start`]).
    fn handle_round_start(&mut self, start: RoundStart) -> Result<(), NetError> {
        let expectation = RoundExpectation {
            profile_id: self.profile_id.clone(),
            rules_hash: self.rules.profile_hash,
            content_sha256: self.content_sha256,
            mode: self.mode.id().to_owned(),
        };
        let outcome = match self.room.on_round_start(start, &expectation) {
            Ok(outcome) => outcome,
            Err(_) => {
                // Lỗi đã lưu trong luồng phòng: không ack loading.
                return Ok(());
            }
        };
        let RoundStartOutcome::Started(plan) = outcome else {
            // `RoundStart` cũ/trùng: bỏ trước mọi side effect.
            return Ok(());
        };
        self.result_retry = None;
        self.result_unavailable = None;
        // Slot trận của client: mục roster có `player == account_id` (không phải
        // `Welcome.player_id` — đó là định danh kết nối).
        self.player_slot = self.room.my_slot();
        // Dựng `World` cho round: ưu tiên nội dung cục bộ của client, không có thì để
        // `World` dựng từ snapshot (server là authority về lưới ô).
        match world_for_round(&plan, self.local_content.as_ref(), &self.rules, self.mode) {
            Ok(world) => self.round_world = world,
            Err(error) => {
                self.room.set_error(error);
                return Ok(());
            }
        }
        self.send_loading_ack(plan.round_id)?;
        self.drive_auto(RoomEvent::RoundStart)?;
        Ok(())
    }

    /// Gửi `LoadingAck` cho round và ghi nhận mốc gửi (nhịp gửi lại **riêng** của ack).
    fn send_loading_ack(&mut self, round_id: u64) -> Result<(), NetError> {
        self.session.send_loading_ack(&LoadingAck { round_id })?;
        self.room.mark_loading_acked(round_id);
        self.last_ack_send = Instant::now();
        Ok(())
    }

    /// Gửi `ResultAck` cho round và ghi nhận mốc gửi (nhịp gửi lại **riêng** của ack).
    fn send_result_ack(&mut self, round_id: u64) -> Result<(), NetError> {
        self.session.send_result_ack(&ResultAck { round_id })?;
        self.room.mark_result_acked(round_id);
        self.last_ack_send = Instant::now();
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
        self.execute_auto(action)
    }

    /// Thực hiện hành động của luồng tự động: đều là lệnh phòng thật trên dây.
    fn execute_auto(&mut self, action: AutoAction) -> Result<(), NetError> {
        let Some(character) = self.auto.as_ref().map(AutoRoomFlow::character) else {
            return Ok(());
        };
        let Some(room_action) = action.room_action(character) else {
            // `RequestLobby` không phải `RoomCommand`: xin danh sách phòng sảnh.
            self.session.send_lobby_list_request()?;
            self.last_auto_send = Instant::now();
            return Ok(());
        };
        let request = self.room_command(room_action)?;
        self.last_auto = Some((request, action));
        self.last_auto_send = Instant::now();
        Ok(())
    }

    /// Gửi lại lệnh phòng của luồng tự động khi chưa thấy phản hồi.
    ///
    /// Mạng có thể mất frame: client gửi lại **cùng** request ID nên server dedup, lệnh không
    /// bị áp hai lần (ADR 0009 Addendum B §B.2). Đây cũng là cách **duy nhất** để bắt server gửi
    /// lại `RoomState` khi frame đó bị mất: protocol V1 không có kind riêng để xin trạng
    /// thái phòng, mà server chỉ trả `RoomState` khi nhận lệnh phòng.
    ///
    /// Chỉ luồng tự động (`--auto-ready`) tự gửi lại; client có UI thì người dùng bấm lại.
    fn auto_retry(&mut self) -> Result<(), NetError> {
        let Some(pending) = self.auto.as_ref().and_then(AutoRoomFlow::pending).cloned() else {
            return Ok(());
        };
        if self.last_auto_send.elapsed() < self.room_retry_interval {
            return Ok(());
        }
        if pending == AutoAction::RequestLobby {
            // `LobbyListRequest` không mang request ID: xin lại danh sách phòng.
            self.session.send_lobby_list_request()?;
            self.last_auto_send = Instant::now();
            return Ok(());
        }
        let Some((request, action)) = self.last_auto.clone() else {
            return Ok(());
        };
        if action != pending {
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

    /// Xin bảng authority sau Ended; không dùng snapshot để tự tạo rank/kết quả.
    fn result_retry(&mut self) -> Result<(), NetError> {
        let Some(round_id) = self.room.round().map(|round| round.round_id) else {
            self.result_retry = None;
            return Ok(());
        };
        let ended = self.current_round().map(u64::from) == Some(round_id)
            && matches!(self.phase(), Some(Phase::Ended { .. }));
        if !ended && self.result_retry.is_none() {
            return Ok(());
        }
        if self.result_unavailable == Some(round_id)
            || self
                .room
                .result()
                .is_some_and(|result| result.round_id == round_id)
        {
            self.result_retry = None;
            return Ok(());
        }
        let retry = self
            .result_retry
            .get_or_insert((round_id, 0, RESULT_RETRY_INITIAL_PUMPS));
        if retry.0 != round_id {
            *retry = (round_id, 0, RESULT_RETRY_INITIAL_PUMPS);
        }
        retry.1 += 1;
        if retry.1 >= retry.2 {
            self.session.request_round_result(round_id)?;
            retry.1 = 0;
            retry.2 = (retry.2 * 2).min(RESULT_RETRY_MAX_PUMPS);
        }
        Ok(())
    }

    /// Gửi lại ack phòng đang chờ bằng chứng server đã nhận.
    ///
    /// `LoadingAck`/`ResultAck` là idempotent (ADR 0009 Addendum B §B.2) nên gửi lại không đổi
    /// ý nghĩa; nếu không gửi lại, một frame bị mất sẽ làm loading barrier timeout hoặc khiến
    /// client chờ vô hạn ở màn kết quả.
    ///
    /// Nhịp gửi lại dùng **đồng hồ riêng** ([`NetClient::last_ack_send`]), không dùng chung
    /// với nhịp gửi lại lệnh phòng ([`NetClient::last_auto_send`]): nếu chung, mỗi lần gửi
    /// lại lệnh phòng sẽ đẩy mốc của ack về hiện tại ⇒ ack **không bao giờ** được gửi lại
    /// khi lệnh phòng còn pending (mà lệnh phòng thì luôn pending sau mỗi `RoundResult`:
    /// client chờ `SetReady` cho vòng sau).
    fn ack_retry(&mut self) -> Result<(), NetError> {
        let Some(pending) = self.room.pending_ack() else {
            return Ok(());
        };
        if self.last_ack_send.elapsed() < self.room_retry_interval {
            return Ok(());
        }
        match pending {
            PendingAck::Loading(round_id) => {
                self.session.send_loading_ack(&LoadingAck { round_id })?;
            }
            PendingAck::Result(round_id) => {
                self.session.send_result_ack(&ResultAck { round_id })?;
            }
        }
        self.last_ack_send = Instant::now();
        Ok(())
    }

    fn apply_snapshot(&mut self, snapshot: boom_proto::Snapshot) -> Result<(), NetError> {
        // Client chỉ áp snapshot của **round mình đã nhận `RoundStart`**:
        //
        // - Chưa có `RoundStart` ⇒ client chưa biết slot trận của mình (slot lấy từ
        //   `RoundStart.roster` theo `account_id`, ADR 0009 Addendum B §B.2) ⇒ bỏ.
        // - `round_id` khác round của kế hoạch ⇒ snapshot thuộc round khác (round cũ, hoặc
        //   round mới mà `RoundStart` chưa tới) ⇒ bỏ.
        //
        // Bỏ chứ không phải lỗi: server gửi `RoundStart` rồi mới gửi snapshot, nhưng mạng có
        // thể đảo thứ tự (`RoundStart` tới sau snapshot) và server còn gửi snapshot khi phòng
        // chưa vào lượt. Client chờ snapshot kế tiếp của round mình đang chơi.
        let Some(plan) = self.room.round() else {
            return Ok(());
        };
        if u64::from(snapshot.round_id) != plan.round_id {
            return Ok(());
        }
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
                    "chưa nhận RoundStart nên chưa biết slot người chơi trong trận".into(),
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
            // Gốc thời gian local của snapshot đã áp: dùng để ước lượng tick authority khi
            // gửi input (không lấy tick state dự đoán vốn cũ hơn server).
            self.snapshot_received_at = Some(Instant::now());
            // Snapshot cho thấy lượt đã chạy là bằng chứng server đã nhận `LoadingAck`
            // (loading barrier đã qua). Snapshot *trước lượt* (`tick == 0`) không phải bằng
            // chứng: xem [`RoomFlow::confirm_loading`].
            self.room
                .confirm_loading(u64::from(snapshot.round_id), snapshot.tick);
        }
        Ok(())
    }

    /// Gửi cửa sổ lệnh chưa ack (bắt đầu từ lệnh chưa ack nhỏ nhất, ADR 0009 §3.2).
    ///
    /// Lệnh **chưa gửi** trong cửa sổ được gán/chỉnh `target_tick` tại lần đầu vào batch,
    /// theo tick authority ước lượng lúc gửi. Lệnh đã gửi giữ nguyên payload để resend
    /// idempotent; đây là điều kiện để lệnh chờ cửa sổ không bị retire oan.
    fn send_window(&mut self) -> Result<(), NetError> {
        let target_tick = self.input_target_tick();
        let (Some(round_id), Some(player)) = (self.current_round(), self.player_slot) else {
            return Ok(());
        };
        let Some(predictor) = self.predictor.as_mut() else {
            return Ok(());
        };
        if !predictor.is_playing() {
            // Lượt đã kết thúc: không gửi lệnh nữa.
            return Ok(());
        }
        if predictor.pending().is_empty() {
            return Ok(());
        }
        let commands = predictor.window_for_send(self.session.limits().max_commands, target_tick);
        if commands.is_empty() {
            return Ok(());
        }
        let batch = InputBatch {
            round_id,
            player,
            ack_snapshot: self.last_snapshot_seq,
            commands,
        };
        self.session.send_input_batch(&batch)
    }
}

/// Dịch event trên dây (`RoundEvent`) về event của core (`SimulationEvent`).
///
/// Tên loại và tên trường lấy từ `EventKind::name()` / `EventKind::fields()` của core — cùng
/// bảng mà `boom_sync::to_round_events` dùng để **đóng gói** event lên dây, nên hai bên
/// dùng chung một tên chứ không phải đoán.
///
/// `None` khi tên loại lạ hoặc thiếu trường: consumer bỏ qua event không hiểu
/// (`EventKind` là `non_exhaustive`) thay vì đoán giá trị (ADR 0009 §2).
#[must_use]
pub fn decode_event(event: &RoundEvent) -> Option<SimulationEvent> {
    let field = |name: &str| -> Option<i64> {
        event
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.value)
    };
    let u8_field = |name: &str| -> Option<u8> { u8::try_from(field(name)?).ok() };
    let i32_field = |name: &str| -> Option<i32> { i32::try_from(field(name)?).ok() };
    let u32_field = |name: &str| -> Option<u32> { u32::try_from(field(name)?).ok() };
    let cell = || -> Option<Cell> {
        Some(Cell {
            x: i32_field("x")?,
            y: i32_field("y")?,
        })
    };
    let kind = match event.name.as_str() {
        "player_turned" => EventKind::PlayerTurned {
            player: u8_field("player")?,
        },
        "bomb_placed" => EventKind::BombPlaced {
            bomb: EntityId(u32_field("bomb")?),
            player: u8_field("player")?,
            cell: cell()?,
            range: u8_field("range")?,
        },
        "bomb_exploded" => EventKind::BombExploded {
            bomb: EntityId(u32_field("bomb")?),
            owner: u8_field("owner")?,
            cell: cell()?,
            chained: field("chained")? != 0,
        },
        "block_destroyed" => EventKind::BlockDestroyed { cell: cell()? },
        "item_revealed" => EventKind::ItemRevealed {
            item: EntityId(u32_field("item")?),
            kind: ItemKind(u16::try_from(field("kind")?).ok()?),
            cell: cell()?,
        },
        "item_destroyed" => EventKind::ItemDestroyed {
            item: EntityId(u32_field("item")?),
            cell: cell()?,
        },
        "item_picked" => EventKind::ItemPicked {
            item: EntityId(u32_field("item")?),
            kind: ItemKind(u16::try_from(field("kind")?).ok()?),
            player: u8_field("player")?,
        },
        "player_trapped" => EventKind::PlayerTrapped {
            player: u8_field("player")?,
            by: u8_field("by")?,
        },
        "player_died" => {
            let by = field("by")?;
            EventKind::PlayerDied {
                player: u8_field("player")?,
                by: if by < 0 {
                    None
                } else {
                    Some(u8::try_from(by).ok()?)
                },
            }
        }
        "round_ended" => {
            let winner = field("winner")?;
            EventKind::RoundEnded {
                outcome: if winner < 0 {
                    Outcome::Draw
                } else {
                    Outcome::Winner(u8::try_from(winner).ok()?)
                },
            }
        }
        _ => return None,
    };
    Some(SimulationEvent {
        id: CoreEventId {
            tick: event.id.tick,
            seq: event.id.seq,
        },
        kind,
    })
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
