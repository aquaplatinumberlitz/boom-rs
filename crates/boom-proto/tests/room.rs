//! Nghiệm thu Addendum B: frame round-trip cho mọi DTO phòng.

use std::fmt::Debug;

use boom_proto::{
    LoadingAck, LobbyList, LobbyListRequest, LobbyRoom, Message, MessageKind, PlayerResult,
    ProtocolLimits, ResultAck, RoomAction, RoomCharacter, RoomChat, RoomChatMessage, RoomCommand,
    RoomMember, RoomOutcome, RoomPhase, RoomRejectReason, RoomRejected, RoomState, RoundInfo,
    RoundMember, RoundResult, RoundStart, decode_frame, encode_frame, read_envelope,
};

const LIMITS: ProtocolLimits = ProtocolLimits::v1();

fn round<M: Message + PartialEq + Debug>(message: M, kind: MessageKind) {
    let frame = encode_frame(&message, &LIMITS).unwrap();
    let (envelope, _) = read_envelope(&frame).unwrap();
    assert_eq!(envelope.kind, kind);
    assert_eq!(decode_frame::<M>(&frame, &LIMITS).unwrap(), message);
}

fn state() -> RoomState {
    RoomState {
        room_id: 1,
        host: 7,
        phase: RoomPhase::Waiting,
        members: vec![RoomMember {
            player: 7,
            slot: 0,
            character: RoomCharacter::C00,
            ready: true,
            name: "a".into(),
        }],
        round: Some(RoundInfo {
            id: 3,
            members: vec![RoundMember {
                player: 7,
                slot: 0,
                character: RoomCharacter::C00,
                name: "a".into(),
            }],
        }),
    }
}

#[test]
fn every_room_frame_round_trips() {
    round(
        RoomCommand {
            request: 42,
            action: RoomAction::Join {
                room: 3,
                character: RoomCharacter::C01,
            },
        },
        MessageKind::RoomCommand,
    );
    round(state(), MessageKind::RoomState);
    round(LobbyListRequest {}, MessageKind::LobbyListRequest);
    round(
        LobbyList {
            rooms: vec![LobbyRoom {
                room_id: 1,
                host: 7,
                phase: RoomPhase::Waiting,
                member_count: 1,
                name: "room".into(),
            }],
        },
        MessageKind::LobbyList,
    );
    round(
        RoomChat {
            request: 1,
            text: "hi".into(),
        },
        MessageKind::RoomChat,
    );
    round(
        RoomChatMessage {
            room_id: 1,
            from: 7,
            from_name: "p".into(),
            text: "hi".into(),
        },
        MessageKind::RoomChatMessage,
    );
    round(LoadingAck { round_id: 1 }, MessageKind::LoadingAck);
    round(ResultAck { round_id: 1 }, MessageKind::ResultAck);
    round(
        RoundStart {
            round_id: 1,
            seed: 2,
            rules_profile_id: "p".into(),
            rules_hash: 3,
            content_sha256: [0; 32],
            map: "Village00".into(),
            mode: "battle_ffa".into(),
            roster: vec![RoundMember {
                player: 7,
                slot: 0,
                character: RoomCharacter::C00,
                name: "a".into(),
            }],
            start_tick: 4,
        },
        MessageKind::RoundStart,
    );
    round(
        RoundResult {
            round_id: 1,
            outcome: RoomOutcome::Winner(7),
            results: vec![PlayerResult {
                player: 7,
                slot: 0,
                rank: 1,
                alive: true,
            }],
        },
        MessageKind::RoundResult,
    );
    round(
        RoomRejected {
            request: 42,
            reason: RoomRejectReason::StaleRequest,
        },
        MessageKind::RoomRejected,
    );
}

#[test]
fn room_command_golden_payload() {
    let command = RoomCommand {
        request: 42,
        action: RoomAction::Join {
            room: 3,
            character: RoomCharacter::C01,
        },
    };
    let payload = boom_proto::encode_payload(&command).unwrap();
    assert_eq!(payload, vec![0x2a, 0x01, 0x03, 0x01]);
    let frame = encode_frame(&command, &LIMITS).unwrap();
    assert_eq!(&frame[6..8], &[0x40, 0x00]);
    assert_eq!(&frame[16..], payload.as_slice());
}
