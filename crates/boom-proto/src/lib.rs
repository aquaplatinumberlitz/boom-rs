//! Giao thức V1 (ADR 0009, ARCHITECTURE §5.4): envelope có revision + DTO + codec.
//!
//! Crate này chỉ chứa **DTO và codec**:
//!
//! - [`envelope`]: envelope 16 byte viết tay (`magic`, `wire_schema`, `kind`, `flags`,
//!   `payload_len`, `reserved`).
//! - [`dto`]: DTO của từng message — handshake, input, snapshot, event, phòng/sảnh (Addendum B).
//! - [`message`]: trait [`Message`] và hàm encode/decode frame.
//! - [`limits`]: giới hạn giao thức (tham số giao thức, **không** phải luật chơi).
//!
//! Không phụ thuộc `boom-core` (ARCHITECTURE §1): layout bộ nhớ core không bị khóa vào dây.
//! Chuyển đổi `SimulationState` ↔ DTO và `InputCommand` → `PlayerInput` nằm ở crate adapter
//! `boom-sync`, dùng chung cho server và client.
//!
//! Quy tắc decode (ADR 0009 §5.2): kiểm magic/schema/kind/flags/reserved/len trước; giới hạn
//! frame/snapshot/batch; kiểm số phần tử trước khi cấp phát; từ chối trailing byte; mọi lỗi là
//! giá trị trả về, không panic và không áp dụng một phần.

mod bounded;
pub mod dto;
pub mod envelope;
pub mod error;
pub mod kind;
pub mod limits;
pub mod message;

pub use dto::RULES_HASH_ALGO;
pub use dto::events::{EventField, EventId, RoundEvent, RoundEvents};
pub use dto::handshake::{Hello, Reject, RejectReason, Welcome};
pub use dto::input::{InputBatch, InputBits, InputCommand, InputRejectReason, InputRejected};
pub use dto::ping::{Ping, Pong};
pub use dto::room::{
    LoadingAck, LobbyList, LobbyListRequest, LobbyRoom, PlayerResult, ResultAck, RoomAction,
    RoomCharacter, RoomChat, RoomChatMessage, RoomCommand, RoomMember, RoomOutcome, RoomPhase,
    RoomRejectReason, RoomRejected, RoomState, RoundInfo, RoundMember, RoundResult, RoundStart,
};
pub use dto::snapshot::{
    BombState, CellKind, FieldState, FlameState, HiddenItem, InputAck, ItemState, Life, Outcome,
    Phase, PlayerState, Snapshot, SnapshotRequest, Stat,
};
pub use envelope::{Envelope, MAGIC, read_envelope};
pub use error::ProtoError;
pub use kind::MessageKind;
pub use limits::ProtocolLimits;
pub use message::{Message, decode_frame, decode_payload, encode_frame, encode_payload};
