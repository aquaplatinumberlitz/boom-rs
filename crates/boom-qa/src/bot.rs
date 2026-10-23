//! Bot headless của harness QA (V1-027).
//!
//! # Vì sao có trait
//!
//! [`BotClient`] là điểm thay thế. Bot mặc định giữ tên API [`DirectDtoBot`] để tương thích,
//! nhưng input dùng chính Predictor của client; fingerprint/roster dùng RoomFlow chung.
//! Auth và room command của harness vẫn dùng DTO.
//!
//! Harness **không** tự dựng bot: nó gọi [`BotFactory`] cho từng bot của lần chạy. Xem
//! [`crate::harness::run_with`] — đó là entry point nhận factory bên ngoài.
//!
//! # Luồng của bot
//!
//! `Hello` → `Welcome` → `Auth`(Register) → `AuthOk` → sảnh (`LobbyListRequest`) → phòng
//! (`Create`/`Join` → `SetReady` → `Start`) → `LoadingAck` → `RoundStart` → input ngẫu nhiên
//! có seed → `RoundResult` → `ResultAck` → về phòng → trận sau.
//!
//! # Bất biến quan trọng (hồ sơ mạng có mất/trùng/đảo thứ tự)
//!
//! Mọi frame từ server có thể **mất, trùng hoặc tới trễ**. Bot vì vậy phải idempotent:
//!
//! 1. **`Auth` chỉ gửi một lần mỗi phiên.** Server từ chối `Auth` lần hai sau `AuthOk`
//!    (ADR 0009 Addendum A §A.1) và đóng phiên; `Welcome` trùng (do mạng nhân bản frame) cũng
//!    gây ra điều đó. Chỉ khi server **trả lời** `AuthFail` thì bot mới được thử lại.
//! 2. **Mỗi lượt chỉ kết thúc một lần.** `RoundResult` và snapshot `Phase::Ended` đều có thể
//!    tới (trùng hoặc đảo thứ tự); bot gộp chúng qua tập lượt đã xử lý.
//! 3. **Lệnh phòng idempotent.** `Create`/`Join` chỉ gửi một lần cho mỗi phòng;
//!    `PlayerAlreadyInRoom` được hiểu là "đã vào phòng" chứ không phải lỗi.
//! 4. **Slot lấy từ server, không tự đặt.** `InputBatch.player` phải là **slot trong phòng**
//!    mà server cấp trong roster của lượt (server bind người chơi theo session rồi so với
//!    `slot_to_player` của lượt). Bot đọc slot từ roster `RoundStart`.
//! 5. **`LoadingAck` gửi khi biết lượt**, không chỉ khi nhận `RoundStart`: `RoundStart` có thể
//!    bị mất, còn `RoomState` (có `round.id`) thì server gửi lại mỗi khi phòng đổi.
//! 6. **RTT ghép theo nonce của `Ping`**: `Pong` mang nonce của đúng `Ping` nó trả lời
//!    (ADR 0009 §2). Mạng cố ý trùng/đảo thứ tự frame nên `Pong` của ping cũ có thể tới
//!    sau `Pong` của ping mới: mỗi `Pong` chỉ ghi mẫu RTT khi nonce khớp một ping **đang
//!    chờ**; `Pong` lạ thì bỏ qua.
//! 7. **Định danh trên dây là `account_id`**: mọi trường `player` của `RoomState` và
//!    `RoundStart.roster` mang `account_id` server cấp trong `AuthOk`, **không** phải
//!    `PlayerId` của kết nối (ADR 0009 Addendum B §B.2). Bot dùng nó để biết mình là host
//!    và lấy slot của mình trong phòng.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use boom_core::{Rng, World};
use boom_proto::dto::auth::{Auth, AuthFail, AuthFailReason, AuthOk, AuthOp};
use boom_proto::{
    Hello, InputBatch, InputRejected, LoadingAck, LobbyList, LobbyListRequest, Message,
    MessageKind, Ping, Pong, Reject, ResultAck, RoomAction, RoomCharacter, RoomCommand,
    RoomRejected, RoomState, RoundResult, RoundStart, Snapshot, encode_frame, read_envelope,
};

use crate::metrics::Samples;
use crate::prediction::{Predictor, SnapshotOutcome};
use crate::room::{RoomFlow, RoundExpectation, RoundStartOutcome};

/// Số chu kỳ `Ping` mà một ping được giữ trong bảng pending chờ `Pong`.
///
/// `Pong` mất vĩnh viễn thì mốc của ping đó không bao giờ được dùng; giữ nó mãi sẽ làm bảng
/// pending phình vô hạn trong lượt chạy dài.
const PING_PENDING_TTL_FACTOR: u64 = 10;

/// Lỗi của bot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BotError {
    /// Frame không decode được.
    Proto(String),
    /// Dựng lại state từ snapshot thất bại.
    Sync(String),
    /// Server từ chối bắt tay.
    Rejected(String),
    /// Auth thất bại.
    AuthFailed(AuthFailReason),
    /// Message lạ trong luồng bot.
    Unexpected(String),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proto(detail) => write!(f, "lỗi giao thức: {detail}"),
            Self::Sync(detail) => write!(f, "lỗi đồng bộ state: {detail}"),
            Self::Rejected(detail) => write!(f, "server từ chối: {detail}"),
            Self::AuthFailed(reason) => write!(f, "auth thất bại: {reason:?}"),
            Self::Unexpected(detail) => write!(f, "message lạ: {detail}"),
        }
    }
}

impl std::error::Error for BotError {}

/// Một lệnh input bot đã gửi (để harness đo độ trễ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SentCommand {
    /// Mốc (ms) bot gửi lệnh.
    pub sent_at_ms: u64,
    /// Tick bot nhắm tới.
    pub target_tick: u32,
}

/// Số liệu bot tự ghi.
#[derive(Clone, Debug, Default)]
pub struct BotStats {
    /// Định danh người chơi **trên dây** (`account_id` do server cấp trong `AuthOk`).
    ///
    /// `None` trước `AuthOk`. Mọi trường `player` trên dây và mọi khoá của server bên
    /// trong (`Authority::applied_seq`, sự kiện `InputRetired`) dùng giá trị này, **không**
    /// phải `PlayerId` của kết nối (ADR 0009 Addendum B §B.2). Harness đọc nó để tra
    /// cùng khoá với server khi đo độ trễ input.
    pub account_id: Option<u64>,
    /// Số lệnh input đã gửi.
    pub commands_sent: u32,
    /// Số batch input bị server từ chối.
    pub input_rejected: u32,
    /// Số lần auth thất bại.
    pub auth_failed: u32,
    /// Số lần bị server từ chối phiên.
    pub rejected: u32,
    /// Số lệnh phòng bị server từ chối.
    pub room_rejected: u32,
    /// Số snapshot đã nhận.
    pub snapshots: u32,
    /// Số mẫu sai số vị trí cùng tick khác 0.
    pub corrections: u32,
    /// Sai số Manhattan lớn nhất (mpx).
    pub max_correction_mpx: i32,
    /// Sai số vị trí tại cùng tick (Manhattan, mpx), gồm mẫu 0.
    pub correction_mpx: Samples,
    /// Số lượt bot đã chơi xong (đã biết lượt kết thúc).
    pub rounds_completed: u32,
    /// Số lượt bot đã ack kết quả (đã gửi `ResultAck`).
    pub results_acked: u32,
    /// Số lượt bot không dựng lại được state hash (snapshot kết thúc lượt không tới).
    pub snapshot_missed: u32,
    /// Lượt hiện tại của bot (0 = chưa vào lượt nào).
    pub round_id: u64,
    /// Lệnh đã gửi, theo `(lượt, sequence)`.
    pub sent: BTreeMap<(u64, u32), SentCommand>,
    /// Hash state bot dựng lại sau resync, theo lượt.
    pub round_hashes: BTreeMap<u64, u64>,
    /// RTT đo bằng Ping/Pong (ms).
    pub rtt: Samples,
}

/// Cấu hình một bot.
#[derive(Clone, Debug)]
pub struct BotConfig {
    /// Tên bot (đưa vào báo cáo).
    pub name: String,
    /// Tên đăng nhập.
    pub username: String,
    /// Mật khẩu.
    pub password: String,
    /// ID người chơi phía server.
    pub player: u64,
    /// Bot này tạo phòng khi sảnh chưa có phòng nào (bot được giao vai trò host).
    pub is_host: bool,
    /// Số bot trong lần chạy (số thành viên phòng mong đợi).
    pub expected_members: usize,
    /// Seed cho input ngẫu nhiên của bot.
    pub seed: u64,
    /// Thế giới dùng để dựng lại state từ snapshot.
    pub world: World,
    /// Định danh profile rules của phiên (handshake).
    pub rules_profile_id: String,
    /// Hash profile rules của phiên (handshake).
    pub rules_hash: u64,
    /// SHA-256 content của phiên (handshake).
    pub content_sha256: [u8; 32],
    /// Chu kỳ gửi input (ms).
    pub input_interval_ms: u64,
    /// Số tick nhắm trước tick server ước lượng khi gửi input (bù jitter/độ trễ còn lại).
    pub input_lead_ticks: u32,
    /// Chu kỳ gửi Ping để đo RTT (ms).
    pub ping_interval_ms: u64,
    /// Chu kỳ gửi lại lệnh idempotent khi chưa thấy phản hồi (ms).
    pub retry_ms: u64,
    /// Chu kỳ xin lại danh sách sảnh khi bot chưa vào được phòng (ms).
    pub lobby_retry_ms: u64,
    /// Trễ một chiều của hồ sơ mạng (ms): độ trễ server→bot của một frame.
    pub one_way_delay_ms: u64,
    /// Chu kỳ thử lại `Auth` sau khi server **trả lời** `AuthFail` (ms).
    pub auth_retry_ms: u64,
    /// Chu kỳ xin lại snapshot kết thúc lượt khi chưa nhận được (ms).
    pub snapshot_retry_ms: u64,
    /// Hạn chờ snapshot kết thúc lượt trước khi bot từ bỏ và ack kết quả (ms).
    pub snapshot_deadline_ms: u64,
    /// Hạn chờ `RoomState` sau khi server đã xác nhận bot đang ở trong phòng (ms).
    ///
    /// Server chỉ đẩy `RoomState` khi phòng đổi; nếu bản đẩy tới bot bị mất và không có
    /// thay đổi phòng nào sau đó, bot rời phòng đó rồi vào lại để lấy lại `RoomState`.
    pub room_state_deadline_ms: u64,
}

/// Một bot trong harness QA.
///
/// Trait này là điểm thay thế cho bot dùng module net của `boom-client` (PR #89).
pub trait BotClient {
    /// Tên bot.
    fn name(&self) -> &str;
    /// Xử lý một frame nhận từ server; trả về các frame bot muốn gửi lại.
    fn on_frame(&mut self, now_ms: u64, frame: &[u8]) -> Result<Vec<Vec<u8>>, BotError>;
    /// Bot được gọi định kỳ để gửi lệnh chủ động (ready, start, input, ping).
    fn on_tick(&mut self, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError>;
    /// Số liệu bot đã ghi.
    fn stats(&self) -> &BotStats;
    /// Pha hiện tại của bot (harness dùng để biết bot đã auth chưa).
    fn phase(&self) -> BotPhase;
    /// Bắt đầu lại luồng phiên (dùng khi harness mở lại kết nối cho bot).
    fn reset(&mut self, player: u64);
}

/// Cách harness **dựng bot** cho một lần chạy — điểm thay thế cho [`DirectDtoBot`].
///
/// Harness không tự dựng bot: nó gọi factory này cho từng bot của lần chạy. Một
/// implementation khác (ví dụ bot dự đoán state dùng module net của `boom-client`) chỉ cần
/// cài factory riêng rồi gọi [`crate::harness::run_with`] — **không cần sửa harness**.
///
/// `BotConfig` mà factory nhận là cấu hình harness đã tính sẵn cho bot đó (tên, tài khoản,
/// `PlayerId`, vai trò host, thế giới, profile rules, nhịp input/ping…).
pub trait BotFactory {
    /// Dựng một bot theo cấu hình mà harness tính sẵn cho bot đó.
    fn create(&self, config: &BotConfig) -> Box<dyn BotClient>;
}

/// Factory mặc định: predictor/API phòng client chung, lifecycle qua DTO.
#[derive(Clone, Copy, Debug, Default)]
pub struct DirectDtoBotFactory;

impl BotFactory for DirectDtoBotFactory {
    fn create(&self, config: &BotConfig) -> Box<dyn BotClient> {
        Box::new(DirectDtoBot::new(config.clone()))
    }
}

/// Pha của bot trong luồng phiên.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BotPhase {
    /// Chưa gửi `Hello`.
    Handshake,
    /// Đã gửi `Hello`, chờ `Welcome`.
    AwaitingWelcome,
    /// Đã gửi `Auth`, chờ `AuthOk`.
    AwaitingAuth,
    /// Đã auth, đang tìm phòng.
    Lobby,
    /// Đã vào phòng, chờ start.
    Room,
    /// Đã start, chờ lượt bắt đầu.
    Loading,
    /// Đang chơi.
    Playing,
    /// Lượt kết thúc, chờ về phòng.
    Results,
}

/// Bot dự đoán bằng API client chung; auth/phòng qua DTO `boom-proto`.
pub struct DirectDtoBot {
    config: BotConfig,
    predictor: Option<Predictor>,
    flow: RoomFlow,
    /// Lượt đã rời: phiên mới chỉ tham gia từ lượt sau (SCOPE_V1).
    abandoned_round: Option<u64>,
    /// Vị trí dự đoán theo tick để so với authority tại chính tick đó.
    predicted_positions: BTreeMap<u32, (i32, i32)>,
    phase: BotPhase,
    limits: boom_proto::ProtocolLimits,
    rng: Rng,
    stats: BotStats,
    request_seq: u64,
    /// Định danh người chơi **trên dây** (`account_id` do server cấp trong `AuthOk`).
    ///
    /// Mọi trường `player` trên dây (`RoomState`, `RoundStart.roster`) mang giá trị này
    /// (ADR 0009 Addendum B §B.2), **không** phải `PlayerId` của kết nối: bot học nó từ
    /// `AuthOk` và dùng để biết mình là host và slot của mình trong phòng.
    account_id: Option<u64>,
    /// Phòng bot đang ở (ID public).
    room_id: Option<u64>,
    /// Phòng bot đã gửi `Join`; `None` = chưa gửi `Join` nào.
    join_requested: Option<u64>,
    /// Mốc (ms) gửi `Join` gần nhất — `Join` bị mất thì phải gửi lại sau `retry_ms`.
    join_requested_at_ms: u64,
    /// Đã gửi `Create` (bot giữ vai trò tạo phòng).
    create_requested: bool,
    /// Mốc (ms) gửi `Create` gần nhất — `Create` bị mất thì phải gửi lại sau `retry_ms`.
    create_requested_at_ms: u64,
    /// Server đã xác nhận bot đang ở trong một phòng (`PlayerAlreadyInRoom`) nhưng bot
    /// chưa nhận `RoomState` nên chưa biết mình ở phòng nào.
    ///
    /// Cờ này chặn vòng lặp `Join` → `PlayerAlreadyInRoom` → `Join`; bot chờ `RoomState`
    /// và nếu quá hạn thì rời phòng để vào lại từ đầu.
    join_confirmed: bool,
    /// Mốc (ms) server xác nhận bot đang ở trong phòng.
    join_confirmed_at_ms: u64,
    /// Bot đã thấy mình ready trong `RoomState` gần nhất.
    self_ready: bool,
    /// `RoomState` gần nhất báo mọi thành viên đủ ready.
    all_ready_seen: bool,
    /// `RoomState` gần nhất báo bot là host của phòng.
    is_room_host: bool,
    start_sent: bool,
    room_members: usize,
    /// Lượt hiện tại (ID public của lượt).
    round_id: Option<u64>,
    /// Lượt bot đã gửi `LoadingAck`.
    loading_acked: Option<u64>,
    /// Sequence input của lượt hiện tại.
    seq: u32,
    /// Slot người chơi **trong phòng** do server cấp (chưa biết = `None`).
    slot: Option<u8>,
    last_snapshot: Option<(u32, u64)>,
    last_snapshot_seq: u32,
    last_input_at_ms: u64,
    last_ping_at_ms: u64,
    hello_sent_at_ms: u64,
    /// Mốc (ms) gửi `SetReady` gần nhất — dùng để làm tươi view phòng định kỳ.
    ready_sent_at_ms: u64,
    start_sent_at_ms: u64,
    lobby_requested_at_ms: u64,
    next_auth_at_ms: u64,
    /// Mốc (ms) lần gửi `Auth` gần nhất trong phiên này.
    auth_sent_at_ms: u64,
    auth_op: AuthOp,
    /// Đã gửi `Auth` ít nhất một lần trong phiên này.
    auth_sent: bool,
    /// Server đã trả lời `AuthFail`: được phép gửi lại `Auth` đúng một lần nữa.
    auth_retry_allowed: bool,
    /// Ping đang chờ `Pong`, theo nonce: `nonce → mốc gửi (ms)`.
    ///
    /// Giữ **nhiều** ping cùng lúc: mạng cố ý mất/trùng/đảo thứ tự frame, nên `Pong` của
    /// một ping có thể tới sau khi bot đã gửi ping mới (thậm chí tới hai lần). Mỗi ping giữ
    /// mốc riêng ⇒ `Pong` ghép đúng ping của nó, và `Pong` mất không làm hỏng ping sau.
    pending_pings: BTreeMap<u64, u64>,
    /// Nonce của ping tiếp theo: tăng đơn điệu, **không dùng lại** (kể cả sau khi mở lại
    /// phiên) để `Pong` của phiên cũ không bao giờ khớp nhầm ping mới.
    ping_nonce: u64,
    /// Lượt nào bot đang giữ `ResultAck`, chờ state cuối lượt.
    pending_result_ack: Option<u64>,
    received_results: BTreeSet<u64>,
    result_request_at_ms: u64,
    result_retry_ms: u64,
    /// Các lượt đã biết kết thúc (chống đếm hai lần khi frame trùng/đảo thứ tự).
    finished_rounds: BTreeSet<u64>,
    /// Các lượt đã gửi `ResultAck`.
    acked_rounds: BTreeSet<u64>,
    /// Mốc (ms) bot nhận `RoundResult` hoặc snapshot `Phase::Ended`.
    round_end_at_ms: u64,
    /// Mốc (ms) bot xin snapshot kết thúc lượt gần nhất.
    snapshot_request_at_ms: u64,
    /// Mốc (ms) bot gửi `LoadingAck` gần nhất (để gửi lại khi frame bị mất).
    loading_ack_at_ms: u64,
}

impl DirectDtoBot {
    /// Dựng bot theo cấu hình.
    pub fn new(config: BotConfig) -> Self {
        let mut rng = Rng::new(config.seed);
        // Rút trước vài số để seed của bot khác nhau vẫn khác nhau ngay từ đầu.
        let _ = rng.next_u64();
        Self {
            config,
            predictor: None,
            flow: RoomFlow::new(),
            abandoned_round: None,
            predicted_positions: BTreeMap::new(),
            phase: BotPhase::Handshake,
            limits: boom_proto::ProtocolLimits::v1(),
            rng,
            stats: BotStats::default(),
            request_seq: 0,
            account_id: None,
            room_id: None,
            join_requested: None,
            join_requested_at_ms: 0,
            create_requested: false,
            create_requested_at_ms: 0,
            join_confirmed: false,
            join_confirmed_at_ms: 0,
            self_ready: false,
            all_ready_seen: false,
            is_room_host: false,
            start_sent: false,
            room_members: 0,
            round_id: None,
            loading_acked: None,
            seq: 0,
            slot: None,
            last_snapshot: None,
            last_snapshot_seq: 0,
            last_input_at_ms: 0,
            last_ping_at_ms: 0,
            hello_sent_at_ms: 0,
            ready_sent_at_ms: 0,
            start_sent_at_ms: 0,
            lobby_requested_at_ms: 0,
            next_auth_at_ms: 0,
            auth_sent_at_ms: 0,
            auth_op: AuthOp::Register,
            auth_sent: false,
            auth_retry_allowed: false,
            pending_pings: BTreeMap::new(),
            ping_nonce: 0,
            pending_result_ack: None,
            received_results: BTreeSet::new(),
            result_request_at_ms: 0,
            result_retry_ms: 0,
            finished_rounds: BTreeSet::new(),
            acked_rounds: BTreeSet::new(),
            round_end_at_ms: 0,
            snapshot_request_at_ms: 0,
            loading_ack_at_ms: 0,
        }
    }

    /// Bắt đầu lại luồng phiên sau khi harness mở lại kết nối.
    ///
    /// `ping_nonce` **không** được đặt lại: nonce phải đơn điệu trong cả lần chạy để
    /// `Pong` của phiên cũ (nếu còn sót) không khớp nhầm ping của phiên mới. `account_id`
    /// cũng giữ nguyên: mở lại phiên vẫn là **cùng một tài khoản** (`AuthOk` mới sẽ xác nhận
    /// lại), còn `slot` thì phải học lại từ roster của lượt mới.
    fn reset_state(&mut self) {
        self.abandoned_round = self.round_id.or(self.abandoned_round);
        self.predictor = None;
        self.flow = RoomFlow::new();
        self.predicted_positions.clear();
        self.phase = BotPhase::Handshake;
        // Request ID thuộc account: lobby giữ mốc dedup cả sau disconnect.
        self.room_id = None;
        self.join_requested = None;
        self.join_requested_at_ms = 0;
        self.create_requested = false;
        self.create_requested_at_ms = 0;
        self.join_confirmed = false;
        self.join_confirmed_at_ms = 0;
        self.self_ready = false;
        self.all_ready_seen = false;
        self.is_room_host = false;
        self.start_sent = false;
        self.room_members = 0;
        self.round_id = None;
        self.loading_acked = None;
        self.seq = 0;
        self.slot = None;
        self.last_snapshot = None;
        self.last_snapshot_seq = 0;
        self.last_input_at_ms = 0;
        self.hello_sent_at_ms = 0;
        self.ready_sent_at_ms = 0;
        self.start_sent_at_ms = 0;
        self.lobby_requested_at_ms = 0;
        self.next_auth_at_ms = 0;
        self.auth_sent_at_ms = 0;
        self.auth_op = if self.stats.account_id.is_some() {
            AuthOp::Login
        } else {
            AuthOp::Register
        };
        self.auth_sent = false;
        self.auth_retry_allowed = false;
        self.pending_pings.clear();
        self.pending_result_ack = None;
        self.round_end_at_ms = 0;
        self.snapshot_request_at_ms = 0;
        self.loading_ack_at_ms = 0;
    }

    fn encode<M: Message>(&self, message: &M) -> Result<Vec<u8>, BotError> {
        encode_frame(message, &self.limits).map_err(|err| BotError::Proto(err.to_string()))
    }

    fn next_request(&mut self) -> u64 {
        self.request_seq += 1;
        self.request_seq
    }

    /// Đã qua `Welcome` chưa — trước đó server chỉ nhận `Hello` (ADR 0009 §1).
    fn handshaken(&self) -> bool {
        !matches!(self.phase, BotPhase::Handshake | BotPhase::AwaitingWelcome)
    }

    /// In dòng chẩn đoán khi đặt `BOOM_QA_TRACE=1` (không ảnh hưởng báo cáo).
    fn trace(&self, message: &str) {
        if std::env::var_os("BOOM_QA_TRACE").is_some() {
            eprintln!("[{}] {message}", self.config.name);
        }
    }

    fn room_command(&mut self, action: RoomAction) -> Result<Vec<u8>, BotError> {
        let request = self.next_request();
        self.trace(&format!("-> RoomCommand req={request} {action:?}"));
        self.encode(&RoomCommand { request, action })
    }

    /// Gửi `Auth` nếu chưa gửi lần nào trong phiên này, hoặc server vừa trả `AuthFail`.
    ///
    /// `Welcome` trùng (mạng nhân bản frame) không được sinh `Auth` thứ hai: server từ chối và
    /// đóng phiên (ADR 0009 Addendum A §A.1).
    fn maybe_send_auth(&mut self, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        if self.auth_sent && !self.auth_retry_allowed {
            return Ok(Vec::new());
        }
        self.auth_sent = true;
        self.auth_retry_allowed = false;
        self.auth_sent_at_ms = now_ms;
        self.next_auth_at_ms = now_ms.saturating_add(self.config.auth_retry_ms);
        self.trace(&format!(
            "-> Auth op={:?} user={}",
            self.auth_op, self.config.username
        ));
        let frame = self.encode(&Auth {
            op: self.auth_op,
            username: self.config.username.clone(),
            password: self.config.password.clone(),
        })?;
        self.phase = BotPhase::AwaitingAuth;
        Ok(vec![frame])
    }

    fn on_welcome(&mut self, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        // Tài khoản đã nhận AuthOk dùng Login khi mở phiên mới. Welcome trùng giữ op.
        if !self.auth_sent {
            self.auth_op = if self.stats.account_id.is_some() {
                AuthOp::Login
            } else {
                AuthOp::Register
            };
        }
        self.maybe_send_auth(now_ms)
    }

    /// `AuthFail`: bot thử lại bằng `Login` nếu server cho phép (ADR 0009 Addendum A §A.1).
    ///
    /// Bot **không** bỏ phiên vì một `AuthFail`: mọi lý do đều có đường thử lại, và lý do
    /// `AccountHasActiveSession` chỉ giải phóng được khi mở phiên mới — việc đó thuộc
    /// harness (bot kẹt ở bước auth sẽ được mở lại phiên).
    fn on_auth_fail(&mut self, reason: AuthFailReason, now_ms: u64) {
        self.stats.auth_failed += 1;
        self.trace(&format!("<- AuthFail {reason:?}"));
        match reason {
            AuthFailReason::UsernameUnavailable | AuthFailReason::RegistrationClosed => {
                // Tài khoản đã có (lần chạy trước, phiên đã mở lại, hoặc đăng ký bị đóng):
                // đăng nhập.
                self.auth_op = AuthOp::Login;
                self.auth_retry_allowed = true;
                self.next_auth_at_ms = now_ms;
            }
            AuthFailReason::InvalidCredentials => {
                // Tài khoản chưa tồn tại (hoặc mật khẩu không khớp): đăng ký lại.
                self.auth_op = AuthOp::Register;
                self.auth_retry_allowed = true;
                self.next_auth_at_ms = now_ms.saturating_add(self.config.auth_retry_ms);
            }
            AuthFailReason::AccountHasActiveSession | AuthFailReason::RateLimited => {
                // Phiên cũ còn sống hoặc vượt rate limit: chờ rồi thử lại.
                self.auth_op = AuthOp::Login;
                self.auth_retry_allowed = true;
                self.next_auth_at_ms = now_ms.saturating_add(self.config.auth_retry_ms);
            }
            AuthFailReason::InvalidInput | AuthFailReason::Unavailable => {
                // Lỗi đầu vào hoặc lỗi phía server: thử lại sau, không bỏ phiên.
                self.auth_retry_allowed = true;
                self.next_auth_at_ms = now_ms.saturating_add(self.config.auth_retry_ms);
            }
        }
    }

    /// `AuthOk`: bot học **account_id** do server cấp và xin danh sách sảnh.
    ///
    /// `account_id` là định danh người chơi trên dây (mọi trường `player` của `RoomState` /
    /// `RoundStart.roster` mang giá trị này, ADR 0009 Addendum B §B.2): bot cần nó để biết
    /// mình có phải host và slot của mình trong phòng.
    fn on_auth_ok(&mut self, now_ms: u64, auth: &AuthOk) -> Result<Vec<Vec<u8>>, BotError> {
        self.auth_retry_allowed = false;
        self.account_id = Some(auth.account_id);
        self.stats.account_id = Some(auth.account_id);
        self.trace(&format!("<- AuthOk account {}", auth.account_id));
        let frame = self.encode(&LobbyListRequest {})?;
        self.phase = BotPhase::Lobby;
        self.lobby_requested_at_ms = now_ms;
        Ok(vec![frame])
    }

    /// Bot vào phòng: vào phòng đã có nếu có; nếu chưa có phòng nào thì bot được giao vai
    /// trò tạo phòng.
    ///
    /// `Join`/`Create` là **lệnh có thể bị mất** (hồ sơ mạng SCOPE_V1 §7.6), nên bot chỉ
    /// khoá lệnh trong `retry_ms`: sau đó gửi lại. Lệnh lặp là idempotent ở phía server
    /// (đã ở trong phòng thì trả `PlayerAlreadyInRoom`), nên gửi lại không làm hỏng state.
    fn on_lobby(&mut self, list: &LobbyList, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        if self.room_id.is_some() {
            return Ok(Vec::new());
        }
        if self.join_confirmed {
            // Server đã xác nhận bot đang ở trong một phòng; bot chờ `RoomState` để biết
            // mình ở phòng nào (hạn chờ xử lý ở `on_tick`).
            return Ok(Vec::new());
        }
        if let Some(room) = list
            .rooms
            .iter()
            .find(|room| room.phase == boom_proto::RoomPhase::Waiting)
        {
            let retry_due = self.join_requested != Some(room.room_id)
                || now_ms.saturating_sub(self.join_requested_at_ms) >= self.config.retry_ms;
            if !retry_due {
                return Ok(Vec::new());
            }
            self.join_requested = Some(room.room_id);
            self.join_requested_at_ms = now_ms;
            let frame = self.room_command(RoomAction::Join {
                room: room.room_id,
                character: RoomCharacter::C01,
            })?;
            return Ok(vec![frame]);
        }
        // Chưa có phòng nào: bot được giao vai trò tạo phòng.
        let retry_due = !self.create_requested
            || now_ms.saturating_sub(self.create_requested_at_ms) >= self.config.retry_ms;
        if self.config.is_host && list.rooms.is_empty() && retry_due {
            self.create_requested = true;
            self.create_requested_at_ms = now_ms;
            let frame = self.room_command(RoomAction::Create {
                character: RoomCharacter::C00,
            })?;
            return Ok(vec![frame]);
        }
        Ok(Vec::new())
    }

    /// Ghi nhận slot người chơi trong phòng từ `RoomState` hoặc roster của `RoundStart`.
    ///
    /// `members` mang **account_id** (định danh người chơi trên dây, ADR 0009 Addendum B
    /// §B.2), **không** phải `PlayerId` của kết nối: server bind session rồi mới gán slot.
    fn learn_slot(&mut self, members: &[(u64, u8)]) {
        let Some(account_id) = self.account_id else {
            return;
        };
        if let Some((_, slot)) = members.iter().find(|(player, _)| *player == account_id) {
            self.slot = Some(*slot);
        }
    }

    fn on_room_state(&mut self, state: &RoomState, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        if state
            .round
            .as_ref()
            .is_some_and(|round| self.abandoned_round.is_some_and(|old| round.id <= old))
        {
            return Ok(Vec::new());
        }
        self.room_id = Some(state.room_id);
        // Bot đã biết mình ở phòng nào: bỏ cờ "server đã xác nhận nhưng chưa biết phòng".
        self.join_confirmed = false;
        self.join_confirmed_at_ms = 0;
        self.room_members = state.members.len();
        // `host` mang **account_id** của host (ADR 0009 Addendum B §B.2), không phải
        // `PlayerId` của kết nối: so với account_id của chính bot.
        self.is_room_host = self.account_id == Some(state.host);
        let members: Vec<(u64, u8)> = state
            .members
            .iter()
            .map(|member| (member.player, member.slot))
            .collect();
        self.learn_slot(&members);
        // `self_ready`/`all_ready_seen` phản ánh **trạng thái quan sát được** ở `RoomState` này.
        // Không được đặt lại chúng sau khi tính: nếu đặt lại, host không bao giờ thấy
        // "phòng đủ người ready" nên không gửi `Start` (phòng treo vĩnh viễn).
        self.self_ready = self.account_id.is_some_and(|account_id| {
            state
                .members
                .iter()
                .any(|member| member.player == account_id && member.ready)
        });
        self.all_ready_seen = state.members.len() == self.config.expected_members
            && state.members.iter().all(|member| member.ready);
        self.trace(&format!(
            "room {} phase {:?} members {} round {:?} host {} self_ready {} all_ready {}",
            state.room_id,
            state.phase,
            state.members.len(),
            state.round.as_ref().map(|r| r.id),
            self.is_room_host,
            self.self_ready,
            self.all_ready_seen
        ));

        let mut out = Vec::new();
        match &state.round {
            Some(round) => {
                if self.is_stale_round(round.id) {
                    // Lượt cũ đến trễ: bỏ qua, không đụng trạng thái lượt hiện tại.
                    return Ok(out);
                }
                // Lượt đã chạy: bot báo đã sẵn sàng (`LoadingAck`) nếu chưa báo cho lượt này.
                // `RoundStart` có thể bị mất nên bot không chỉ dựa vào nó.
                if self.begin_round(round.id) {
                    // `RoundStart` có thể bị mất: `RoomState` cũng cho biết lượt, nên bot
                    // chuyển sang pha `Loading` ngay khi biết ID lượt.
                    self.phase = BotPhase::Loading;
                }
                if self.loading_acked != Some(round.id) {
                    self.loading_acked = Some(round.id);
                    self.loading_ack_at_ms = now_ms;
                    let frame = self.encode(&LoadingAck { round_id: round.id })?;
                    out.push(frame);
                }
            }
            None => {
                // Phòng đang chờ (chưa có lượt nào chạy): bot ở pha Room, sẵn sàng lượt sau.
                // Dọn trạng thái **lượt** (không đụng mốc ID lượt đơn điệu) để lượt sau không
                // dùng lại sequence cũ.
                self.loading_acked = None;
                self.loading_ack_at_ms = 0;
                self.pending_result_ack = None;
                self.start_sent = false;
                // Lượt bị hủy (barrier loading quá hạn) ⇒ phòng về waiting: bot phải về
                // pha Room để ready/start lại. Lượt **đang chơi** thì giữ nguyên pha: lúc
                // đó `RoomState` không có lượt chỉ là frame cũ đến trễ.
                if !matches!(self.phase, BotPhase::Playing) {
                    self.phase = BotPhase::Room;
                }
            }
        }
        if self.phase == BotPhase::Room
            && self.all_ready_seen
            && self.is_room_host
            && !self.start_sent
        {
            self.start_sent = true;
            self.start_sent_at_ms = now_ms;
            let frame = self.room_command(RoomAction::Start)?;
            out.push(frame);
        }
        Ok(out)
    }

    /// Lượt `round_id` đã cũ hơn lượt hiện tại không (frame đến trễ / frame nhân bản cũ).
    fn is_stale_round(&self, round_id: u64) -> bool {
        matches!(self.round_id, Some(current) if round_id < current)
    }

    /// Ghi nhận lượt mới; trả về true nếu đây là lượt mới (cần dọn trạng thái lượt cũ).
    ///
    /// ID lượt public của server **tăng đơn điệu** (authority cấp `next_round` tăng dần để
    /// result/replay không trùng), nên ID ≤ lượt hiện tại là lượt cũ hoặc lượt trùng: frame đến
    /// trễ (đảo thứ tự) hoặc frame nhân bản không được xử lý như lượt mới — nếu không, bot đặt
    /// lại sequence input và gửi lại `seq` cũ với payload khác ⇒ server từ chối
    /// `BadSequence` và cả lượt kẹt.
    fn begin_round(&mut self, round_id: u64) -> bool {
        if let Some(current) = self.round_id
            && round_id <= current
        {
            return false;
        }
        self.round_id = Some(round_id);
        self.predictor = None;
        self.predicted_positions.clear();
        self.stats.round_id = round_id;
        // Sequence input đánh số lại theo từng lượt (ADR 0009 §3.1).
        self.seq = 0;
        self.stats.sent.clear();
        self.loading_acked = None;
        self.last_snapshot = None;
        self.last_snapshot_seq = 0;
        self.pending_result_ack = None;
        self.result_request_at_ms = 0;
        self.result_retry_ms = 0;
        self.start_sent = false;
        true
    }

    fn on_round_start(
        &mut self,
        start: &RoundStart,
        now_ms: u64,
    ) -> Result<Vec<Vec<u8>>, BotError> {
        if self
            .abandoned_round
            .is_some_and(|old| start.round_id <= old)
        {
            return Ok(Vec::new());
        }
        self.flow.set_my_player(
            self.account_id
                .ok_or_else(|| BotError::Unexpected("RoundStart trước AuthOk".to_owned()))?,
        );
        let expected = RoundExpectation {
            profile_id: self.config.rules_profile_id.clone(),
            rules_hash: self.config.rules_hash,
            content_sha256: self.config.content_sha256,
            mode: self.config.world.mode().id().to_owned(),
        };
        let outcome = self
            .flow
            .on_round_start(start.clone(), &expected)
            .map_err(|err| BotError::Sync(err.to_string()))?;
        if matches!(
            outcome,
            RoundStartOutcome::Stale | RoundStartOutcome::Duplicate
        ) {
            return Ok(Vec::new());
        }
        let round_id = start.round_id;
        self.trace(&format!(
            "RoundStart round={round_id} roster={}",
            start.roster.len()
        ));
        let roster: Vec<(u64, u8)> = start
            .roster
            .iter()
            .map(|member| (member.player, member.slot))
            .collect();
        if self.is_stale_round(round_id) {
            // Lượt cũ đến trễ: bỏ qua, không đụng trạng thái lượt hiện tại.
            return Ok(Vec::new());
        }
        self.learn_slot(&roster);
        if self.begin_round(round_id) {
            self.phase = BotPhase::Loading;
        } else if !matches!(self.phase, BotPhase::Loading | BotPhase::Playing) {
            // Frame trùng (mạng nhân bản) của lượt đang chơi: không hạ pha bot.
            self.phase = BotPhase::Loading;
        }
        if self.loading_acked != Some(round_id) {
            self.loading_acked = Some(round_id);
            self.loading_ack_at_ms = now_ms;
            let frame = self.encode(&LoadingAck { round_id })?;
            return Ok(vec![frame]);
        }
        Ok(Vec::new())
    }

    /// Ghi nhận lượt đã kết thúc (từ `RoundResult` hoặc snapshot `Phase::Ended`).
    ///
    /// Trả về true nếu đây là lần đầu bot biết lượt này kết thúc.
    fn note_round_finished(&mut self, round_id: u64) -> bool {
        if !self.finished_rounds.insert(round_id) {
            return false;
        }
        self.stats.rounds_completed += 1;
        true
    }

    /// Gửi `ResultAck` cho lượt đã kết thúc (một lần cho mỗi lượt).
    fn release_result_ack(&mut self, round_id: u64) -> Result<Vec<Vec<u8>>, BotError> {
        if !self.acked_rounds.insert(round_id) {
            return Ok(Vec::new());
        }
        self.pending_result_ack = None;
        self.stats.results_acked += 1;
        let frame = self.encode(&ResultAck { round_id })?;
        Ok(vec![frame])
    }

    /// Bot đã có state cuối lượt thì được ack kết quả.
    fn ack_if_have_state(&mut self, round_id: u64) -> Result<Vec<Vec<u8>>, BotError> {
        if !self.received_results.contains(&round_id)
            || !self.stats.round_hashes.contains_key(&round_id)
        {
            return Ok(Vec::new());
        }
        self.release_result_ack(round_id)
    }

    fn on_snapshot(&mut self, snapshot: &Snapshot, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        self.stats.snapshots += 1;
        let round_id = u64::from(snapshot.round_id);
        if self.abandoned_round.is_some_and(|old| round_id <= old) {
            return Ok(Vec::new());
        }
        if self.is_stale_round(round_id) {
            // Snapshot của lượt cũ (đến trễ): bỏ qua, không đụng state hiện tại.
            return Ok(Vec::new());
        }
        if self.begin_round(round_id) {
            // Snapshot tới trước `RoomState`/`RoundStart`: bot vẫn biết lượt từ snapshot.
            self.phase = BotPhase::Loading;
        }
        if let Some(slot) = self.slot {
            if self.predictor.is_none() {
                self.predictor = Some(
                    Predictor::new(self.config.world.clone(), slot)
                        .map_err(|err| BotError::Sync(err.to_string()))?,
                );
            }
            let predictor = self.predictor.as_mut().expect("predictor");
            let outcome = predictor
                .apply_snapshot(snapshot)
                .map_err(|err| BotError::Sync(err.to_string()))?;
            if !matches!(outcome, SnapshotOutcome::Applied { .. }) {
                return Ok(Vec::new());
            }
            if let Some((x, y)) = self.predicted_positions.remove(&snapshot.tick)
                && let Some(player) = snapshot.players.iter().find(|p| p.id == slot)
            {
                let amplitude = i64::from(x).abs_diff(i64::from(player.x))
                    + i64::from(y).abs_diff(i64::from(player.y));
                self.stats.correction_mpx.push(amplitude);
                if amplitude > 0 {
                    self.stats.corrections += 1;
                }
                self.stats.max_correction_mpx = self
                    .stats
                    .max_correction_mpx
                    .max(i32::try_from(amplitude).unwrap_or(i32::MAX));
            }
            self.predicted_positions
                .retain(|tick, _| *tick > snapshot.tick);
        }
        self.round_id = Some(round_id);
        self.last_snapshot = Some((snapshot.tick, now_ms));
        self.last_snapshot_seq = snapshot.snapshot_seq;
        // Đồng bộ sequence với server: server cho biết seq cuối nó đã giải quyết.
        if let Some(slot) = self.slot
            && let Some(ack) = snapshot.acks.iter().find(|ack| ack.player == slot)
        {
            self.seq = self.seq.max(ack.last_seq);
        }
        match snapshot.phase {
            boom_proto::Phase::Playing => {
                // **Phân biệt barrier loading với lượt đã chơi**: state core được dựng
                // với phase `Playing` ngay từ đầu lượt, nên server cũng gửi snapshot
                // `Playing` khi vẫn đang chờ barrier. Chỉ khi **tick tiến** (tick > 0) thì
                // server mới thật sự chạy tick — lúc đó bot mới được gửi input.
                //
                // Snapshot "Playing" tới trễ (đảo thứ tự) không được hạ pha bot đang ở
                // `Results`.
                if snapshot.tick > 0 {
                    if !matches!(self.phase, BotPhase::Results) {
                        self.phase = BotPhase::Playing;
                    }
                } else if matches!(self.phase, BotPhase::Room | BotPhase::Loading) {
                    self.phase = BotPhase::Loading;
                }
                Ok(Vec::new())
            }
            boom_proto::Phase::Ended(_) => {
                // Lượt kết thúc: dựng lại state để so hash với server (kiểm hội tụ).
                if !self.stats.round_hashes.contains_key(&round_id) {
                    let state = boom_sync::from_snapshot(snapshot, &self.config.world)
                        .map_err(|err| BotError::Sync(err.to_string()))?;
                    self.trace(&format!(
                        "hash round {round_id} = {} (server tick {})",
                        state.hash(),
                        snapshot.tick
                    ));
                    self.stats.round_hashes.insert(round_id, state.hash());
                }
                self.note_round_finished(round_id);
                self.phase = BotPhase::Results;
                self.round_end_at_ms = now_ms;
                self.ack_if_have_state(round_id)
            }
        }
    }

    fn on_round_result(
        &mut self,
        result: &RoundResult,
        now_ms: u64,
    ) -> Result<Vec<Vec<u8>>, BotError> {
        let round_id = result.round_id;
        if self.abandoned_round.is_some_and(|old| round_id <= old) {
            return Ok(Vec::new());
        }
        if self.is_stale_round(round_id) {
            // Kết quả của lượt cũ đến trễ: bỏ qua.
            return Ok(Vec::new());
        }
        self.received_results.insert(round_id);
        self.round_end_at_ms = now_ms;
        self.note_round_finished(round_id);
        self.phase = BotPhase::Results;
        if self.stats.round_hashes.contains_key(&round_id) {
            // Snapshot kết thúc lượt đã tới trước `RoundResult`: ack ngay.
            return self.release_result_ack(round_id);
        }
        // Chưa có state cuối lượt: giữ `ResultAck`, xin snapshot.
        // Server xoá lượt khi mọi người ack, nên phải có state cuối lượt **trước** khi ack.
        self.pending_result_ack = Some(round_id);
        self.snapshot_request_at_ms = now_ms;
        self.trace(&format!(
            "RoundResult round={round_id}: giữ ack, xin snapshot"
        ));
        let request = self.encode(&boom_proto::SnapshotRequest {
            round_id: u32::try_from(round_id).unwrap_or(u32::MAX),
            last_snapshot_seq: self.last_snapshot_seq,
        })?;
        Ok(vec![request])
    }

    fn on_input_rejected(&mut self, rejected: &InputRejected) {
        self.stats.input_rejected += 1;
        if self.round_id == Some(u64::from(rejected.round_id))
            && let Some(predictor) = self.predictor.as_mut()
        {
            predictor.acknowledge(rejected.applied_seq);
        }
        // Server cho biết seq cuối nó đã giải quyết: đồng bộ để không vượt `MAX_INPUT_GAP`.
        self.seq = self.seq.max(rejected.applied_seq);
        self.trace(&format!(
            "input bị từ chối: {:?} seq {} applied {}",
            rejected.reason, rejected.first_seq, rejected.applied_seq
        ));
    }

    fn on_room_rejected(&mut self, rejected: &RoomRejected, now_ms: u64) {
        self.stats.room_rejected += 1;
        self.trace(&format!(
            "lệnh phòng bị từ chối: request {} {:?}",
            rejected.request, rejected.reason
        ));
        match rejected.reason {
            boom_proto::RoomRejectReason::PlayerAlreadyInRoom => {
                // Đã ở trong phòng: lệnh `Join`/`Create` bị lặp do frame trùng, hoặc frame
                // gốc tới trễ. Server **xác nhận** bot đang ở trong một phòng, nên bot dừng
                // gửi `Join` (tránh vòng lặp `Join` → `PlayerAlreadyInRoom` → `Join`) và
                // chờ `RoomState` để biết mình ở phòng nào.
                self.join_confirmed = true;
                self.join_confirmed_at_ms = now_ms;
                self.join_requested = None;
                self.create_requested = false;
            }
            boom_proto::RoomRejectReason::NotAllReady => {
                // Host thử start lại ở vòng sau khi thấy phòng đủ ready.
                self.start_sent = false;
            }
            boom_proto::RoomRejectReason::AlreadyStarted => {
                // Lượt đã chạy: `Start` gửi lại (do frame bị mất) là vô hại, không cần
                // vào lại phòng.
                self.start_sent = true;
            }
            boom_proto::RoomRejectReason::NotHost => {
                // Không phải host: không được gửi `Start` nữa.
                self.start_sent = true;
                self.is_room_host = false;
            }
            _ => {
                // Lệnh phòng hỏng: cho phép gửi lại ở vòng sau.
                self.join_requested = None;
                self.create_requested = false;
            }
        }
    }

    /// Ghi mẫu RTT cho **đúng** ping mà `Pong` trả lời.
    ///
    /// `Pong` mang nonce của `Ping` (ADR 0009 §2). Hồ sơ mạng cố ý trùng/đảo thứ tự
    /// frame, nên `Pong` của ping cũ có thể tới **sau** `Pong` của ping mới, hoặc tới hai
    /// lần. Vì vậy mỗi `Pong` chỉ được ghi mẫu khi nonce của nó khớp một ping **đang
    /// chờ**; `Pong` lạ (stale, trùng, hoặc của ping đã mất) thì bỏ qua — nếu không, RTT
    /// bị ghi nhầm sang ping khác và mẫu của ping thật bị mất.
    fn on_pong(&mut self, nonce: u64, now_ms: u64) {
        let Some(sent_at) = self.pending_pings.remove(&nonce) else {
            self.trace(&format!(
                "<- Pong nonce {nonce} không khớp ping nào (bỏ qua)"
            ));
            return;
        };
        self.stats.rtt.push(now_ms.saturating_sub(sent_at));
    }

    /// Nonce của ping tiếp theo: tăng đơn điệu, không dùng lại.
    fn next_ping_nonce(&mut self) -> u64 {
        self.ping_nonce = self.ping_nonce.saturating_add(1);
        self.ping_nonce
    }

    /// Bỏ ping quá hạn (mất `Pong` vĩnh viễn) để bảng pending không phình vô hạn.
    ///
    /// Hạn lấy theo chu kỳ ping của hồ sơ: quá [`PING_PENDING_TTL_FACTOR`] chu kỳ mà chưa
    /// có `Pong` thì coi như mất — mẫu RTT của nó không còn giá trị.
    fn prune_pending_pings(&mut self, now_ms: u64) {
        let ttl = self
            .config
            .ping_interval_ms
            .saturating_mul(PING_PENDING_TTL_FACTOR)
            .max(1);
        self.pending_pings
            .retain(|_, sent_at| now_ms.saturating_sub(*sent_at) <= ttl);
    }

    fn hello_frame(&mut self) -> Result<Vec<u8>, BotError> {
        let frame = self.encode(&Hello {
            wire_schema: boom_proto::limits::WIRE_SCHEMA,
            state_schema: boom_proto::limits::WIRE_SCHEMA,
            rules_profile_id: self.config.rules_profile_id.clone(),
            rules_hash: self.config.rules_hash,
            content_sha256: self.config.content_sha256,
            client_build: self.config.name.clone(),
            capabilities: Vec::new(),
        })?;
        self.phase = BotPhase::AwaitingWelcome;
        Ok(frame)
    }

    fn send_ready(&mut self, now_ms: u64) -> Result<Vec<u8>, BotError> {
        self.ready_sent_at_ms = now_ms;
        self.room_command(RoomAction::SetReady { ready: true })
    }

    /// Ước lượng tick authority lúc batch input mới tới server.
    ///
    /// `last_snapshot` là tick server trong snapshot và mốc bot nhận nó. Từ mốc đó cộng số
    /// tick đã trôi (đồng hồ harness), trễ một chiều của hồ sơ và `input_lead_ticks` để
    /// `target_tick` không cũ hơn cửa sổ `INPUT_LATE` của server (ADR 0009 §3.2, §3.5).
    /// Lệnh phải chờ cửa sổ gửi được tính lại `target_tick` ở [`Predictor::window_for_send`]
    /// tại lần gửi đầu, nên không bị trễ thêm do xếp hàng.
    fn input_target_tick(&self, now_ms: u64, fallback_tick: u32) -> u32 {
        let hz = self.config.world.rules().ticks_per_second.max(1);
        let one_way_ms = u32::try_from(self.config.one_way_delay_ms).unwrap_or(u32::MAX);
        let one_way_ticks = boom_core::ms_to_ticks(one_way_ms, hz);
        let Some((snapshot_tick, received_at_ms)) = self.last_snapshot else {
            return fallback_tick;
        };
        let elapsed_ms = u32::try_from(now_ms.saturating_sub(received_at_ms)).unwrap_or(u32::MAX);
        snapshot_tick
            .saturating_add(boom_core::ms_to_ticks(elapsed_ms, hz))
            .saturating_add(one_way_ticks)
            .saturating_add(self.config.input_lead_ticks)
    }

    fn send_input(&mut self, now_ms: u64) -> Result<Option<Vec<u8>>, BotError> {
        let Some(round_id) = self.round_id else {
            return Ok(None);
        };
        let Some(slot) = self.slot else {
            // Chưa biết slot trong phòng: server sẽ từ chối batch, chờ `RoomState`/roster.
            return Ok(None);
        };
        let direction = u8::try_from(self.rng.below(4) + 1).unwrap_or(1);
        let place_bomb = self.rng.below(8) == 0;
        let dir = match direction {
            1 => boom_core::Dir::Up,
            2 => boom_core::Dir::Down,
            3 => boom_core::Dir::Left,
            _ => boom_core::Dir::Right,
        };
        let fallback_tick = self
            .predictor
            .as_ref()
            .and_then(Predictor::predicted_state)
            .map_or(0, |state| state.tick);
        let target_tick = self.input_target_tick(now_ms, fallback_tick);
        let Some(predictor) = self.predictor.as_mut() else {
            return Ok(None);
        };
        let seq = predictor
            .predict_at(boom_core::PlayerInput { dir, place_bomb }, target_tick)
            .map_err(|err| BotError::Sync(err.to_string()))?;
        let Some(seq) = seq else {
            return Ok(None);
        };
        if let Some(state) = predictor.predicted_state()
            && let Some(player) = state.players.iter().find(|p| p.id == slot)
        {
            self.predicted_positions
                .insert(state.tick, (player.x.0, player.y.0));
        }
        let unsent_before: Vec<u32> = predictor
            .pending()
            .iter()
            .filter(|command| !command.sent)
            .map(|command| command.seq)
            .collect();
        let commands = predictor.window_for_send(self.limits.max_commands, target_tick);
        self.seq = seq;
        let batch = InputBatch {
            round_id: u32::try_from(round_id).unwrap_or(u32::MAX),
            player: slot,
            ack_snapshot: self.last_snapshot_seq,
            commands: commands.clone(),
        };
        // Ghi mốc cho **mọi** lệnh lần đầu vào batch này (lệnh có thể đã chờ cửa sổ qua
        // nhiều chu kỳ), không chỉ lệnh vừa tạo: `resolved`/latency đo theo sequence.
        for command in &commands {
            if unsent_before.contains(&command.seq) {
                self.stats.sent.insert(
                    (round_id, command.seq),
                    SentCommand {
                        sent_at_ms: now_ms,
                        target_tick: command.target_tick,
                    },
                );
            }
        }
        self.stats.commands_sent += 1;
        self.last_input_at_ms = now_ms;
        Ok(Some(self.encode(&batch)?))
    }
}

impl BotClient for DirectDtoBot {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn on_frame(&mut self, now_ms: u64, frame: &[u8]) -> Result<Vec<Vec<u8>>, BotError> {
        let (envelope, payload) =
            read_envelope(frame).map_err(|err| BotError::Proto(err.to_string()))?;
        self.trace(&format!("<- {:?} ({} byte)", envelope.kind, frame.len()));
        match envelope.kind {
            MessageKind::Welcome => self.on_welcome(now_ms),
            MessageKind::AuthOk => {
                let auth: AuthOk = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_auth_ok(now_ms, &auth)
            }
            MessageKind::AuthFail => {
                let fail: AuthFail = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_auth_fail(fail.reason, now_ms);
                Ok(Vec::new())
            }
            MessageKind::Reject => {
                let reject: Reject = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                // Server từ chối rồi đóng phiên: bot dừng, harness mở lại phiên nếu cần.
                self.stats.rejected += 1;
                self.trace(&format!(
                    "bị từ chối: {:?} {}",
                    reject.reason, reject.detail
                ));
                Ok(Vec::new())
            }
            MessageKind::LobbyList => {
                let list: LobbyList = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_lobby(&list, now_ms)
            }
            MessageKind::RoomState => {
                let state: RoomState = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_room_state(&state, now_ms)
            }
            MessageKind::RoundStart => {
                let start: RoundStart = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_round_start(&start, now_ms)
            }
            MessageKind::Snapshot => {
                let snapshot: Snapshot = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_snapshot(&snapshot, now_ms)
            }
            MessageKind::RoundResult => {
                let result: RoundResult = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_round_result(&result, now_ms)
            }
            MessageKind::InputRejected => {
                let rejected: InputRejected = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_input_rejected(&rejected);
                Ok(Vec::new())
            }
            MessageKind::RoomRejected => {
                let rejected: RoomRejected = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_room_rejected(&rejected, now_ms);
                Ok(Vec::new())
            }
            MessageKind::Ping => {
                let ping: Ping = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                let frame = self.encode(&Pong(ping.0))?;
                Ok(vec![frame])
            }
            MessageKind::Pong => {
                let pong: Pong = boom_proto::decode_payload(payload)
                    .map_err(|e| BotError::Proto(e.to_string()))?;
                self.on_pong(pong.0, now_ms);
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn on_tick(&mut self, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        let mut out = Vec::new();
        match self.phase {
            BotPhase::Handshake => {
                out.push(self.hello_frame()?);
                self.hello_sent_at_ms = now_ms;
            }
            BotPhase::AwaitingWelcome => {
                // `Welcome` có thể bị mất: gửi lại `Hello` (server trả lại `Welcome`).
                if now_ms.saturating_sub(self.hello_sent_at_ms) >= self.config.retry_ms {
                    out.push(self.hello_frame()?);
                    self.hello_sent_at_ms = now_ms;
                }
            }
            BotPhase::AwaitingAuth => {
                // Chỉ thử lại `Auth` khi server **đã trả lời** `AuthFail`: `Auth` lần hai sau
                // `AuthOk` là lỗi giao thức, server đóng phiên (ADR 0009 Addendum A §A.1).
                // Nếu `AuthOk` bị mất, bot chờ; harness mở phiên mới khi hết hạn.
                if self.auth_retry_allowed && now_ms >= self.next_auth_at_ms {
                    out.extend(self.maybe_send_auth(now_ms)?);
                }
            }
            BotPhase::Lobby => {
                if self.room_id.is_some() {
                    // Đã biết phòng: chờ `RoomState`.
                } else if self.join_confirmed {
                    // Server xác nhận bot đang ở trong phòng nhưng `RoomState` chưa tới.
                    // Server chỉ đẩy `RoomState` khi phòng đổi, nên nếu quá hạn mà vẫn
                    // chưa có, bot rời phòng đó để vào lại từ đầu.
                    if now_ms.saturating_sub(self.join_confirmed_at_ms)
                        >= self.config.room_state_deadline_ms
                    {
                        self.join_confirmed = false;
                        self.join_confirmed_at_ms = 0;
                        self.join_requested = None;
                        self.create_requested = false;
                        self.lobby_requested_at_ms = now_ms;
                        let frame = self.room_command(RoomAction::Leave)?;
                        out.push(frame);
                    }
                } else if now_ms.saturating_sub(self.lobby_requested_at_ms)
                    >= self.config.lobby_retry_ms
                {
                    // Chưa vào phòng: xin lại danh sách sảnh (có thể phòng chưa tồn tại).
                    self.lobby_requested_at_ms = now_ms;
                    let frame = self.encode(&LobbyListRequest {})?;
                    out.push(frame);
                }
            }
            BotPhase::Room => {
                // Gửi lại `SetReady` định kỳ. Lệnh này **idempotent** ở phía server và
                // mọi lệnh phòng thành công đều khiến server đẩy lại `RoomState` cho mọi
                // thành viên — nên đây là cách bot làm **tươi** view phòng. Cần thiết vì
                // `RoomState` là frame đẩy (push): một bản bị mất sẽ khiến bot giữ view
                // cũ mãi mãi (ví dụ host không thấy "phòng đủ ready" nên không start).
                if now_ms.saturating_sub(self.ready_sent_at_ms) >= self.config.retry_ms {
                    out.push(self.send_ready(now_ms)?);
                }
                if self.is_room_host
                    && self.all_ready_seen
                    && now_ms.saturating_sub(self.start_sent_at_ms) >= self.config.retry_ms
                {
                    // Host thử `Start` định kỳ: lệnh `Start` đầu có thể bị mất, bị từ
                    // chối (`NotAllReady`) hoặc frame "phòng đủ ready" tới trễ. Lệnh start
                    // lặp không tạo thêm round (SCOPE_V1 §3); server là bên quyết định
                    // "đủ ready".
                    //
                    // Chỉ start khi bot **đã thấy** phòng đủ thành viên và tất cả ready:
                    // harness chạy đúng số bot của lần chạy, nên start sớm sẽ bỏ sót bot
                    // chưa vào phòng ⇒ bot đó không có state hash của lượt đó.
                    self.start_sent_at_ms = now_ms;
                    self.start_sent = true;
                    let frame = self.room_command(RoomAction::Start)?;
                    out.push(frame);
                }
            }
            BotPhase::Loading => {
                // Barrier loading: `LoadingAck` phải tới server, nên gửi lại định kỳ —
                // frame đầu có thể bị mất và server sẽ hủy lượt khi quá hạn barrier.
                // `LoadingAck` lặp là idempotent (server bỏ qua khi đã vào pha chơi).
                if let Some(round_id) = self.round_id {
                    if self.loading_acked != Some(round_id) {
                        self.loading_acked = Some(round_id);
                        self.loading_ack_at_ms = now_ms;
                        let frame = self.encode(&LoadingAck { round_id })?;
                        out.push(frame);
                    } else if now_ms.saturating_sub(self.loading_ack_at_ms) >= self.config.retry_ms
                    {
                        self.loading_ack_at_ms = now_ms;
                        let frame = self.encode(&LoadingAck { round_id })?;
                        out.push(frame);
                    }
                }
            }
            BotPhase::Playing => {
                if now_ms.saturating_sub(self.last_input_at_ms) >= self.config.input_interval_ms
                    && let Some(frame) = self.send_input(now_ms)?
                {
                    out.push(frame);
                }
            }
            BotPhase::Results => {
                // Đang giữ `ResultAck` chờ state cuối lượt: xin lại snapshot nếu chưa tới.
                if let Some(round_id) = self.pending_result_ack {
                    let waited = now_ms.saturating_sub(self.round_end_at_ms);
                    if waited >= self.config.snapshot_deadline_ms {
                        // Hết hạn: ack kết quả để lượt kết thúc, ghi nhận thiếu hash.
                        self.stats.snapshot_missed += 1;
                        self.trace(&format!(
                            "round {round_id}: không nhận được snapshot kết thúc sau {waited} ms"
                        ));
                        out.extend(self.release_result_ack(round_id)?);
                    } else if now_ms.saturating_sub(self.snapshot_request_at_ms)
                        >= self.config.snapshot_retry_ms
                    {
                        self.snapshot_request_at_ms = now_ms;
                        self.trace(&format!("round {round_id}: xin lại snapshot"));
                        let request = self.encode(&boom_proto::SnapshotRequest {
                            round_id: u32::try_from(round_id).unwrap_or(u32::MAX),
                            last_snapshot_seq: self.last_snapshot_seq,
                        })?;
                        out.push(request);
                    }
                }
            }
        }
        if let Some(round_id) = self.round_id
            && self.finished_rounds.contains(&round_id)
            && !self.received_results.contains(&round_id)
            && now_ms.saturating_sub(self.result_request_at_ms)
                >= self.result_retry_ms.max(self.config.retry_ms)
        {
            out.push(self.encode(&boom_proto::RoundResultRequest { round_id })?);
            self.result_request_at_ms = now_ms;
            self.result_retry_ms = (self.result_retry_ms.max(self.config.retry_ms) * 2).min(2000);
        }
        if self.handshaken()
            && now_ms.saturating_sub(self.last_ping_at_ms) >= self.config.ping_interval_ms
        {
            self.last_ping_at_ms = now_ms;
            let nonce = self.next_ping_nonce();
            self.pending_pings.insert(nonce, now_ms);
            self.prune_pending_pings(now_ms);
            out.push(self.encode(&Ping(nonce))?);
        }
        Ok(out)
    }

    fn stats(&self) -> &BotStats {
        &self.stats
    }

    fn phase(&self) -> BotPhase {
        self.phase
    }

    /// Bắt đầu lại luồng phiên sau khi harness mở lại kết nối.
    fn reset(&mut self, player: u64) {
        self.config.player = player;
        self.reset_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;
    use boom_core::mode_by_id;
    use boom_proto::InputBits;
    use boom_proto::LobbyRoom;
    use boom_proto::dto::room::RoomMember;

    /// Thế giới dùng cho test bot (bản đồ + profile rules tích hợp sẵn của QA).
    fn test_world() -> World {
        let rows = crate::profile::map_rows(crate::profile::builtin_map("qa-village").unwrap());
        let terrain = Terrain::from_ascii(&rows).unwrap();
        let profile =
            RulesProfile::from_toml(crate::profile::builtin_rules("qa-fast").unwrap()).unwrap();
        let rules = boom_core::Rules::from_profile(&profile).unwrap();
        let mode = mode_by_id("battle_ffa").unwrap();
        World::new(terrain, rules, mode).unwrap()
    }

    /// Bot tối thiểu để test luồng phòng.
    fn test_bot() -> DirectDtoBot {
        DirectDtoBot::new(BotConfig {
            name: "bot0".to_owned(),
            username: "qa_bot_test".to_owned(),
            password: "qa-pass-test".to_owned(),
            player: 1,
            is_host: false,
            expected_members: 2,
            seed: 7,
            world: test_world(),
            rules_profile_id: "qa-fast".to_owned(),
            rules_hash: 1,
            content_sha256: [0; 32],
            input_interval_ms: 20,
            input_lead_ticks: 1,
            ping_interval_ms: 1_000,
            retry_ms: 500,
            lobby_retry_ms: 200,
            one_way_delay_ms: 10,
            auth_retry_ms: 1_000,
            snapshot_retry_ms: 100,
            snapshot_deadline_ms: 2_000,
            room_state_deadline_ms: 3_000,
        })
    }

    #[test]
    fn correction_cung_tick_khong_dem_snapshot_trung_va_round_cu() {
        use boom_core::{CharacterStats, PlayerSetup, SimulationState, Stat};
        let mut bot = test_bot();
        bot.slot = Some(0);
        let stats = CharacterStats {
            speed: Stat::new(1, 1),
            bombs: Stat::new(1, 1),
            range: Stat::new(1, 1),
        };
        let state = SimulationState::new_round(
            &bot.config.world,
            &[PlayerSetup { id: 0, stats }, PlayerSetup { id: 1, stats }],
            7,
        )
        .unwrap();
        let ctx = boom_sync::SnapshotContext {
            snapshot_seq: 1,
            round_id: 1,
            state_schema: 1,
            rules_hash: bot.config.world.rules().profile_hash,
            content_epoch: 1,
            acks: vec![],
        };
        let first = boom_sync::to_snapshot(&state, &ctx).unwrap();
        bot.on_snapshot(&first, 0).unwrap();
        // Resend dùng cùng payload cho seq đã gửi, kể cả có input mới.
        let one = bot.send_input(20).unwrap().unwrap();
        let two = bot.send_input(40).unwrap().unwrap();
        let decode = |bytes: &[u8]| -> InputBatch {
            boom_proto::decode_payload(read_envelope(bytes).unwrap().1).unwrap()
        };
        assert_eq!(decode(&one).commands[0], decode(&two).commands[0]);
        assert_eq!(decode(&two).commands.len(), 2);
        // Fault fixture: dự đoán lệch +300/+400 mpx tại tick authority kế tiếp.
        let mut next_state = state.clone();
        next_state
            .step(
                &bot.config.world,
                &[boom_core::PlayerInput::default(); 2],
                &mut vec![],
            )
            .unwrap();
        let player = next_state.players.iter().find(|p| p.id == 0).unwrap();
        bot.predicted_positions
            .insert(next_state.tick, (player.x.0 + 300, player.y.0 + 400));
        let next = boom_sync::to_snapshot(
            &next_state,
            &boom_sync::SnapshotContext {
                snapshot_seq: 2,
                ..ctx
            },
        )
        .unwrap();
        bot.on_snapshot(&next, 20).unwrap();
        assert_eq!(bot.stats.correction_mpx.percentile(50), Some(700));
        assert_eq!(bot.stats.correction_mpx.percentile(95), Some(700));
        assert_eq!(bot.stats.corrections, 1);
        bot.on_snapshot(&next, 30).unwrap();
        bot.on_snapshot(&first, 40).unwrap();
        assert_eq!(bot.stats.correction_mpx.count(), 1);
        assert_eq!(bot.last_snapshot_seq, 2);
        bot.begin_round(2);
        bot.on_snapshot(&next, 50).unwrap();
        assert_eq!(bot.stats.correction_mpx.count(), 1);
    }

    /// Danh sách sảnh có một phòng đang chờ.
    fn lobby_with_room() -> LobbyList {
        LobbyList {
            rooms: vec![LobbyRoom {
                room_id: 1,
                host: 2,
                phase: boom_proto::RoomPhase::Waiting,
                member_count: 1,
                name: "room".to_owned(),
            }],
        }
    }

    #[test]
    fn direction_codes_are_in_range() {
        let mut rng = Rng::new(7);
        for _ in 0..100 {
            let direction = u8::try_from(rng.below(4) + 1).expect("mã hướng 1..=4");
            assert!((1..=4).contains(&direction));
            assert!(InputBits::new(direction, false).is_some());
        }
    }

    /// `Join` có thể bị mất (hồ sơ mạng SCOPE_V1 §7.6): bot phải gửi lại sau `retry_ms`,
    /// không khoá lệnh vĩnh viễn — nếu không, bot kẹt ở sảnh và không bao giờ vào phòng.
    #[test]
    fn join_is_retried_after_retry_interval() {
        let mut bot = test_bot();
        bot.phase = BotPhase::Lobby;
        let list = lobby_with_room();

        assert_eq!(
            bot.on_lobby(&list, 0).unwrap().len(),
            1,
            "lần đầu phải Join"
        );
        assert!(
            bot.on_lobby(&list, 100).unwrap().is_empty(),
            "trong `retry_ms` không gửi lại"
        );
        assert_eq!(
            bot.on_lobby(&list, 600).unwrap().len(),
            1,
            "sau `retry_ms` phải gửi lại `Join`"
        );
    }

    /// Server trả `PlayerAlreadyInRoom` nghĩa là bot **đã** ở trong phòng: bot dừng gửi
    /// `Join` (tránh vòng lặp `Join` → `PlayerAlreadyInRoom` → `Join`) và chờ `RoomState`.
    #[test]
    fn already_in_room_stops_the_join_loop() {
        let mut bot = test_bot();
        bot.phase = BotPhase::Lobby;
        let list = lobby_with_room();

        assert_eq!(bot.on_lobby(&list, 0).unwrap().len(), 1);
        bot.on_room_rejected(
            &RoomRejected {
                request: 1,
                reason: boom_proto::RoomRejectReason::PlayerAlreadyInRoom,
            },
            10,
        );
        assert!(
            bot.on_lobby(&list, 600).unwrap().is_empty(),
            "đã ở trong phòng thì không gửi `Join` nữa"
        );
    }

    /// Host chỉ start lượt khi **đã thấy** phòng đủ thành viên và tất cả ready: start sớm
    /// sẽ bỏ sót bot chưa vào phòng ⇒ bot đó không có state hash của lượt đó.
    ///
    /// Bot vẫn gửi `SetReady` (idempotent) để làm tươi view phòng, vì `RoomState` là frame
    /// đẩy nên một bản bị mất sẽ khiến host giữ view cũ mãi mãi.
    #[test]
    fn host_waits_for_all_members_before_start() {
        let mut bot = test_bot();
        bot.phase = BotPhase::Room;
        bot.is_room_host = true;
        bot.self_ready = true;

        // Phòng còn thiếu thành viên: host chỉ làm tươi view, không start.
        let out = bot.on_tick(600).unwrap();
        assert!(
            !room_actions(&out).contains(&RoomAction::Start),
            "phòng chưa đủ người thì host không start"
        );

        // Phòng đủ người và tất cả ready: host start.
        bot.all_ready_seen = true;
        bot.ready_sent_at_ms = 600;
        let out = bot.on_tick(1200).unwrap();
        assert!(
            room_actions(&out).contains(&RoomAction::Start),
            "phòng đủ ready thì host start"
        );
    }

    /// Các hành động phòng trong một loạt frame bot gửi đi.
    fn room_actions(frames: &[Vec<u8>]) -> Vec<RoomAction> {
        let limits = boom_proto::ProtocolLimits::v1();
        frames
            .iter()
            .filter_map(|frame| {
                let (envelope, _) = read_envelope(frame).expect("đọc envelope");
                if envelope.kind != MessageKind::RoomCommand {
                    return None;
                }
                let command: RoomCommand =
                    boom_proto::decode_frame(frame, &limits).expect("mã hoá RoomCommand");
                Some(command.action)
            })
            .collect()
    }

    /// Nonce của `Ping` đầu tiên trong một loạt frame bot gửi đi.
    fn ping_nonce(frames: &[Vec<u8>]) -> u64 {
        let limits = boom_proto::ProtocolLimits::v1();
        frames
            .iter()
            .filter_map(|frame| {
                let (envelope, _) = read_envelope(frame).expect("đọc envelope");
                if envelope.kind != MessageKind::Ping {
                    return None;
                }
                let ping: Ping = boom_proto::decode_frame(frame, &limits).expect("mã hoá Ping");
                Some(ping.0)
            })
            .next()
            .expect("bot phải gửi Ping")
    }

    /// Frame `Pong` trả lời `Ping` có nonce đó.
    fn pong_frame(nonce: u64) -> Vec<u8> {
        let limits = boom_proto::ProtocolLimits::v1();
        boom_proto::encode_frame(&Pong(nonce), &limits).expect("mã hoá Pong")
    }

    /// Bot đã bắt tay thì `on_tick` gửi `Ping` (nhịp `ping_interval_ms` của bot test).
    fn pinged_bot() -> DirectDtoBot {
        let mut bot = test_bot();
        bot.phase = BotPhase::Lobby;
        bot
    }

    /// RTT phải ghép **đúng** ping: `Pong` mang nonce của `Ping` (ADR 0009 §2).
    #[test]
    fn rtt_sample_follows_the_ping_nonce() {
        let mut bot = pinged_bot();

        let nonce = ping_nonce(&bot.on_tick(1_000).unwrap());
        bot.on_frame(1_040, &pong_frame(nonce)).unwrap();

        assert_eq!(bot.stats().rtt.values(), [40u64].as_slice());
    }

    /// `Pong` trùng hoặc tới trễ **không** được ghi mẫu RTT của ping mới và **không** được
    /// xoá ping đang chờ: nếu không, `Pong` thật của ping mới sẽ bị bỏ mất.
    #[test]
    fn stale_pong_does_not_consume_the_pending_ping() {
        let mut bot = pinged_bot();

        let nonce_a = ping_nonce(&bot.on_tick(1_000).unwrap());
        bot.on_frame(1_040, &pong_frame(nonce_a)).unwrap();
        assert_eq!(bot.stats().rtt.values(), [40u64].as_slice());

        let nonce_b = ping_nonce(&bot.on_tick(2_000).unwrap());
        assert_ne!(nonce_a, nonce_b, "mỗi ping phải có nonce riêng");

        // Bản trùng/tới trễ của `Pong` A tới **sau** khi bot đã gửi `Ping` B.
        bot.on_frame(2_010, &pong_frame(nonce_a)).unwrap();
        assert_eq!(
            bot.stats().rtt.values(),
            [40u64].as_slice(),
            "Pong cũ không được ghi mẫu RTT"
        );

        // `Pong` B thật: mẫu của B vẫn phải được ghi.
        bot.on_frame(2_050, &pong_frame(nonce_b)).unwrap();
        assert_eq!(bot.stats().rtt.values(), [40u64, 50].as_slice());
    }

    /// `Pong` mất không được làm hỏng ping kế tiếp: mỗi ping giữ mốc riêng, nên `Pong` tới
    /// muộn vẫn ghi được mẫu của chính nó.
    #[test]
    fn lost_pong_does_not_block_the_next_ping() {
        let mut bot = pinged_bot();

        let nonce_a = ping_nonce(&bot.on_tick(1_000).unwrap());
        // `Pong` A mất: không có frame nào tới.
        let nonce_b = ping_nonce(&bot.on_tick(2_000).unwrap());

        bot.on_frame(2_040, &pong_frame(nonce_b)).unwrap();
        assert_eq!(
            bot.stats().rtt.values(),
            [40u64].as_slice(),
            "chỉ mẫu của ping B"
        );

        // `Pong` A tới muộn: mẫu riêng của A, không đụng mẫu của B.
        bot.on_frame(2_100, &pong_frame(nonce_a)).unwrap();
        assert_eq!(bot.stats().rtt.values(), [40u64, 1_100].as_slice());
    }

    #[test]
    fn reconnect_uses_login_for_an_authenticated_account() {
        let mut bot = test_bot();
        bot.stats.account_id = Some(42);
        let previous = bot.next_request();
        bot.reset(1);
        assert!(
            bot.next_request() > previous,
            "request ID không được lùi khi reconnect"
        );
        let frames = bot.on_welcome(1).unwrap();
        let auth: Auth =
            boom_proto::decode_frame(&frames[0], &boom_proto::ProtocolLimits::v1()).unwrap();
        assert_eq!(auth.op, AuthOp::Login);
        assert_eq!(bot.stats.auth_failed, 0);
    }

    /// `Pong` của lần chạy trước (nonce không dùng lại) không được khớp nhầm ping mới.
    #[test]
    fn pong_of_a_previous_session_is_ignored() {
        let mut bot = pinged_bot();
        let nonce_a = ping_nonce(&bot.on_tick(1_000).unwrap());
        // Mở lại phiên: bảng pending được dọn, nonce vẫn đơn điệu.
        bot.reset(1);
        bot.phase = BotPhase::Lobby;

        let nonce_b = ping_nonce(&bot.on_tick(2_000).unwrap());
        assert_ne!(nonce_a, nonce_b);

        bot.on_frame(2_010, &pong_frame(nonce_a)).unwrap();
        assert!(
            bot.stats().rtt.values().is_empty(),
            "Pong của phiên cũ không được ghi mẫu"
        );

        bot.on_frame(2_050, &pong_frame(nonce_b)).unwrap();
        assert_eq!(bot.stats().rtt.values(), [50u64].as_slice());
    }

    /// `RoomState.host` và roster mang **account_id** (ADR 0009 Addendum B §B.2), **không**
    /// phải `PlayerId` của kết nối: bot học `account_id` từ `AuthOk` và dùng nó để biết
    /// mình là host và slot của mình trong phòng.
    #[test]
    fn host_and_slot_come_from_the_account_id() {
        let mut bot = test_bot();
        // `PlayerId` của kết nối (bot này là bot số 1) **khác** `account_id` do server cấp.
        bot.config.player = 1;
        bot.on_auth_ok(
            0,
            &AuthOk {
                account_id: 42,
                display_name: "bot0".to_owned(),
                token: "token".to_owned(),
                expires_at_ms: 0,
            },
        )
        .unwrap();
        assert_eq!(bot.account_id, Some(42));

        // Phòng có host là account_id 42 (bot này) và bot kia ở slot 0.
        let state = RoomState {
            room_id: 1,
            host: 42,
            phase: boom_proto::RoomPhase::Waiting,
            members: vec![
                RoomMember {
                    player: 7,
                    slot: 0,
                    character: RoomCharacter::C00,
                    ready: true,
                    name: "a".into(),
                },
                RoomMember {
                    player: 42,
                    slot: 1,
                    character: RoomCharacter::C01,
                    ready: true,
                    name: "a".into(),
                },
            ],
            round: None,
        };
        bot.on_room_state(&state, 10).unwrap();

        assert!(
            bot.is_room_host,
            "host là account_id của bot thì bot là host"
        );
        assert_eq!(bot.slot, Some(1), "slot lấy theo account_id");
        assert!(bot.self_ready, "ready của mình lấy theo account_id");
    }

    /// `PlayerId` của kết nối trùng với `account_id` của **người khác** thì bot không
    /// được tưởng mình là host: nếu không, bot gửi `Start` liên tục và bị từ chối
    /// (`NotHost`) ⇒ phòng treo.
    #[test]
    fn connection_player_id_is_not_the_account_id() {
        let mut bot = test_bot();
        bot.config.player = 42;
        bot.on_auth_ok(
            0,
            &AuthOk {
                account_id: 7,
                display_name: "bot1".to_owned(),
                token: "token".to_owned(),
                expires_at_ms: 0,
            },
        )
        .unwrap();

        let state = RoomState {
            room_id: 1,
            // Host là account_id 42 — trùng `PlayerId` của kết nối bot này.
            host: 42,
            phase: boom_proto::RoomPhase::Waiting,
            members: vec![
                RoomMember {
                    player: 42,
                    slot: 0,
                    character: RoomCharacter::C00,
                    ready: true,
                    name: "a".into(),
                },
                RoomMember {
                    player: 7,
                    slot: 1,
                    character: RoomCharacter::C01,
                    ready: true,
                    name: "a".into(),
                },
            ],
            round: None,
        };
        bot.on_room_state(&state, 10).unwrap();

        assert!(
            !bot.is_room_host,
            "host so với account_id, không so với PlayerId"
        );
        assert_eq!(bot.slot, Some(1), "slot lấy theo account_id");
        assert!(bot.self_ready, "ready của mình lấy theo account_id");
    }
}
