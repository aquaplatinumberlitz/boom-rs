//! Chuyển event của core → DTO `RoundEvents` (ADR 0009 §2, ARCHITECTURE §5.4).
//!
//! Event đi trong `RoundEvents` cùng stream TCP, có `round_id` + `EventId`; consumer dedup theo
//! `(round_id, EventId)`. Snapshot **không** phát lại event.

use boom_core::SimulationEvent;
use boom_proto::dto::snapshot::InputAck;
use boom_proto::{EventField, EventId, RoundEvent, RoundEvents};

/// Dựng `RoundEvents` từ danh sách event của core trong một tick.
///
/// Tên loại và trường lấy từ `EventKind::name()` / `EventKind::fields()` của core, nên digest và
/// log của cả hai bên giống nhau.
pub fn to_round_events(round_id: u32, events: &[SimulationEvent]) -> RoundEvents {
    RoundEvents {
        round_id,
        events: events
            .iter()
            .map(|e| RoundEvent {
                id: EventId {
                    tick: e.id.tick,
                    seq: e.id.seq,
                },
                name: e.kind.name().to_owned(),
                fields: e
                    .kind
                    .fields()
                    .into_iter()
                    .map(|(name, value)| EventField {
                        name: name.to_owned(),
                        value,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Ack input của một người chơi, để server đóng gói vào snapshot.
pub fn input_ack(player: u8, last_seq: u32, applied_tick: u32) -> InputAck {
    InputAck {
        player,
        last_seq,
        applied_tick,
    }
}
