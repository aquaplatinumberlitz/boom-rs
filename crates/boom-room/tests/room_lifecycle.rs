//! Acceptance máy trạng thái phòng cho SCOPE_V1 §7 mục 2–3 ở mức state machine.

use boom_room::{
    CharacterId, Lobby, MemberView, Outcome, PlayerId, ReadyResetReason, RequestId, RoomAction,
    RoomCommand, RoomError, RoomEvent, RoomId, RoomPhase, RoomPolicy, RoundId, RoundMember, SlotId,
};

const MAX_PLAYERS: u8 = 4;
const MIN_PLAYERS: u8 = 2;

fn policy() -> RoomPolicy {
    // Fixture policy của SCOPE_V1 §3 (DESIGN); trong production server nạp từ profile/scope.
    RoomPolicy::new(MAX_PLAYERS, MIN_PLAYERS).expect("policy fixture hợp lệ")
}

fn player(id: u64) -> PlayerId {
    PlayerId(id)
}

fn request(id: u64) -> RequestId {
    RequestId(id)
}

fn create(player_id: u64, request_id: u64, character: CharacterId) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::Create { character },
    }
}

fn join(player_id: u64, request_id: u64, room: RoomId, character: CharacterId) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::Join { room, character },
    }
}

fn leave(player_id: u64, request_id: u64) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::Leave,
    }
}

fn kick(player_id: u64, request_id: u64, target: u64) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::Kick {
            target: player(target),
        },
    }
}

fn select(player_id: u64, request_id: u64, character: CharacterId) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::SelectCharacter { character },
    }
}

fn ready(player_id: u64, request_id: u64, value: bool) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::SetReady { ready: value },
    }
}

fn start(player_id: u64, request_id: u64) -> RoomCommand {
    RoomCommand {
        player: player(player_id),
        request: request(request_id),
        action: RoomAction::Start,
    }
}

fn apply_ok(lobby: &mut Lobby, command: RoomCommand) -> Vec<RoomEvent> {
    match lobby.apply(command).expect("lệnh phải thành công") {
        Outcome::Applied(events) => events,
        Outcome::Duplicate => panic!("lệnh không được là duplicate"),
    }
}

fn apply_err(lobby: &mut Lobby, command: RoomCommand) -> RoomError {
    lobby.apply(command).expect_err("lệnh phải bị từ chối")
}

fn member(player_id: u64, slot: u8, character: CharacterId, ready: bool) -> MemberView {
    MemberView {
        player: player(player_id),
        slot: SlotId(slot),
        character,
        ready,
    }
}

#[test]
fn create_room_has_unique_id_host_slot_and_one_room_per_player() {
    let mut lobby = Lobby::new(policy());

    let events = apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    assert_eq!(
        events,
        vec![RoomEvent::RoomCreated {
            room: RoomId(1),
            host: player(1),
            character: CharacterId::C00,
        }]
    );
    let room = lobby.room(RoomId(1)).expect("phòng vừa tạo phải có");
    assert_eq!(room.id, RoomId(1));
    assert_eq!(room.host, player(1));
    assert_eq!(room.phase, RoomPhase::Waiting);
    assert_eq!(room.members, vec![member(1, 0, CharacterId::C00, false)]);
    assert_eq!(room.round, None);
    assert_eq!(lobby.player_room(player(1)), Some(RoomId(1)));

    apply_ok(&mut lobby, create(2, 1, CharacterId::C01));
    assert_eq!(lobby.rooms().len(), 2);
    assert_eq!(
        lobby.room(RoomId(2)).expect("phòng thứ hai").host,
        player(2)
    );
    assert_eq!(
        lobby.room(RoomId(2)).expect("phòng thứ hai").members[0].character,
        CharacterId::C01
    );

    // Một tài khoản chỉ ở một phòng: tạo mới và join phòng khác đều bị từ chối.
    assert_eq!(
        apply_err(&mut lobby, create(1, 2, CharacterId::C01)),
        RoomError::PlayerAlreadyInRoom {
            player: player(1),
            room: RoomId(1),
        }
    );
    assert_eq!(
        apply_err(&mut lobby, join(1, 2, RoomId(2), CharacterId::C00)),
        RoomError::PlayerAlreadyInRoom {
            player: player(1),
            room: RoomId(1),
        }
    );

    // Cùng request ID không tạo thêm phòng.
    assert!(
        lobby
            .apply(create(1, 1, CharacterId::C00))
            .unwrap()
            .is_duplicate()
    );
    assert_eq!(lobby.rooms().len(), 2);
}

#[test]
fn join_fills_lowest_slots_and_cannot_exceed_capacity() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C00));
    apply_ok(&mut lobby, join(3, 1, room, CharacterId::C01));
    apply_ok(&mut lobby, join(4, 1, room, CharacterId::C01));

    let view = lobby.room(room).expect("phòng còn tồn tại");
    assert_eq!(
        view.members,
        vec![
            member(1, 0, CharacterId::C00, false),
            member(2, 1, CharacterId::C00, false),
            member(3, 2, CharacterId::C01, false),
            member(4, 3, CharacterId::C01, false),
        ]
    );

    // Join thứ năm bị từ chối, kể cả khi chạy lặp lại: không bao giờ quá 4 slot.
    for _ in 0..3 {
        assert_eq!(
            apply_err(&mut lobby, join(5, 1, room, CharacterId::C00)),
            RoomError::RoomFull {
                room,
                capacity: MAX_PLAYERS,
            }
        );
    }
    assert_eq!(
        lobby.room(room).expect("phòng còn tồn tại").members.len(),
        4
    );
    assert_eq!(lobby.player_room(player(5)), None);

    // Rời phòng giải phóng đúng slot thấp nhất; người mới được nhận vào slot đó.
    apply_ok(&mut lobby, leave(2, 2));
    let events = apply_ok(&mut lobby, join(5, 2, room, CharacterId::C01));
    assert_eq!(
        events,
        vec![RoomEvent::PlayerJoined {
            room,
            player: player(5),
            slot: SlotId(1),
            character: CharacterId::C01,
        }]
    );
    let view = lobby.room(room).expect("phòng còn tồn tại");
    assert_eq!(
        view.members
            .iter()
            .map(|m| (m.player, m.slot))
            .collect::<Vec<_>>(),
        vec![
            (player(1), SlotId(0)),
            (player(5), SlotId(1)),
            (player(3), SlotId(2)),
            (player(4), SlotId(3)),
        ]
    );
}

#[test]
fn host_leave_transfers_to_earliest_remaining_member() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C00));
    apply_ok(&mut lobby, join(3, 1, room, CharacterId::C01));

    let events = apply_ok(&mut lobby, leave(1, 2));
    assert_eq!(
        events,
        vec![
            RoomEvent::PlayerLeft {
                room,
                player: player(1),
            },
            RoomEvent::HostChanged {
                room,
                host: player(2),
                previous: player(1),
            },
        ]
    );
    assert_eq!(lobby.room(room).expect("phòng còn").host, player(2));

    let events = apply_ok(&mut lobby, leave(2, 2));
    assert_eq!(
        events,
        vec![
            RoomEvent::PlayerLeft {
                room,
                player: player(2),
            },
            RoomEvent::HostChanged {
                room,
                host: player(3),
                previous: player(2),
            },
        ]
    );
    assert_eq!(lobby.room(room).expect("phòng còn").host, player(3));

    // Người cuối cùng rời: phòng bị xóa, không còn host/round.
    let events = apply_ok(&mut lobby, leave(3, 2));
    assert_eq!(
        events,
        vec![RoomEvent::PlayerLeft {
            room,
            player: player(3),
        }]
    );
    assert_eq!(lobby.room(room), None);
    assert_eq!(lobby.player_room(player(3)), None);
    assert_eq!(
        apply_err(&mut lobby, leave(3, 3)),
        RoomError::PlayerNotInRoom { player: player(3) }
    );
}

#[test]
fn roster_or_character_change_clears_all_ready() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C00));

    apply_ok(&mut lobby, ready(1, 2, true));
    apply_ok(&mut lobby, ready(2, 2, true));
    assert!(
        lobby
            .room(room)
            .expect("phòng còn")
            .members
            .iter()
            .all(|m| m.ready)
    );

    let events = apply_ok(&mut lobby, select(2, 3, CharacterId::C01));
    assert_eq!(
        events,
        vec![
            RoomEvent::CharacterSelected {
                room,
                player: player(2),
                character: CharacterId::C01,
            },
            RoomEvent::ReadyReset {
                room,
                reason: ReadyResetReason::SelectionChanged,
                cleared: vec![player(1), player(2)],
            },
        ]
    );
    assert!(
        lobby
            .room(room)
            .expect("phòng còn")
            .members
            .iter()
            .all(|m| !m.ready)
    );

    apply_ok(&mut lobby, ready(1, 3, true));
    apply_ok(&mut lobby, ready(2, 4, true));
    // Chọn lại đúng nhân vật cũ không phải thay đổi: không event, không hủy ready.
    let events = apply_ok(&mut lobby, select(2, 5, CharacterId::C01));
    assert!(events.is_empty());
    assert!(
        lobby
            .room(room)
            .expect("phòng còn")
            .members
            .iter()
            .all(|m| m.ready)
    );

    // Roster đổi (join) hủy ready của mọi người đang ready.
    let events = apply_ok(&mut lobby, join(3, 1, room, CharacterId::C01));
    assert_eq!(
        events,
        vec![
            RoomEvent::PlayerJoined {
                room,
                player: player(3),
                slot: SlotId(2),
                character: CharacterId::C01,
            },
            RoomEvent::ReadyReset {
                room,
                reason: ReadyResetReason::RosterChanged,
                cleared: vec![player(1), player(2)],
            },
        ]
    );

    apply_ok(&mut lobby, ready(1, 4, true));
    apply_ok(&mut lobby, ready(2, 6, true));
    apply_ok(&mut lobby, ready(3, 2, true));
    let events = apply_ok(&mut lobby, leave(3, 3));
    assert_eq!(
        events,
        vec![
            RoomEvent::PlayerLeft {
                room,
                player: player(3),
            },
            RoomEvent::ReadyReset {
                room,
                reason: ReadyResetReason::RosterChanged,
                cleared: vec![player(1), player(2)],
            },
        ]
    );
    assert!(
        lobby
            .room(room)
            .expect("phòng còn")
            .members
            .iter()
            .all(|m| !m.ready)
    );
}

#[test]
fn start_requires_host_minimum_players_and_all_ready_then_creates_one_round() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, ready(1, 2, true));

    assert_eq!(
        apply_err(&mut lobby, start(1, 3)),
        RoomError::NotEnoughPlayers {
            room,
            required: MIN_PLAYERS,
            present: 1,
        }
    );

    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C01));
    assert_eq!(
        apply_err(&mut lobby, start(1, 4)),
        RoomError::NotAllReady {
            room,
            waiting: vec![player(1), player(2)],
        }
    );
    apply_ok(&mut lobby, ready(2, 2, true));
    // Host cũng phải ready (SCOPE_V1 §3; khác client gốc R-063).
    assert_eq!(
        apply_err(&mut lobby, start(1, 5)),
        RoomError::NotAllReady {
            room,
            waiting: vec![player(1)],
        }
    );
    apply_ok(&mut lobby, ready(1, 6, true));
    assert_eq!(
        apply_err(&mut lobby, start(2, 3)),
        RoomError::NotHost {
            player: player(2),
            room,
        }
    );

    let events = apply_ok(&mut lobby, start(1, 7));
    let members = vec![
        RoundMember {
            player: player(1),
            slot: SlotId(0),
            character: CharacterId::C00,
        },
        RoundMember {
            player: player(2),
            slot: SlotId(1),
            character: CharacterId::C01,
        },
    ];
    assert_eq!(
        events,
        vec![RoomEvent::RoundStarted {
            room,
            round: RoundId(1),
            members: members.clone(),
        }]
    );
    let view = lobby.room(room).expect("phòng còn");
    assert_eq!(view.phase, RoomPhase::Started);
    assert_eq!(
        view.round,
        Some(boom_room::RoundView {
            id: RoundId(1),
            members: members.clone(),
        })
    );

    // Gửi lại đúng request ID: không tạo lượt thứ hai, không phát event.
    let duplicate = lobby.apply(start(1, 7)).expect("request lặp phải dedup");
    assert!(duplicate.is_duplicate());
    assert!(duplicate.events().is_empty());
    assert_eq!(
        lobby
            .room(room)
            .expect("phòng còn")
            .round
            .as_ref()
            .unwrap()
            .id,
        RoundId(1)
    );

    // Request ID mới sau khi đã start: bị từ chối, vẫn chỉ một lượt.
    assert_eq!(
        apply_err(&mut lobby, start(1, 8)),
        RoomError::AlreadyStarted {
            room,
            round: RoundId(1),
        }
    );
    assert_eq!(
        lobby
            .room(room)
            .expect("phòng còn")
            .round
            .as_ref()
            .unwrap()
            .members,
        members
    );

    // Roster/selection/ready/kick bị chặn khi đã start; snapshot round không đổi.
    assert_eq!(
        apply_err(&mut lobby, join(3, 1, room, CharacterId::C01)),
        RoomError::NotWaiting {
            room,
            phase: RoomPhase::Started,
        }
    );
    assert_eq!(
        apply_err(&mut lobby, select(1, 9, CharacterId::C01)),
        RoomError::NotWaiting {
            room,
            phase: RoomPhase::Started,
        }
    );
    assert_eq!(
        apply_err(&mut lobby, ready(1, 10, false)),
        RoomError::NotWaiting {
            room,
            phase: RoomPhase::Started,
        }
    );
    assert_eq!(
        apply_err(&mut lobby, kick(1, 11, 2)),
        RoomError::NotWaiting {
            room,
            phase: RoomPhase::Started,
        }
    );
}

#[test]
fn kick_is_host_only_and_only_while_waiting() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C00));
    apply_ok(&mut lobby, join(3, 1, room, CharacterId::C01));

    assert_eq!(
        apply_err(&mut lobby, kick(2, 2, 3)),
        RoomError::NotHost {
            player: player(2),
            room,
        }
    );
    assert_eq!(
        apply_err(&mut lobby, kick(1, 2, 1)),
        RoomError::CannotKickSelf { player: player(1) }
    );
    assert_eq!(
        apply_err(&mut lobby, kick(1, 2, 9)),
        RoomError::TargetNotInRoom {
            target: player(9),
            room,
        }
    );

    // Kick là thay đổi roster: ready của người còn lại bị hủy.
    apply_ok(&mut lobby, ready(2, 2, true));
    let events = apply_ok(&mut lobby, kick(1, 4, 3));
    assert_eq!(
        events,
        vec![
            RoomEvent::PlayerKicked {
                room,
                player: player(3),
                by: player(1),
            },
            RoomEvent::ReadyReset {
                room,
                reason: ReadyResetReason::RosterChanged,
                cleared: vec![player(2)],
            },
        ]
    );
    assert_eq!(lobby.player_room(player(3)), None);
    assert_eq!(lobby.room(room).expect("phòng còn").members.len(), 2);

    // Người bị kick được vào lại (phòng vẫn waiting) và nhận slot thấp nhất trống.
    let events = apply_ok(&mut lobby, join(3, 2, room, CharacterId::C01));
    assert_eq!(
        events,
        vec![RoomEvent::PlayerJoined {
            room,
            player: player(3),
            slot: SlotId(2),
            character: CharacterId::C01,
        }]
    );

    // Sau start, host cũng không kick được.
    apply_ok(&mut lobby, ready(1, 5, true));
    apply_ok(&mut lobby, ready(2, 3, true));
    apply_ok(&mut lobby, ready(3, 3, true));
    apply_ok(&mut lobby, start(1, 6));
    assert_eq!(
        apply_err(&mut lobby, kick(1, 7, 2)),
        RoomError::NotWaiting {
            room,
            phase: RoomPhase::Started,
        }
    );
}

#[test]
fn duplicate_and_stale_requests_are_deterministic() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 10, CharacterId::C00));

    let duplicate = lobby.apply(create(1, 10, CharacterId::C00)).unwrap();
    assert!(duplicate.is_duplicate());
    assert!(duplicate.events().is_empty());
    assert_eq!(lobby.rooms().len(), 1);

    assert_eq!(
        apply_err(&mut lobby, create(1, 9, CharacterId::C00)),
        RoomError::StaleRequest {
            player: player(1),
            last: request(10),
            received: request(9),
        }
    );

    // Lệnh lỗi không đổi state và không ghi request ID: thử lại cùng ID sau khi sửa điều kiện.
    assert_eq!(
        apply_err(&mut lobby, join(2, 5, RoomId(99), CharacterId::C00)),
        RoomError::RoomNotFound { room: RoomId(99) }
    );
    apply_ok(&mut lobby, join(2, 5, room, CharacterId::C00));
    assert_eq!(lobby.room(room).expect("phòng còn").members.len(), 2);

    assert!(
        lobby
            .apply(join(2, 5, room, CharacterId::C00))
            .unwrap()
            .is_duplicate()
    );
    assert_eq!(lobby.room(room).expect("phòng còn").members.len(), 2);
    assert_eq!(
        apply_err(&mut lobby, join(2, 4, room, CharacterId::C00)),
        RoomError::StaleRequest {
            player: player(2),
            last: request(5),
            received: request(4),
        }
    );
}

#[test]
fn members_and_round_are_sorted_by_slot_after_refill() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C00));
    apply_ok(&mut lobby, join(3, 1, room, CharacterId::C01));

    apply_ok(&mut lobby, leave(2, 2));
    apply_ok(&mut lobby, join(4, 1, room, CharacterId::C01));
    assert_eq!(
        lobby
            .room(room)
            .expect("phòng còn")
            .members
            .iter()
            .map(|m| m.player)
            .collect::<Vec<_>>(),
        vec![player(1), player(4), player(3)]
    );

    apply_ok(&mut lobby, ready(1, 3, true));
    apply_ok(&mut lobby, ready(4, 2, true));
    apply_ok(&mut lobby, ready(3, 2, true));
    let events = apply_ok(&mut lobby, start(1, 4));
    assert!(matches!(
        events.as_slice(),
        [RoomEvent::RoundStarted { members, .. }]
            if members
                == &vec![
                    RoundMember {
                        player: player(1),
                        slot: SlotId(0),
                        character: CharacterId::C00,
                    },
                    RoundMember {
                        player: player(4),
                        slot: SlotId(1),
                        character: CharacterId::C01,
                    },
                    RoundMember {
                        player: player(3),
                        slot: SlotId(2),
                        character: CharacterId::C01,
                    },
                ]
    ));
}

#[test]
fn host_leave_during_round_keeps_round_snapshot_and_transfers_host() {
    let mut lobby = Lobby::new(policy());
    let room = RoomId(1);
    apply_ok(&mut lobby, create(1, 1, CharacterId::C00));
    apply_ok(&mut lobby, join(2, 1, room, CharacterId::C01));
    apply_ok(&mut lobby, ready(1, 2, true));
    apply_ok(&mut lobby, ready(2, 2, true));
    apply_ok(&mut lobby, start(1, 3));

    let before = lobby
        .room(room)
        .expect("phòng còn")
        .round
        .expect("đã start");
    let events = apply_ok(&mut lobby, leave(1, 4));
    assert_eq!(
        events,
        vec![
            RoomEvent::PlayerLeft {
                room,
                player: player(1),
            },
            RoomEvent::HostChanged {
                room,
                host: player(2),
                previous: player(1),
            },
        ]
    );
    let view = lobby.room(room).expect("phòng vẫn còn");
    assert_eq!(view.phase, RoomPhase::Started);
    assert_eq!(view.host, player(2));
    // Rời phòng không làm mất roster đã chốt của lượt đang chạy.
    assert_eq!(view.round, Some(before));
}
