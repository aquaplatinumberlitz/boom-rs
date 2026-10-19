//! Nghiệm thu chuyển event của core → DTO `RoundEvents` (ADR 0009 §2).

use boom_core::{EntityId, EventId as CoreEventId, EventKind, SimulationEvent};
use boom_proto::{ProtocolLimits, RoundEvents, decode_frame, encode_frame};
use boom_sync::to_round_events;

const LIMITS: ProtocolLimits = ProtocolLimits::v1();

#[test]
fn events_keep_id_name_and_fields() {
    let events = vec![SimulationEvent {
        id: CoreEventId { tick: 12, seq: 3 },
        kind: EventKind::BombPlaced {
            bomb: EntityId(4),
            player: 1,
            cell: boom_core::Cell::new(3, 2),
            range: 5,
        },
    }];
    let dto = to_round_events(9, &events);
    assert_eq!(dto.round_id, 9);
    assert_eq!(dto.events.len(), 1);
    assert_eq!(dto.events[0].id.tick, 12);
    assert_eq!(dto.events[0].id.seq, 3);
    assert_eq!(dto.events[0].name, "bomb_placed");
    let fields: Vec<(&str, i64)> = dto.events[0]
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.value))
        .collect();
    assert_eq!(
        fields,
        vec![("bomb", 4), ("player", 1), ("x", 3), ("y", 2), ("range", 5)]
    );
}

#[test]
fn events_round_trip_through_the_wire() {
    let events = vec![SimulationEvent {
        id: CoreEventId { tick: 1, seq: 0 },
        kind: EventKind::PlayerDied {
            player: 2,
            by: Some(0),
        },
    }];
    let dto = to_round_events(1, &events);
    let frame = encode_frame(&dto, &LIMITS).expect("encode");
    let back: RoundEvents = decode_frame(&frame, &LIMITS).expect("decode");
    assert_eq!(back, dto);
}
