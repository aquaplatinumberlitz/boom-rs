//! Module mạng của client V1 (ADR 0005, ADR 0009 §"Tác động tới prediction/reconciliation").
//!
//! Module này **không phụ thuộc macroquad**: nó chỉ nói chuyện với server qua trait
//! `Transport` của `boom-net`, nên test được headless (không cửa sổ, không render).
//!
//! Chia hai lớp:
//!
//! - [`session`]: đường truyền + codec — handshake `Hello`/`Welcome`/`Reject`, gửi
//!   `InputBatch`, nhận `Snapshot`/`RoundEvents`/`InputRejected`, dedup event theo
//!   `(round_id, EventId)`.
//! - [`prediction`]: state confirmed + state dự đoán của chính mình, replay lệnh chưa ack khi
//!   có snapshot (correction).
//!
//! [`NetClient`] ghép hai lớp trên cho vòng lặp của client: bơm message, dự đoán một bước và
//! gửi cửa sổ lệnh chưa ack.
//!
//! Chưa làm trong module này (ngoài phạm vi issue #80): auth (`Auth`/`AuthOk`), sảnh/phòng
//! (`RoomCommand`/`RoomState` chưa có DTO), audio.

mod prediction;
mod session;

#[cfg(test)]
mod tests;

pub use prediction::{PredictError, Predictor, world_from_snapshot};
pub use session::{ClientMessage, ClientSession, NetError};

use std::time::Duration;

use boom_content::rules::RulesProfile;
use boom_core::{Mode, PlayerInput, Rules, SimulationState, mode_by_id};
use boom_net::{TcpTransport, Transport};
use boom_proto::{Hello, InputBatch, RoundEvent, Welcome};

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
    round_id: Option<u32>,
    last_snapshot_seq: u32,
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
            round_id: None,
            last_snapshot_seq: 0,
            events: Vec::new(),
        }
    }

    /// Handshake với server.
    pub fn handshake(&mut self, hello: &Hello) -> Result<Welcome, NetError> {
        let welcome = self.session.handshake(hello)?;
        self.player_slot = Some(welcome.player_id);
        Ok(welcome)
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

    /// Dự đoán một bước của mình rồi gửi cửa sổ lệnh chưa ack.
    ///
    /// Chưa có snapshot nào thì chưa dự đoán được: lệnh chỉ được tạo khi đã biết state
    /// confirmed (ADR 0005).
    pub fn predict(&mut self, input: PlayerInput) -> Result<(), NetError> {
        let Some(predictor) = self.predictor.as_mut() else {
            return Ok(());
        };
        predictor.predict(input)?;
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

    /// Lấy event mới (đã dedup theo `(round_id, EventId)`).
    pub fn take_events(&mut self) -> Vec<RoundEvent> {
        std::mem::take(&mut self.events)
    }

    /// Round hiện tại (từ snapshot gần nhất).
    pub fn round_id(&self) -> Option<u32> {
        self.round_id
    }

    /// Slot người chơi do server cấp.
    pub fn player_slot(&self) -> Option<u8> {
        self.player_slot
    }

    fn handle(&mut self, message: ClientMessage) -> Result<(), NetError> {
        match message {
            ClientMessage::Snapshot(snapshot) => self.apply_snapshot(snapshot)?,
            ClientMessage::RoundEvents(events) => {
                let fresh = self.session.accept_events(events);
                self.events.extend(fresh);
            }
            ClientMessage::InputRejected(rejected) => {
                // Reject là tín hiệu resync sequence, không phải "hủy cả cửa sổ" (ADR 0009
                // §3.2): bỏ lệnh đã giải quyết, giữ nguyên payload lệnh còn lại để resend, và
                // xin snapshot để chắc state (ADR 0009 §3.4(d)).
                if let Some(predictor) = self.predictor.as_mut() {
                    predictor.acknowledge(rejected.applied_seq);
                }
                if let Some(round_id) = self.round_id {
                    self.session
                        .request_snapshot(round_id, self.last_snapshot_seq)?;
                }
            }
            ClientMessage::Reject(reject) => return Err(NetError::Rejected(reject)),
            ClientMessage::UnknownKind(code) => {
                let _ = self.session.close();
                return Err(NetError::Unexpected(format!(
                    "server gửi kind lạ {code:#06x}; đóng kết nối"
                )));
            }
            ClientMessage::Welcome(_) | ClientMessage::Ping(_) | ClientMessage::Pong(_) => {}
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
            self.predictor = Some(Predictor::new(world, player));
        }
        let predictor = self.predictor.as_mut().expect("đã dựng predictor ở trên");
        predictor.apply_snapshot(&snapshot)?;
        self.round_id = Some(snapshot.round_id);
        self.last_snapshot_seq = snapshot.snapshot_seq;
        Ok(())
    }

    /// Gửi cửa sổ lệnh chưa ack (bắt đầu từ lệnh chưa ack nhỏ nhất, ADR 0009 §3.2).
    fn send_window(&mut self) -> Result<(), NetError> {
        let Some(predictor) = self.predictor.as_ref() else {
            return Ok(());
        };
        if predictor.pending().is_empty() {
            return Ok(());
        }
        let commands = predictor.window(self.session.limits().max_commands);
        if commands.is_empty() {
            return Ok(());
        }
        let (Some(round_id), Some(player)) = (self.round_id, self.player_slot) else {
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

/// Nối server bằng profile rules dev của client rồi handshake.
///
/// Client chưa có bundle content (P4) nên hash content khai trong `Hello` lấy từ biến môi
/// trường `BOOM_CONTENT_SHA256` (hex 64 ký tự); không có thì khai toàn 0. Server so với bundle
/// **của chính nó** (ADR 0009 §1), nên lệch sẽ bị từ chối rõ ràng chứ không đoán.
pub fn connect(addr: &str) -> Result<NetClient<TcpTransport>, NetError> {
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
