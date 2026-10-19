//! Module mạng của client V1 (ADR 0005, ADR 0009 §"Tác động tới prediction/reconciliation").
//!
//! Module này **không phụ thuộc macroquad**: nó chỉ nói chuyện với server qua trait
//! `Transport` của `boom-net`, nên test được headless (không cửa sổ, không render).
//!
//! Chia hai lớp:
//!
//! - [`session`]: đường truyền + codec — handshake `Hello`/`Welcome`/`Reject`, auth
//!   `Auth`/`AuthOk`/`AuthFail`, gửi `InputBatch`, nhận `Snapshot`/`RoundEvents`/`InputRejected`,
//!   dedup event theo `(round_id, EventId)`.
//! - [`prediction`]: state confirmed + state dự đoán của chính mình, replay lệnh chưa ack khi
//!   có snapshot (correction).
//!
//! [`NetClient`] ghép hai lớp trên cho vòng lặp của client: bơm message, dự đoán một bước và
//! gửi cửa sổ lệnh chưa ack.
//!
//! Chưa làm trong module này (ngoài phạm vi issue #80): sảnh/phòng (`RoomCommand`/`RoomState`
//! chưa có DTO phía client), audio.

mod prediction;
mod session;

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;

pub use prediction::{
    ALPHA_ONE, BombShadow, PredictError, Predictor, SnapshotOutcome, world_from_snapshot,
};
pub use session::{ClientMessage, ClientSession, NetError};

use std::fmt;
use std::time::Duration;

use boom_content::rules::RulesProfile;
use boom_core::{Mode, Phase, PlayerInput, Rules, SimulationState, mode_by_id};
use boom_net::{TcpTransport, Transport};
use boom_proto::dto::auth::{Auth, AuthOk, AuthOp};
use boom_proto::{Hello, InputBatch, RoundEvent, Welcome};

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
        }
    }

    /// Handshake với server.
    pub fn handshake(&mut self, hello: &Hello) -> Result<Welcome, NetError> {
        let welcome = self.session.handshake(hello)?;
        self.player_slot = Some(welcome.player_id);
        self.snapshot_interval_ms = u64::from(welcome.snapshot_interval_ms.max(1));
        self.welcome = Some(welcome.clone());
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
            // Nhóm phòng/sảnh (ADR 0009 Addendum B) chưa dùng ở #80: bỏ qua, không đoán.
            ClientMessage::Room(_) => {}
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

    fn apply_snapshot(&mut self, snapshot: boom_proto::Snapshot) -> Result<(), NetError> {
        if self.predictor.is_none() {
            // Snapshot đầu tiên: dựng thế giới của client (map chưa có bundle nên lấy từ
            // snapshot) rồi tạo predictor.
            let Some(player) = self.player_slot else {
                return Err(NetError::Unexpected(
                    "chưa handshake nên chưa biết slot người chơi".into(),
                ));
            };
            let world = world_from_snapshot(&snapshot, &self.rules, self.mode)
                .map_err(|error| NetError::Unexpected(format!("dựng world client: {error}")))?;
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
