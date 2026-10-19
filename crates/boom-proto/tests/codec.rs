//! Nghiệm thu codec: round-trip mọi message, golden vector ADR 0009 §5.3, từ chối frame hỏng
//! và kích thước snapshot theo bảng §4.2.

use boom_proto::dto::snapshot::{
    BombState, FieldState, FlameState, HiddenItem, InputAck, ItemState,
};
use boom_proto::{
    CellKind, EventField, EventId, Hello, InputBatch, InputBits, InputCommand, InputRejectReason,
    InputRejected, Life, MessageKind, Phase, Ping, PlayerState, Pong, ProtoError, ProtocolLimits,
    Reject, RejectReason, RoundEvent, RoundEvents, Snapshot, SnapshotRequest, Stat, Welcome,
    decode_frame, encode_frame, encode_payload, read_envelope,
};

const LIMITS: ProtocolLimits = ProtocolLimits::v1();

/// Cạnh trên của lưới Village00: 15×13 = 195 ô.
const CELLS: usize = 15 * 13;

fn hello() -> Hello {
    Hello {
        wire_schema: 1,
        state_schema: 1,
        rules_profile_id: "kr00967-v1-dev".into(),
        rules_hash: 0x1234_5678_9abc_def0,
        content_sha256: [7u8; 32],
        client_build: "boom-rs/0.1".into(),
        capabilities: vec!["snapshot".into(), "round_events".into()],
    }
}

fn welcome() -> Welcome {
    Welcome {
        session_id: "s-1".into(),
        player_id: 3,
        wire_schema: 1,
        state_schema: 1,
        server_version: "0.1.0".into(),
        rules_profile_id: "kr00967-v1-dev".into(),
        rules_hash: 0x1234_5678_9abc_def0,
        content_sha256: [9u8; 32],
        content_epoch: 9,
        snapshot_interval_ms: 50,
        protocol_limits_version: 1,
    }
}

fn input_batch() -> InputBatch {
    InputBatch {
        round_id: 1,
        player: 0,
        ack_snapshot: 10,
        commands: vec![
            InputCommand {
                seq: 7,
                target_tick: 120,
                bits: InputBits(0x08),
            },
            InputCommand {
                seq: 8,
                target_tick: 121,
                bits: InputBits(0x04),
            },
        ],
    }
}

fn input_rejected() -> InputRejected {
    InputRejected {
        round_id: 1,
        reason: InputRejectReason::OutOfWindow,
        applied_seq: 8,
        first_seq: 9,
        command_count: 1,
        server_tick: 130,
    }
}

/// Batch chỉ khác ở dãy `seq` của các lệnh (dùng để dựng frame hỏng cho test decode).
fn batch_with_seqs(seqs: &[u32]) -> InputBatch {
    InputBatch {
        round_id: 1,
        player: 0,
        ack_snapshot: 10,
        commands: seqs
            .iter()
            .map(|seq| InputCommand {
                seq: *seq,
                target_tick: 120,
                bits: InputBits(0x08),
            })
            .collect(),
    }
}

fn round_events() -> RoundEvents {
    RoundEvents {
        round_id: 1,
        events: vec![RoundEvent {
            id: EventId { tick: 12, seq: 0 },
            name: "bomb_exploded".into(),
            fields: vec![EventField {
                name: "bomb".into(),
                value: 4,
            }],
        }],
    }
}

fn snapshot_request() -> SnapshotRequest {
    SnapshotRequest {
        round_id: 1,
        last_snapshot_seq: 12,
    }
}

/// `count` ô cuối của lưới, tăng dần. Ô cuối có chỉ số ≥ 128 nên `cell` (u16) mất 2 byte
/// trong varint postcard — đúng giả định của bảng kích thước ADR 0009 §4.2.
fn last_cells(count: usize) -> Vec<u16> {
    (CELLS - count..CELLS).map(|c| c as u16).collect()
}

/// Snapshot dựng theo bảng kích thước ADR 0009 §4.2: mọi trường lấy giá trị tệ nhất của
/// varint postcard (u32 → 5 byte, u64 → 10 byte, `cell` u16 → 2 byte, toạ độ `Mpx` trong miền
/// V1 → 3 byte).
fn snapshot(players: usize, bombs: usize, flames: usize, items: usize, hidden: usize) -> Snapshot {
    let hidden_cells = last_cells(hidden);
    let mut kinds = vec![CellKind::Floor; CELLS];
    for cell in &hidden_cells {
        kinds[*cell as usize] = CellKind::Breakable;
    }

    Snapshot {
        snapshot_seq: u32::MAX,
        round_id: u32::MAX,
        tick: u32::MAX,
        state_schema: u16::MAX,
        rules_hash: u64::MAX,
        content_epoch: u32::MAX,
        phase: Phase::Ended(boom_proto::Outcome::Winner(255)),
        rng_state: u64::MAX,
        next_entity_id: u32::MAX,
        acks: (0..players)
            .map(|p| InputAck {
                player: p as u8,
                last_seq: u32::MAX,
                applied_tick: u32::MAX,
            })
            .collect(),
        players: (0..players)
            .map(|p| PlayerState {
                id: p as u8,
                x: 599_999,
                y: 519_999,
                facing: 4,
                speed: Stat { value: 1, max: 10 },
                bombs: Stat { value: 1, max: 10 },
                range: Stat { value: 1, max: 7 },
                life: Life::Trapped { since: u32::MAX },
            })
            .collect(),
        field: FieldState {
            width: 15,
            height: 13,
            kinds_bits: FieldState::pack_kinds(&kinds),
            hidden: hidden_cells
                .iter()
                .map(|cell| HiddenItem {
                    cell: *cell,
                    kind: 0,
                })
                .collect(),
        },
        bombs: last_cells(bombs)
            .into_iter()
            .rev()
            .enumerate()
            .map(|(i, cell)| BombState {
                id: u32::MAX - (bombs as u32 - 1 - i as u32),
                owner: 0,
                cell,
                range: 1,
                explodes_at: u32::MAX,
                grace: vec![0, 1, 2, 3],
            })
            .collect(),
        flames: last_cells(flames)
            .into_iter()
            .map(|cell| FlameState {
                cell,
                until: u32::MAX,
                source: u32::MAX,
                owner: 0,
            })
            .collect(),
        items: last_cells(items)
            .into_iter()
            .rev()
            .enumerate()
            .map(|(i, cell)| ItemState {
                id: u32::MAX - (items as u32 - 1 - i as u32),
                kind: 0,
                cell,
            })
            .collect(),
    }
}

#[test]
fn round_trip_every_message() {
    let hello_frame = encode_frame(&hello(), &LIMITS).unwrap();
    let welcome_frame = encode_frame(&welcome(), &LIMITS).unwrap();
    let reject_frame = encode_frame(
        &Reject {
            reason: RejectReason::RulesMismatch,
            detail: "profile khác".into(),
        },
        &LIMITS,
    )
    .unwrap();
    let ping_frame = encode_frame(&Ping(42), &LIMITS).unwrap();
    let pong_frame = encode_frame(&Pong(42), &LIMITS).unwrap();
    let batch_frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    let rejected_frame = encode_frame(&input_rejected(), &LIMITS).unwrap();
    let snapshot_frame = encode_frame(&snapshot(4, 8, 20, 6, 40), &LIMITS).unwrap();
    let request_frame = encode_frame(&snapshot_request(), &LIMITS).unwrap();
    let events_frame = encode_frame(&round_events(), &LIMITS).unwrap();

    for (kind, bytes) in [
        (MessageKind::Hello, &hello_frame),
        (MessageKind::Welcome, &welcome_frame),
        (MessageKind::Reject, &reject_frame),
        (MessageKind::Ping, &ping_frame),
        (MessageKind::Pong, &pong_frame),
        (MessageKind::InputBatch, &batch_frame),
        (MessageKind::InputRejected, &rejected_frame),
        (MessageKind::Snapshot, &snapshot_frame),
        (MessageKind::SnapshotRequest, &request_frame),
        (MessageKind::RoundEvents, &events_frame),
    ] {
        let (envelope, _) = read_envelope(bytes).unwrap();
        assert_eq!(envelope.kind, kind, "kind của frame phải khớp message");
        assert_eq!(envelope.payload_len as usize, bytes.len() - 16);
    }

    assert_eq!(
        decode_frame::<Hello>(&hello_frame, &LIMITS).unwrap(),
        hello()
    );
    assert_eq!(
        decode_frame::<Welcome>(&welcome_frame, &LIMITS).unwrap(),
        welcome()
    );
    assert_eq!(
        decode_frame::<Reject>(&reject_frame, &LIMITS).unwrap(),
        Reject {
            reason: RejectReason::RulesMismatch,
            detail: "profile khác".into(),
        }
    );
    assert_eq!(
        decode_frame::<Ping>(&ping_frame, &LIMITS).unwrap(),
        Ping(42)
    );
    assert_eq!(
        decode_frame::<Pong>(&pong_frame, &LIMITS).unwrap(),
        Pong(42)
    );
    assert_eq!(
        decode_frame::<InputBatch>(&batch_frame, &LIMITS).unwrap(),
        input_batch()
    );
    assert_eq!(
        decode_frame::<InputRejected>(&rejected_frame, &LIMITS).unwrap(),
        input_rejected()
    );
    assert_eq!(
        decode_frame::<Snapshot>(&snapshot_frame, &LIMITS).unwrap(),
        snapshot(4, 8, 20, 6, 40)
    );
    assert_eq!(
        decode_frame::<SnapshotRequest>(&request_frame, &LIMITS).unwrap(),
        snapshot_request()
    );
    assert_eq!(
        decode_frame::<RoundEvents>(&events_frame, &LIMITS).unwrap(),
        round_events()
    );
}

#[test]
fn golden_vector_of_adr_0009_section_5_3() {
    // Payload và envelope ghi sẵn trong ADR 0009 §5.3.
    let payload = [0x01, 0x00, 0x0A, 0x02, 0x07, 0x78, 0x08, 0x08, 0x79, 0x04];
    let envelope = [
        0x42, 0x4F, 0x4F, 0x4D, 0x01, 0x00, 0x10, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    let frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    assert_eq!(&frame[..16], &envelope);
    assert_eq!(&frame[16..], &payload);
    assert_eq!(frame.len(), 26);

    let decoded = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap();
    assert_eq!(decoded, input_batch());
    assert_eq!(decoded.commands[0].target_tick, 120);
    assert!(decoded.commands[0].bits.place_bomb());
    assert_eq!(decoded.commands[1].bits.direction(), 4);
}

#[test]
fn decode_rejects_wrong_wire_schema() {
    let mut frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    frame[4..6].copy_from_slice(&2u16.to_le_bytes());
    assert!(matches!(
        decode_frame::<InputBatch>(&frame, &LIMITS),
        Err(ProtoError::UnsupportedWireSchema(2))
    ));
}

#[test]
fn decode_rejects_payload_over_message_limit() {
    // Khai `payload_len` vượt trần 1 KiB của `InputBatch`.
    let frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    let payload_len = LIMITS.max_input_batch_bytes + 1;
    let mut frame = frame;
    frame.truncate(16);
    frame[10..14].copy_from_slice(&(payload_len as u32).to_le_bytes());
    frame.extend(std::iter::repeat_n(0u8, payload_len));

    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::PayloadTooLarge { .. }),
        "thực tế: {err:?}"
    );
}

#[test]
fn decode_rejects_frame_over_frame_limit() {
    let frame = vec![0u8; LIMITS.max_frame_bytes + 1];
    assert!(matches!(
        decode_frame::<InputBatch>(&frame, &LIMITS),
        Err(ProtoError::FrameTooLarge { .. })
    ));
}

#[test]
fn decode_rejects_snapshot_payload_over_snapshot_limit() {
    let frame = encode_frame(&snapshot(4, 8, 20, 6, 40), &LIMITS).unwrap();
    let payload_len = LIMITS.max_snapshot_bytes + 1;
    let mut frame = frame;
    frame.truncate(16);
    frame[10..14].copy_from_slice(&(payload_len as u32).to_le_bytes());
    frame.extend(std::iter::repeat_n(0u8, payload_len));

    let err = decode_frame::<Snapshot>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::PayloadTooLarge { .. }),
        "thực tế: {err:?}"
    );
}

#[test]
fn decode_rejects_length_mismatch() {
    let mut frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    frame[10..14].copy_from_slice(&9u32.to_le_bytes());
    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::LengthMismatch { .. }),
        "thực tế: {err:?}"
    );
}

#[test]
fn decode_rejects_trailing_bytes() {
    let mut frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    frame.push(0);
    frame[10..14].copy_from_slice(&11u32.to_le_bytes());
    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::TrailingBytes(1)),
        "thực tế: {err:?}"
    );
}

#[test]
fn decode_rejects_kind_mismatch() {
    let frame = encode_frame(&input_batch(), &LIMITS).unwrap();
    let err = decode_frame::<Snapshot>(&frame, &LIMITS).unwrap_err();
    assert!(matches!(
        err,
        ProtoError::KindMismatch {
            expected: MessageKind::Snapshot,
            got: MessageKind::InputBatch
        }
    ));
}

/// Regression review PR #56: `seq` trùng ở biên `u32::MAX` và `seq = 0` phải bị từ chối khi
/// decode, không chỉ khi gọi `check` (ADR 0009 §3.1: sequence bắt đầu từ 1, tăng 1 mỗi lệnh).
#[test]
fn decode_rejects_broken_input_seq() {
    // `[MAX, MAX]`: hai lệnh cùng sequence ở biên u32.
    let frame = encode_frame(&batch_with_seqs(&[u32::MAX, u32::MAX]), &LIMITS).unwrap();
    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::Invalid { field: "seq", .. }),
        "thực tế: {err:?}"
    );

    // `[MAX, MAX]` với payload khác nhau: cùng sequence phải cùng lệnh.
    let mut batch = batch_with_seqs(&[u32::MAX, u32::MAX]);
    batch.commands[1].bits = InputBits(0b0000_0100);
    let frame = encode_frame(&batch, &LIMITS).unwrap();
    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::Invalid { field: "seq", .. }),
        "thực tế: {err:?}"
    );

    // `[0, 1]`: sequence phải bắt đầu từ 1.
    let frame = encode_frame(&batch_with_seqs(&[0, 1]), &LIMITS).unwrap();
    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::Invalid { field: "seq", .. }),
        "thực tế: {err:?}"
    );

    // Một lệnh đơn với `seq = 0`.
    let frame = encode_frame(&batch_with_seqs(&[0]), &LIMITS).unwrap();
    let err = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(err, ProtoError::Invalid { field: "seq", .. }),
        "thực tế: {err:?}"
    );
}

/// `[MAX-1, MAX]` vẫn hợp lệ: sequence tăng đúng 1 mỗi lệnh.
#[test]
fn decode_accepts_consecutive_seq_at_u32_max() {
    let frame = encode_frame(&batch_with_seqs(&[u32::MAX - 1, u32::MAX]), &LIMITS).unwrap();
    let decoded = decode_frame::<InputBatch>(&frame, &LIMITS).unwrap();
    assert_eq!(decoded.commands[0].seq, u32::MAX - 1);
    assert_eq!(decoded.commands[1].seq, u32::MAX);
}

#[test]
fn decode_rejects_player_count_over_session_limit() {
    // 5 người vượt miền V1 (4): decoder phải từ chối.
    let mut snap = snapshot(5, 8, 20, 6, 40);
    // Ack đi kèm người chơi nên cắt còn 4 để lỗi nằm đúng ở `players`.
    snap.acks.truncate(4);
    let frame = encode_frame(&snap, &LIMITS).unwrap();
    let err = decode_frame::<Snapshot>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(
            err,
            ProtoError::CountLimit {
                field: "players",
                len: 5,
                max: 4
            }
        ),
        "thực tế: {err:?}"
    );
}

#[test]
fn decode_rejects_bad_field_domain() {
    let mut snap = snapshot(4, 8, 20, 6, 40);
    // Ô ngoài lưới 15×13.
    snap.flames[0].cell = 300;
    let err =
        decode_frame::<Snapshot>(&encode_frame(&snap, &LIMITS).unwrap(), &LIMITS).unwrap_err();
    assert!(
        matches!(
            err,
            ProtoError::Invalid {
                field: "flames.cell",
                ..
            }
        ),
        "thực tế: {err:?}"
    );
}

#[test]
fn snapshot_typical_size_matches_adr_0009_table() {
    // ADR 0009 §4.2: dòng "điển hình" của bảng là 829 byte cho cả frame (813 byte payload).
    let snap = snapshot(4, 8, 20, 6, 40);
    let frame = encode_frame(&snap, &LIMITS).unwrap();
    assert_eq!(frame.len(), 829, "frame phải đúng 829 byte");
    assert_eq!(frame.len() - 16, 813, "payload phải đúng 813 byte");
}

#[test]
fn snapshot_worst_case_is_within_adr_ceiling() {
    // Trần của ADR 0009 §4.2: 5 692 byte cho cả frame.
    let snap = snapshot(4, 40, 195, 195, 195);
    let frame = encode_frame(&snap, &LIMITS).unwrap();
    assert!(
        frame.len() <= 5_692,
        "trần ADR 0009 §4.2 là 5 692 byte, thực tế {}",
        frame.len()
    );
    assert!(
        frame.len() <= LIMITS.max_snapshot_bytes,
        "snapshot phải nhỏ hơn trần 16 KiB"
    );
}

#[test]
fn snapshot_block_sizes_match_adr_0009_formulas() {
    let snap = snapshot(4, 8, 20, 6, 40);
    let payload = encode_payload(&snap).unwrap();

    // Header:  5+5+5+3+10+5+3+10+5 = 51
    // acks:    1 + 4·(1+5+5) = 45
    // players: 1 + 4·(1+3+3+1+2+2+2+6) = 81
    // field:   1+1+(1+49)+(1+40·3) = 173
    // bombs:   1 + 8·(5+1+2+1+5+1+4) = 153
    // flames:  1 + 20·(2+5+5+1) = 261
    // items:   1 + 6·(5+1+2) = 49
    assert_eq!(51 + 45 + 81 + 173 + 153 + 261 + 49, 813);
    assert_eq!(payload.len(), 813);
}
