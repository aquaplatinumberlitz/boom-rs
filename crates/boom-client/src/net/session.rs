//! Phiên mạng phía client: handshake, gửi input, nhận snapshot/event.
//!
//! Lớp này chỉ lo **đường truyền và codec** (ADR 0006 + ADR 0009 §1–§3). Nó không biết luật
//! chơi, không dự đoán: phần dự đoán/reconciliation nằm ở [`super::prediction`].
//!
//! Đọc và ghi tách hai chiều: một luồng đọc chặn đẩy frame vào hàng đợi, phía ghi gọi
//! `send_frame` trực tiếp. Nhờ vậy client không bao giờ chặn vòng render khi chờ server, và
//! test headless vẫn chạy được với `MemoryTransport`.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use boom_net::{NetError as TransportError, Transport};
use boom_proto::kind::{Channel, MessageKind, Role};
use boom_proto::message::decode_frame_on;
use boom_proto::{
    EventId, Hello, InputBatch, InputRejected, Message, Ping, Pong, ProtoError, ProtocolLimits,
    Reject, RoundEvent, RoundEvents, Snapshot, SnapshotRequest, Welcome, encode_frame,
    read_envelope,
};

/// Số frame chờ tối đa trong hàng đợi đọc phía client.
///
/// **Tham số vận hành** (design), không phải luật chơi: hàng đợi có trần để client chậm không
/// làm phình bộ nhớ. Không nằm trong `Rules` (đây không phải luật trận).
const INBOUND_QUEUE_FRAMES: usize = 64;

/// Thời gian chờ handshake.
///
/// **Tham số vận hành** (design): server chặn ở bước handshake; chờ lâu hơn thì coi như
/// kết nối hỏng và đóng.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Message server gửi cho client, đã decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    /// Server chấp nhận handshake.
    Welcome(Welcome),
    /// Server từ chối handshake/session.
    Reject(Reject),
    /// Snapshot đầy đủ của round.
    Snapshot(Snapshot),
    /// Event của round.
    RoundEvents(RoundEvents),
    /// Server từ chối một batch input.
    InputRejected(InputRejected),
    /// Ping của server (đo RTT).
    Ping(Ping),
    /// Pong của server.
    Pong(Pong),
    /// Kind lạ hoặc chiều không hợp lệ: phải đóng kết nối, không đoán (ADR 0009 §2).
    UnknownKind(u16),
}

/// Lỗi của tầng mạng phía client.
#[derive(Debug)]
pub enum NetError {
    /// Lỗi transport: I/O, frame quá trần, kết nối đóng.
    Transport(TransportError),
    /// Lỗi codec/envelope của `boom-proto`.
    Proto(ProtoError),
    /// Đối phương đóng kết nối.
    Closed,
    /// Hết thời gian chờ message.
    Timeout,
    /// Server từ chối handshake (sai rules/content/schema).
    Rejected(Reject),
    /// Message không hợp lệ trong ngữ cảnh hiện tại.
    Unexpected(String),
    /// Lỗi I/O khi mở socket hoặc đọc/ghi cục bộ.
    Io(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "lỗi transport: {e}"),
            Self::Proto(e) => write!(f, "lỗi giao thức: {e}"),
            Self::Closed => write!(f, "kết nối đã đóng"),
            Self::Timeout => write!(f, "hết thời gian chờ message"),
            Self::Rejected(reject) => write!(
                f,
                "server từ chối phiên: {:?} ({})",
                reject.reason, reject.detail
            ),
            Self::Unexpected(detail) => write!(f, "message không mong đợi: {detail}"),
            Self::Io(detail) => write!(f, "lỗi I/O: {detail}"),
        }
    }
}

impl std::error::Error for NetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            Self::Proto(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for NetError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

impl From<ProtoError> for NetError {
    fn from(value: ProtoError) -> Self {
        Self::Proto(value)
    }
}

/// Frame đọc được từ transport, chưa decode.
enum Inbound {
    Frame(Vec<u8>),
    Closed,
    Failed(String),
}

/// Phiên mạng phía client trên một transport hai chiều.
///
/// `reader` và `writer` là hai nửa của cùng kết nối: với TCP thì hai socket clone của một
/// stream, với test thì hai cặp `MemoryTransport` (một chiều mỗi cặp) — cùng cách server
/// chia hai chiều.
pub struct ClientSession<W: Transport> {
    writer: W,
    inbound: Receiver<Inbound>,
    limits: ProtocolLimits,
    /// Event đã trình bày, dedup theo `(round_id, EventId)` (ADR 0009 §5.4).
    seen_events: BTreeSet<(u32, EventId)>,
}

impl<W: Transport> ClientSession<W> {
    /// Mở phiên: `reader` chạy ở luồng riêng, `writer` dùng ở luồng gọi.
    pub fn new<R: Transport + Send + 'static>(reader: R, writer: W) -> Self {
        let (tx, rx) = mpsc::sync_channel(INBOUND_QUEUE_FRAMES);
        thread::spawn(move || {
            let mut reader = reader;
            loop {
                match reader.recv_frame() {
                    Ok(Some(frame)) => {
                        if tx.send(Inbound::Frame(frame)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = tx.send(Inbound::Closed);
                        break;
                    }
                    Err(error) => {
                        let _ = tx.send(Inbound::Failed(error.to_string()));
                        break;
                    }
                }
            }
        });
        Self {
            writer,
            inbound: rx,
            limits: ProtocolLimits::v1(),
            seen_events: BTreeSet::new(),
        }
    }

    /// Miền giới hạn giao thức đang dùng.
    pub fn limits(&self) -> &ProtocolLimits {
        &self.limits
    }

    /// Handshake: gửi `Hello`, chờ `Welcome` hoặc `Reject`.
    ///
    /// `Reject` là kết quả hợp lệ (server từ chối do lệch rules/content/schema), trả về
    /// [`NetError::Rejected`] để caller hiện lỗi rõ ràng (ADR 0009 §1).
    pub fn handshake(&mut self, hello: &Hello) -> Result<Welcome, NetError> {
        self.send(hello)?;
        match self.next_message(HANDSHAKE_TIMEOUT)? {
            Some(ClientMessage::Welcome(welcome)) => Ok(welcome),
            Some(ClientMessage::Reject(reject)) => Err(NetError::Rejected(reject)),
            Some(other) => Err(NetError::Unexpected(format!(
                "message {other:?} đến trước Welcome"
            ))),
            None => Err(NetError::Timeout),
        }
    }

    /// Gửi một batch input.
    pub fn send_input_batch(&mut self, batch: &InputBatch) -> Result<(), NetError> {
        self.send(batch)
    }

    /// Xin snapshot đầy đủ để resync (ADR 0009 §4.3).
    pub fn request_snapshot(
        &mut self,
        round_id: u32,
        last_snapshot_seq: u32,
    ) -> Result<(), NetError> {
        self.send(&SnapshotRequest {
            round_id,
            last_snapshot_seq,
        })
    }

    /// Đọc message tiếp theo; `Ok(None)` khi hết thời gian chờ.
    pub fn next_message(&mut self, timeout: Duration) -> Result<Option<ClientMessage>, NetError> {
        let raw = match self.inbound.recv_timeout(timeout) {
            Ok(raw) => raw,
            Err(RecvTimeoutError::Timeout) => return Ok(None),
            Err(RecvTimeoutError::Disconnected) => return Err(NetError::Closed),
        };
        let frame = match raw {
            Inbound::Frame(frame) => frame,
            Inbound::Closed => return Err(NetError::Closed),
            Inbound::Failed(detail) => return Err(NetError::Io(detail)),
        };
        Ok(Some(self.decode(&frame)?))
    }

    /// Lọc event trùng: chỉ trả về event chưa trình bày.
    ///
    /// Dedup theo `(round_id, EventId)` (ADR 0009 §5.4): snapshot không phát lại event, và
    /// reconcile không được phát âm thanh/hiệu ứng hai lần.
    pub fn accept_events(&mut self, events: RoundEvents) -> Vec<RoundEvent> {
        let mut fresh = Vec::new();
        for event in events.events {
            let key = (events.round_id, event.id);
            if self.seen_events.insert(key) {
                fresh.push(event);
            }
        }
        fresh
    }

    /// Đóng kết nối từ phía client.
    pub fn close(&mut self) -> Result<(), NetError> {
        self.writer.close().map_err(NetError::Transport)
    }

    fn send<M: Message>(&mut self, message: &M) -> Result<(), NetError> {
        let frame = encode_frame(message, &self.limits)?;
        self.writer.send_frame(&frame).map_err(NetError::Transport)
    }

    fn decode(&self, frame: &[u8]) -> Result<ClientMessage, NetError> {
        let (envelope, _) = read_envelope(frame)?;
        let kind = envelope.kind;
        let message = match kind {
            MessageKind::Welcome => ClientMessage::Welcome(self.decode_payload::<Welcome>(frame)?),
            MessageKind::Reject => ClientMessage::Reject(self.decode_payload::<Reject>(frame)?),
            MessageKind::Snapshot => {
                ClientMessage::Snapshot(self.decode_payload::<Snapshot>(frame)?)
            }
            MessageKind::RoundEvents => {
                ClientMessage::RoundEvents(self.decode_payload::<RoundEvents>(frame)?)
            }
            MessageKind::InputRejected => {
                ClientMessage::InputRejected(self.decode_payload::<InputRejected>(frame)?)
            }
            MessageKind::Ping => ClientMessage::Ping(self.decode_payload::<Ping>(frame)?),
            MessageKind::Pong => ClientMessage::Pong(self.decode_payload::<Pong>(frame)?),
            // Kind lạ hoặc chiều không hợp lệ (ADR 0009 §2): caller đóng kết nối, không đoán.
            _ => ClientMessage::UnknownKind(kind.code()),
        };
        Ok(message)
    }

    /// Decode payload theo chiều nhận của client: chỉ nhận kind server→client trên kênh game.
    fn decode_payload<M: Message>(&self, frame: &[u8]) -> Result<M, NetError> {
        Ok(decode_frame_on::<M>(
            frame,
            &self.limits,
            Channel::Game,
            Role::Client,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_net::{MemoryConfig, MemoryTransport};
    use boom_proto::Hello;

    fn hello() -> Hello {
        Hello {
            wire_schema: 1,
            state_schema: 1,
            rules_profile_id: "p".into(),
            rules_hash: 7,
            content_sha256: [0; 32],
            client_build: "test".into(),
            capabilities: Vec::new(),
        }
    }

    #[test]
    fn handshake_nhan_welcome() {
        let (mut server_side, client_side) = MemoryTransport::pair(MemoryConfig::default());
        let (mut server_writer, client_reader) = MemoryTransport::pair(MemoryConfig::default());
        let mut session = ClientSession::new(client_reader, client_side);

        // Server giả lập: đọc Hello rồi trả Welcome.
        let handle = thread::spawn(move || {
            let frame = server_side.recv_frame().unwrap().unwrap();
            let hello: Hello = boom_proto::decode_frame(&frame, &ProtocolLimits::v1()).unwrap();
            assert_eq!(hello.rules_profile_id, "p");
            let welcome = Welcome {
                session_id: "s1".into(),
                player_id: 0,
                wire_schema: 1,
                state_schema: 1,
                rules_profile_id: "p".into(),
                rules_hash: 7,
                content_epoch: 1,
                snapshot_interval_ms: 50,
                protocol_limits_version: 1,
            };
            let frame = boom_proto::encode_frame(&welcome, &ProtocolLimits::v1()).unwrap();
            server_writer.send_frame(&frame).unwrap();
        });

        let welcome = session.handshake(&hello()).expect("handshake");
        assert_eq!(welcome.session_id, "s1");
        handle.join().unwrap();
    }

    #[test]
    fn handshake_bao_reject() {
        let (mut server_side, client_side) = MemoryTransport::pair(MemoryConfig::default());
        let (mut server_writer, client_reader) = MemoryTransport::pair(MemoryConfig::default());
        let mut session = ClientSession::new(client_reader, client_side);

        let handle = thread::spawn(move || {
            let _ = server_side.recv_frame().unwrap();
            let reject = Reject {
                reason: boom_proto::RejectReason::RulesMismatch,
                detail: "lệch rules".into(),
            };
            let frame = boom_proto::encode_frame(&reject, &ProtocolLimits::v1()).unwrap();
            server_writer.send_frame(&frame).unwrap();
        });

        match session.handshake(&hello()) {
            Err(NetError::Rejected(reject)) => {
                assert_eq!(reject.reason, boom_proto::RejectReason::RulesMismatch);
            }
            other => panic!("phải báo Reject, thấy {other:?}"),
        }
        handle.join().unwrap();
    }

    #[test]
    fn event_trung_chi_nhan_mot_lan() {
        let (client_reader, _peer) = MemoryTransport::pair(MemoryConfig::default());
        let (client_writer, _peer) = MemoryTransport::pair(MemoryConfig::default());
        let mut session = ClientSession::new(client_reader, client_writer);
        let event = RoundEvent {
            id: EventId { tick: 3, seq: 0 },
            name: "player_turned".into(),
            fields: Vec::new(),
        };
        let events = RoundEvents {
            round_id: 1,
            events: vec![event.clone()],
        };
        assert_eq!(session.accept_events(events.clone()).len(), 1);
        assert_eq!(
            session.accept_events(events).len(),
            0,
            "event lặp phải bị lọc"
        );
    }
}
