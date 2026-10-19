//! Event của round: `RoundEvents` (ADR 0009 §2, ARCHITECTURE §5.4).
//!
//! Event đi trong `RoundEvents` cùng stream TCP, có `round_id` + `EventId`; consumer dedup theo
//! `(round_id, EventId)`. Snapshot **không** phát lại event.

use serde::{Deserialize, Serialize};

use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::ProtocolLimits;
use crate::message::Message;

/// Một trường số nguyên của event trên dây.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventField {
    /// Tên trường, ổn định theo `EventKind::fields` của core.
    pub name: String,
    /// Giá trị.
    pub value: i64,
}

/// ID ổn định của event trong một tick: `(tick, thứ tự trong tick)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventId {
    /// Tick phát event.
    pub tick: u32,
    /// Thứ tự trong tick.
    pub seq: u16,
}

/// Một event của round trên dây.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundEvent {
    /// ID event; dedup theo `(round_id, id)`.
    pub id: EventId,
    /// Tên loại event, ổn định (snake_case).
    pub name: String,
    /// Các trường dạng số nguyên, thứ tự cố định.
    pub fields: Vec<EventField>,
}

/// `RoundEvents` (kind `0x0030`, S→C): event của một round.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundEvents {
    /// Round mà event thuộc về.
    pub round_id: u32,
    /// Danh sách event, sắp theo `id`.
    pub events: Vec<RoundEvent>,
}

impl RoundEvents {
    /// Kiểm DTO sau khi decode: event phải sắp theo `id` để consumer dedup ổn định.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        for w in self.events.windows(2) {
            if w[0].id >= w[1].id {
                return Err(ProtoError::Invalid {
                    field: "events.id",
                    detail: "event phải sắp tăng theo id".into(),
                });
            }
        }
        Ok(())
    }
}

impl Message for RoundEvents {
    const KIND: MessageKind = MessageKind::RoundEvents;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        RoundEvents::check(self, limits)
    }
}
