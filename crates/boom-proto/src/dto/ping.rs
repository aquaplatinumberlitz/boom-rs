//! `Ping`/`Pong` (ADR 0009 §2): đo RTT và liveness, payload là `u64` không dấu.

use serde::{Deserialize, Serialize};

use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::ProtocolLimits;
use crate::message::Message;

/// `Ping` (kind `0x000E`): đánh dấu thời điểm gửi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping(pub u64);

/// `Pong` (kind `0x000F`): trả nguyên giá trị của `Ping`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong(pub u64);

impl Ping {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Pong {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Message for Ping {
    const KIND: MessageKind = MessageKind::Ping;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ping::check(self, limits)
    }
}

impl Message for Pong {
    const KIND: MessageKind = MessageKind::Pong;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Pong::check(self, limits)
    }
}
