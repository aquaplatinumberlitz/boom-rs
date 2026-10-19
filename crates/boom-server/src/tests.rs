use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, Dir, Life, Stat, World, mode_by_id};
use boom_proto::{InputBatch, InputBits, InputCommand};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand, RoundId};

use crate::{
    Authority, CharacterStatsTable, InputError, MatchOutcome, MatchSetup, ServerConfig, ServerPhase,
};

const RULES: &str = include_str!("../../../fixtures/rules/skeleton-test.toml");

fn stats() -> CharacterStats {
    CharacterStats {
        speed: Stat::new(1, 5),
        bombs: Stat::new(1, 5),
        range: Stat::new(1, 5),
    }
}

fn setup() -> MatchSetup {
    let rows = ["#######", "#0.1.2#", "#.....#", "#3....#", "#######"];
    let rules = RulesProfile::from_toml(RULES).unwrap();
    let world = World::new(
        Terrain::from_ascii(&rows).unwrap(),
        boom_core::Rules::from_profile(&rules).unwrap(),
        mode_by_id("battle_ffa").unwrap(),
    )
    .unwrap();
    MatchSetup {
        world,
        stats: CharacterStatsTable {
            c00: stats(),
            c01: stats(),
        },
        map: "skeleton-test".to_owned(),
    }
}

fn authority() -> Authority {
    Authority::new(ServerConfig::v1(), setup())
}

struct Requests {
    next: BTreeMap<PlayerId, u64>,
}

impl Requests {
    fn new(players: &[PlayerId]) -> Self {
        Self {
            next: players.iter().map(|p| (*p, 1)).collect(),
        }
    }

    fn take(&mut self, player: PlayerId) -> RequestId {
        let value = self.next.entry(player).or_insert(1);
        let out = RequestId(*value);
        *value += 1;
        out
    }
}

fn command(player: PlayerId, request: RequestId, action: RoomAction) -> RoomCommand {
    RoomCommand {
        player,
        request,
        action,
    }
}

fn round_u32(a: &Authority) -> u32 {
    u32::try_from(a.room_view().unwrap().round.unwrap().id.0).unwrap()
}

fn dir_code(dir: Dir) -> u8 {
    match dir {
        Dir::None => 0,
        Dir::Up => 1,
        Dir::Down => 2,
        Dir::Left => 3,
        Dir::Right => 4,
    }
}

fn input_batch(
    a: &Authority,
    player: PlayerId,
    seq: u32,
    target_tick: u32,
    dir: Dir,
    place_bomb: bool,
) -> InputBatch {
    let slot = a
        .room_view()
        .unwrap()
        .members
        .iter()
        .find(|m| m.player == player)
        .unwrap()
        .slot
        .0;
    InputBatch {
        round_id: round_u32(a),
        player: slot,
        ack_snapshot: 0,
        commands: vec![InputCommand {
            seq,
            target_tick,
            bits: InputBits::new(dir_code(dir), place_bomb).unwrap(),
        }],
    }
}

fn start_loading(a: &mut Authority, players: &[PlayerId], now: u64) -> RoundId {
    let mut req = Requests::new(players);
    let host = players[0];
    a.room_command(
        command(
            host,
            req.take(host),
            RoomAction::Create {
                character: CharacterId::C00,
            },
        ),
        now,
    )
    .unwrap();
    let room = a.room_id().unwrap();
    for (index, &player) in players.iter().enumerate().skip(1) {
        let character = if index % 2 == 0 {
            CharacterId::C00
        } else {
            CharacterId::C01
        };
        a.room_command(
            command(
                player,
                req.take(player),
                RoomAction::Join { room, character },
            ),
            now,
        )
        .unwrap();
    }
    for &player in players {
        a.room_command(
            command(
                player,
                req.take(player),
                RoomAction::SetReady { ready: true },
            ),
            now,
        )
        .unwrap();
    }
    a.room_command(command(host, req.take(host), RoomAction::Start), now)
        .unwrap();
    a.room_view().unwrap().round.unwrap().id
}

fn create_start(a: &mut Authority, players: &[PlayerId], now: u64) -> RoundId {
    let round = start_loading(a, players, now);
    for &player in players {
        a.ack_loading(player, now).unwrap();
    }
    assert_eq!(a.phase(), ServerPhase::Playing);
    round
}

fn ready_and_start(a: &mut Authority, players: &[PlayerId], now: u64, first: u64) -> RoundId {
    let host = players[0];
    for (i, &player) in players.iter().enumerate() {
        let request = RequestId(first + i as u64 + 10);
        a.room_command(
            command(player, request, RoomAction::SetReady { ready: true }),
            now,
        )
        .unwrap();
    }
    let mut max = first + players.len() as u64 + 20;
    a.room_command(command(host, RequestId(max), RoomAction::Start), now)
        .unwrap();
    max += 1;
    let _ = max;
    let round = a.room_view().unwrap().round.unwrap().id;
    for &player in players {
        a.ack_loading(player, now).unwrap();
    }
    round
}

#[test]
fn two_and_four_players_complete_two_rounds() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        let mut a = authority();
        let round1 = create_start(&mut a, &players, 0);
        assert_eq!(a.room_view().unwrap().members.len(), players.len());
        // Start lặp ở loading không tạo round thứ hai.
        let repeated = a.room_command(command(players[0], RequestId(99), RoomAction::Start), 0);
        assert!(matches!(
            repeated,
            Err(crate::ServerError::WrongPhase { .. })
        ));
        let events = a.advance_to(200_000).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::InternalServerEvent::RoundFinished { .. }))
        );
        assert_eq!(a.phase(), ServerPhase::Results);
        let result = a.last_result().unwrap();
        assert_eq!(result.round, round1);
        assert_eq!(result.outcome, MatchOutcome::Draw);
        for &player in &players {
            a.ack_result(player, 200_000).unwrap();
        }
        assert_eq!(a.phase(), ServerPhase::Waiting);
        assert!(a.room_view().unwrap().members.iter().all(|m| !m.ready));
        // Round 2: request ID mới, ready reset, state mới, round ID khác.
        let round2 = ready_and_start(&mut a, &players, 200_000, 1);
        assert_ne!(round2, round1);
        assert_eq!(a.tick(), Some(0));
        a.advance_to(400_000).unwrap();
        assert_eq!(a.last_result().unwrap().round, round2);
        assert_eq!(a.phase(), ServerPhase::Results);
    }
}

#[test]
fn duplicate_input_is_idempotent_and_out_of_window_is_rejected() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    create_start(&mut a, &players, 0);
    let batch = input_batch(&a, players[0], 1, 0, Dir::Right, false);
    let ack = a.submit_input(players[0], &batch, 0).unwrap();
    assert_eq!(ack.applied_seq, 0);
    let hash = a.state_hash().unwrap();
    // Cùng seq nhưng payload khác trong pending là BadSequence, state không đổi.
    let mut altered = batch.clone();
    altered.commands[0].target_tick = 1;
    assert!(matches!(
        a.submit_input(players[0], &altered, 0),
        Err(InputError::BadSequence { seq: 1 })
    ));
    assert_eq!(a.state_hash(), Some(hash));

    a.advance_to(16).unwrap();
    let after_apply = a.state_hash().unwrap();
    let dup = a.submit_input(players[0], &batch, 16).unwrap();
    assert_eq!(dup.applied_seq, 1);
    assert!(dup.retired.is_empty());
    assert_eq!(a.state_hash(), Some(after_apply));
    a.advance_to(32).unwrap();
    assert!(a.player_position(players[0]).is_some());

    let before_window = a.state_hash().unwrap();
    let window = input_batch(&a, players[1], 1, 100, Dir::Left, false);
    assert!(matches!(
        a.submit_input(players[1], &window, 32),
        Err(InputError::OutOfWindow { .. })
    ));
    assert_eq!(a.state_hash(), Some(before_window));
}

#[test]
fn late_input_is_retired_and_not_applied() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    create_start(&mut a, &players, 0);
    a.advance_to(200).unwrap();
    let tick = a.tick().unwrap();
    let pos = a.player_position(players[0]).unwrap();
    let batch = input_batch(
        &a,
        players[0],
        1,
        tick.saturating_sub(20),
        Dir::Right,
        false,
    );
    let ack = a.submit_input(players[0], &batch, 200).unwrap();
    assert_eq!(ack.applied_seq, 1);
    assert_eq!(ack.retired, vec![1]);
    a.advance_to(216).unwrap();
    assert_eq!(a.player_position(players[0]).unwrap(), pos);
}

#[test]
fn loading_timeout_cancels_start_and_resets_ready() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let mut req = Requests::new(&players);
    let host = players[0];
    a.room_command(
        command(
            host,
            req.take(host),
            RoomAction::Create {
                character: CharacterId::C00,
            },
        ),
        0,
    )
    .unwrap();
    let room = a.room_id().unwrap();
    a.room_command(
        command(
            players[1],
            req.take(players[1]),
            RoomAction::Join {
                room,
                character: CharacterId::C01,
            },
        ),
        0,
    )
    .unwrap();
    for &player in &players {
        a.room_command(
            command(
                player,
                req.take(player),
                RoomAction::SetReady { ready: true },
            ),
            0,
        )
        .unwrap();
    }
    a.room_command(command(host, req.take(host), RoomAction::Start), 0)
        .unwrap();
    assert_eq!(a.phase(), ServerPhase::Loading);
    a.ack_loading(host, 0).unwrap();
    let events = a.advance_to(15_000).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::InternalServerEvent::RoundCancelled { .. }))
    );
    assert_eq!(a.phase(), ServerPhase::Waiting);
    assert!(a.room_view().unwrap().members.iter().all(|m| !m.ready));
    assert!(a.last_result().is_none());
}

#[test]
fn disconnect_goes_neutral_then_eliminates_once() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    create_start(&mut a, &players, 0);
    a.disconnect(players[1], 0).unwrap();
    a.advance_to(9_000).unwrap();
    assert_ne!(a.player_life(players[1]), Some(boom_core::Life::Dead));
    a.advance_to(10_001).unwrap();
    assert_eq!(a.player_life(players[1]), Some(boom_core::Life::Dead));
    assert_eq!(
        a.last_result().unwrap().outcome,
        MatchOutcome::Winner(players[0])
    );
}

#[test]
fn duplicate_room_request_is_not_applied_twice() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let host = players[0];
    let cmd = command(
        host,
        RequestId(1),
        RoomAction::Create {
            character: CharacterId::C00,
        },
    );
    a.room_command(cmd.clone(), 0).unwrap();
    let room = a.room_id().unwrap();
    let dup = a.room_command(cmd, 0).unwrap();
    assert!(dup.duplicate);
    assert_eq!(a.room_view().unwrap().id, room);
    assert_eq!(a.room_view().unwrap().members.len(), 1);
}

#[test]
fn results_timeout_returns_to_room_without_all_acks() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let _ = create_start(&mut a, &players, 0);
    a.advance_to(200_000).unwrap();
    assert_eq!(a.phase(), ServerPhase::Results);
    a.ack_result(players[0], 200_000).unwrap();
    let events = a.advance_to(205_000).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::InternalServerEvent::ReturnedToWaiting { .. }))
    );
    assert_eq!(a.phase(), ServerPhase::Waiting);
}

#[test]
fn gap_timeout_retires_only_missing_sequences() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    create_start(&mut a, &players, 0);
    // Bỏ seq 1, gửi seq 2; gap timeout phải retire seq 1 rồi áp seq 2.
    let batch = input_batch(&a, players[0], 2, 2, Dir::Right, false);
    a.submit_input(players[0], &batch, 0).unwrap();
    let events = a.advance_to(200).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::InternalServerEvent::InputRetired { seq: 1, .. }))
    );
    let ack = a
        .submit_input(
            players[0],
            &input_batch(&a, players[0], 3, 20, Dir::Left, false),
            200,
        )
        .unwrap();
    assert_eq!(ack.applied_seq, 2);
}

#[test]
fn loading_ack_at_or_after_deadline_cancels_instead_of_starting() {
    let players = [PlayerId(1), PlayerId(2)];
    for deadline in [15_000, 20_000] {
        let mut a = authority();
        let _ = start_loading(&mut a, &players, 0);
        a.ack_loading(players[0], 0).unwrap();
        let events = a.ack_loading(players[1], deadline).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::InternalServerEvent::RoundCancelled { .. }))
        );
        assert_eq!(a.phase(), ServerPhase::Waiting);
        assert!(a.room_view().unwrap().members.iter().all(|m| !m.ready));
        assert!(a.tick().is_none());
        assert!(a.last_result().is_none());
    }
}

#[test]
fn loading_disconnect_removes_player_from_waiting() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let _ = start_loading(&mut a, &players, 0);
    a.ack_loading(players[0], 0).unwrap();
    let events = a.disconnect(players[1], 100).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::InternalServerEvent::RoundCancelled { .. }))
    );
    assert_eq!(a.phase(), ServerPhase::Waiting);
    let view = a.room_view().unwrap();
    assert_eq!(view.members.len(), 1);
    assert_eq!(view.host, players[0]);
}

#[test]
fn results_disconnect_is_not_readded_to_waiting() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let _ = create_start(&mut a, &players, 0);
    a.advance_to(200_000).unwrap();
    a.disconnect(players[1], 200_000).unwrap();
    a.ack_result(players[0], 200_000).unwrap();
    a.advance_to(205_000).unwrap();
    assert_eq!(a.phase(), ServerPhase::Waiting);
    let view = a.room_view().unwrap();
    assert_eq!(view.members.len(), 1);
    assert_eq!(view.host, players[0]);
}

#[test]
fn non_host_start_is_rejected() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let mut req = Requests::new(&players);
    let host = players[0];
    a.room_command(
        command(
            host,
            req.take(host),
            RoomAction::Create {
                character: CharacterId::C00,
            },
        ),
        0,
    )
    .unwrap();
    let room = a.room_id().unwrap();
    a.room_command(
        command(
            players[1],
            req.take(players[1]),
            RoomAction::Join {
                room,
                character: CharacterId::C01,
            },
        ),
        0,
    )
    .unwrap();
    for &player in &players {
        a.room_command(
            command(
                player,
                req.take(player),
                RoomAction::SetReady { ready: true },
            ),
            0,
        )
        .unwrap();
    }
    let err = a
        .room_command(
            command(players[1], req.take(players[1]), RoomAction::Start),
            0,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        crate::ServerError::Room(boom_room::RoomError::NotHost { .. })
    ));
    assert_eq!(a.phase(), ServerPhase::Waiting);
}

#[test]
fn host_disconnect_does_not_resurrect_host() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let _ = create_start(&mut a, &players, 0);
    a.disconnect(players[0], 0).unwrap();
    a.advance_to(10_001).unwrap();
    assert_eq!(
        a.last_result().unwrap().outcome,
        MatchOutcome::Winner(players[1])
    );
    a.advance_to(15_001).unwrap();
    assert_eq!(a.phase(), ServerPhase::Waiting);
    let view = a.room_view().unwrap();
    assert_eq!(view.members.len(), 1);
    assert_eq!(view.host, players[1]);
    assert!(!view.members[0].ready);
    a.disconnect(players[1], 15_001).unwrap();
    assert_eq!(a.phase(), ServerPhase::Waiting);
    assert!(a.room_view().is_none());
    assert!(a.room_id().is_none());
}

#[test]
fn all_disconnected_closes_room_after_results() {
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = authority();
    let _ = create_start(&mut a, &players, 0);
    a.disconnect(players[0], 0).unwrap();
    a.disconnect(players[1], 0).unwrap();
    a.advance_to(10_001).unwrap();
    a.advance_to(15_001).unwrap();
    assert_eq!(a.phase(), ServerPhase::Closed);
    assert!(a.room_view().is_none());
    assert!(a.room_id().is_none());
}

#[test]
fn second_create_is_rejected_until_room_is_deleted() {
    let host = PlayerId(10);
    let other = PlayerId(20);
    let mut a = authority();
    a.room_command(
        command(
            host,
            RequestId(1),
            RoomAction::Create {
                character: CharacterId::C00,
            },
        ),
        0,
    )
    .unwrap();
    let room = a.room_id().unwrap();
    let err = a
        .room_command(
            command(
                other,
                RequestId(1),
                RoomAction::Create {
                    character: CharacterId::C01,
                },
            ),
            0,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        crate::ServerError::RoomExists { room: existing } if existing == room
    ));
    assert_eq!(a.room_id(), Some(room));
    assert_eq!(a.room_view().unwrap().members.len(), 1);
    a.room_command(command(host, RequestId(2), RoomAction::Leave), 0)
        .unwrap();
    assert!(a.room_view().is_none());
    a.room_command(
        command(
            other,
            RequestId(2),
            RoomAction::Create {
                character: CharacterId::C01,
            },
        ),
        0,
    )
    .unwrap();
    assert_eq!(a.phase(), ServerPhase::Waiting);
    assert!(a.room_id().is_some());
}

// --- Vòng IO / transport trong bộ nhớ (phần 2) ---

use std::time::{Duration, Instant};

use boom_net::{MemoryConfig, MemoryTransport, Transport};
use boom_proto::{
    Hello, InputRejectReason, InputRejected, LoadingAck, LobbyList, LobbyListRequest, Message,
    MessageKind, Ping, Reject, RejectReason, ResultAck, RoomAction as RoomActionDto, RoomCharacter,
    RoomChat, RoomChatMessage, RoomCommand as RoomCommandDto, RoomRejectReason, RoomRejected,
    RoomState, RoundResult, RoundStart, Snapshot, Welcome, decode_frame, encode_frame,
    limits::ProtocolLimits, read_envelope,
};

use crate::{HandshakeConfig, Server};

struct WireClient {
    writer: MemoryTransport,
    reader: MemoryTransport,
}

fn handshake_config(a: &Authority) -> HandshakeConfig {
    HandshakeConfig {
        rules_profile_id: "skeleton-test".to_owned(),
        rules_hash: a.rules_hash(),
        content_sha256: [7; 32],
        content_epoch: 11,
        snapshot_interval_ms: 16,
    }
}

fn wire_client(server: &mut Server, player: PlayerId, slot: u8) -> WireClient {
    wire_client_with_configs(
        server,
        player,
        slot,
        MemoryConfig::default(),
        MemoryConfig::default(),
    )
}

fn wire_client_with_configs(
    server: &mut Server,
    player: PlayerId,
    slot: u8,
    to_server: MemoryConfig,
    to_client: MemoryConfig,
) -> WireClient {
    let (writer, server_reader) = MemoryTransport::pair(to_server);
    let (server_writer, reader) = MemoryTransport::pair(to_client);
    server
        .accept(player, slot, server_reader, server_writer)
        .unwrap();
    WireClient { writer, reader }
}

fn hello(rules_hash: u64) -> Hello {
    Hello {
        wire_schema: 1,
        state_schema: 1,
        rules_profile_id: "skeleton-test".to_owned(),
        rules_hash,
        content_sha256: [7; 32],
        client_build: "test".to_owned(),
        capabilities: Vec::new(),
    }
}

fn send<M: Message>(c: &mut WireClient, message: &M) {
    let frame = encode_frame(message, &ProtocolLimits::v1()).unwrap();
    c.writer.send_frame(&frame).unwrap();
}

fn recv_raw(c: &mut WireClient) -> Vec<u8> {
    let start = Instant::now();
    loop {
        match c.reader.try_recv_frame().unwrap() {
            Some(frame) => return frame,
            None if start.elapsed() > Duration::from_secs(2) => panic!("timeout chờ frame"),
            None => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}

fn recv<M: Message>(c: &mut WireClient) -> M {
    let frame = recv_raw(c);
    decode_frame(&frame, &ProtocolLimits::v1()).unwrap()
}

fn recv_of<M: Message>(c: &mut WireClient, kind: MessageKind) -> M {
    let start = Instant::now();
    loop {
        match c.reader.try_recv_frame().unwrap() {
            Some(frame) => {
                let (envelope, _) = read_envelope(&frame).unwrap();
                if envelope.kind == kind {
                    return decode_frame(&frame, &ProtocolLimits::v1()).unwrap();
                }
            }
            None if start.elapsed() > Duration::from_secs(2) => {
                panic!("timeout chờ {kind:?}")
            }
            None => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}

fn try_recv_of<M: Message>(c: &mut WireClient, kind: MessageKind, timeout: Duration) -> Option<M> {
    let start = Instant::now();
    loop {
        if let Some(frame) = c.reader.try_recv_frame().unwrap() {
            let (envelope, _) = read_envelope(&frame).unwrap();
            if envelope.kind == kind {
                return Some(decode_frame(&frame, &ProtocolLimits::v1()).unwrap());
            }
        } else if start.elapsed() >= timeout {
            return None;
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn send_room(c: &mut WireClient, request: u64, action: RoomActionDto) {
    send(c, &RoomCommandDto { request, action });
}

fn authenticate(server: &mut Server, players: &[PlayerId]) {
    for &player in players {
        server.mark_authenticated_for_test(player).unwrap();
    }
}

fn recv_room_state(c: &mut WireClient) -> RoomState {
    recv_of(c, MessageKind::RoomState)
}

fn recv_lobby(c: &mut WireClient) -> LobbyList {
    recv_of(c, MessageKind::LobbyList)
}

fn recv_room_state_members(c: &mut WireClient, expected: usize) -> RoomState {
    loop {
        let state: RoomState = recv_of(c, MessageKind::RoomState);
        if state.members.len() == expected {
            return state;
        }
    }
}

fn recv_lobby_member_count(c: &mut WireClient, expected: u8) -> LobbyList {
    loop {
        let list: LobbyList = recv_of(c, MessageKind::LobbyList);
        if list
            .rooms
            .first()
            .is_some_and(|r| r.member_count == expected)
        {
            return list;
        }
    }
}

fn recv_snapshot(c: &mut WireClient) -> Snapshot {
    recv_of(c, MessageKind::Snapshot)
}

fn recv_input_rejected(c: &mut WireClient) -> InputRejected {
    loop {
        let frame = recv_raw(c);
        let (envelope, _) = read_envelope(&frame).unwrap();
        if envelope.kind == MessageKind::InputRejected {
            return decode_frame(&frame, &ProtocolLimits::v1()).unwrap();
        }
    }
}

fn server_with_clients(players: &[PlayerId]) -> (Server, Vec<WireClient>) {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    let mut server = Server::new(a, h);
    let clients: Vec<WireClient> = players
        .iter()
        .enumerate()
        .map(|(slot, &player)| wire_client(&mut server, player, slot as u8))
        .collect();
    (server, clients)
}

fn wire_handshake(server: &mut Server, clients: &mut [WireClient]) {
    let h = {
        let setup = setup();
        let a = Authority::new(ServerConfig::v1(), setup);
        handshake_config(&a)
    };
    for c in clients.iter_mut() {
        let msg = hello(h.rules_hash);
        send(c, &msg);
        let _ = server.poll(0, Duration::from_millis(100)).unwrap();
        let _: Welcome = recv(c);
    }
}

#[test]
fn handshake_accepts_valid_and_rejects_wrong_rules() {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    let mut server = Server::new(a, h.clone());
    let mut good = wire_client(&mut server, PlayerId(1), 0);
    let mut bad = wire_client(&mut server, PlayerId(2), 1);

    let good_hello = hello(h.rules_hash);
    send(&mut good, &good_hello);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let welcome: Welcome = recv(&mut good);
    assert_eq!(welcome.player_id, 0);
    assert_eq!(welcome.rules_hash, h.rules_hash);

    let bad_hello = hello(h.rules_hash ^ 1);
    send(&mut bad, &bad_hello);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut bad);
    assert_eq!(reject.reason, RejectReason::RulesMismatch);

    let mut wrong_content = wire_client(&mut server, PlayerId(3), 2);
    let mut content_hello = hello(h.rules_hash);
    content_hello.content_sha256[0] ^= 1;
    send(&mut wrong_content, &content_hello);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut wrong_content);
    assert_eq!(reject.reason, RejectReason::ContentMismatch);
    assert_eq!(server.session_count(), 1);
}

#[test]
fn wrong_wire_schema_is_rejected() {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    let mut server = Server::new(a, h);
    let mut c = wire_client(&mut server, PlayerId(1), 0);
    let mut bad = hello(0);
    bad.wire_schema = 2;
    send(&mut c, &bad);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut c);
    assert_eq!(reject.reason, RejectReason::UnsupportedWireSchema);

    let mut c2 = wire_client(&mut server, PlayerId(2), 1);
    let mut bad_state = hello(0);
    bad_state.state_schema = 2;
    send(&mut c2, &bad_state);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut c2);
    assert_eq!(reject.reason, RejectReason::UnsupportedStateSchema);
}

#[test]
fn memory_transport_input_dedup_and_window() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    // Room/lifecycle chưa có DTO trong boom-proto; dùng API authority đã bind session.
    let mut req = Requests::new(&players);
    let host = players[0];
    server
        .room_command(
            command(
                host,
                req.take(host),
                RoomAction::Create {
                    character: CharacterId::C00,
                },
            ),
            0,
        )
        .unwrap();
    let room = server.room_view().unwrap().id;
    server
        .room_command(
            command(
                players[1],
                req.take(players[1]),
                RoomAction::Join {
                    room,
                    character: CharacterId::C01,
                },
            ),
            0,
        )
        .unwrap();
    for &player in &players {
        server
            .room_command(
                command(
                    player,
                    req.take(player),
                    RoomAction::SetReady { ready: true },
                ),
                0,
            )
            .unwrap();
    }
    server
        .room_command(command(host, req.take(host), RoomAction::Start), 0)
        .unwrap();
    for &player in &players {
        server.ack_loading(player, 0).unwrap();
    }
    assert_eq!(server.phase(), ServerPhase::Playing);

    let round = round_u32(server.authority());
    let batch = InputBatch {
        round_id: round,
        player: 0,
        ack_snapshot: 0,
        commands: vec![InputCommand {
            seq: 1,
            target_tick: 0,
            bits: InputBits::new(4, false).unwrap(),
        }],
    };
    send(&mut clients[0], &batch);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let first: Snapshot = recv_snapshot(&mut clients[0]);
    assert_eq!(first.round_id, round);
    let tick = first.tick;
    // Cùng batch gửi lại: không nhân state/tick.
    let before_tick = server.authority().tick();
    send(&mut clients[0], &batch);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    assert_eq!(server.authority().tick(), before_tick);
    assert_eq!(server.authority().applied_seq(players[0]), Some(1));

    // Ngoài cửa sổ: InputRejected, state không đổi.
    let window = InputBatch {
        round_id: round,
        player: 0,
        ack_snapshot: 0,
        commands: vec![InputCommand {
            seq: 2,
            target_tick: tick.saturating_add(1_000),
            bits: InputBits::new(4, false).unwrap(),
        }],
    };
    send(&mut clients[0], &window);
    let _ = server.poll(16, Duration::from_millis(200)).unwrap();
    let rejected = recv_input_rejected(&mut clients[0]);
    assert_eq!(rejected.reason, InputRejectReason::OutOfWindow);
}

#[test]
fn memory_transport_two_and_four_clients_two_rounds() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        let (mut server, mut clients) = server_with_clients(&players);
        wire_handshake(&mut server, &mut clients);
        let mut req = Requests::new(&players);
        let host = players[0];
        server
            .room_command(
                command(
                    host,
                    req.take(host),
                    RoomAction::Create {
                        character: CharacterId::C00,
                    },
                ),
                0,
            )
            .unwrap();
        let room = server.room_view().unwrap().id;
        for &player in players.iter().skip(1) {
            server
                .room_command(
                    command(
                        player,
                        req.take(player),
                        RoomAction::Join {
                            room,
                            character: CharacterId::C01,
                        },
                    ),
                    0,
                )
                .unwrap();
        }
        for &player in &players {
            server
                .room_command(
                    command(
                        player,
                        req.take(player),
                        RoomAction::SetReady { ready: true },
                    ),
                    0,
                )
                .unwrap();
        }
        server
            .room_command(command(host, req.take(host), RoomAction::Start), 0)
            .unwrap();
        for &player in &players {
            server.ack_loading(player, 0).unwrap();
        }
        let round1 = round_u32(server.authority());
        for (slot, c) in clients.iter_mut().enumerate() {
            send(
                c,
                &InputBatch {
                    round_id: round1,
                    player: slot as u8,
                    ack_snapshot: 0,
                    commands: vec![InputCommand {
                        seq: 1,
                        target_tick: server.authority().tick().unwrap_or(0).saturating_add(1),
                        bits: InputBits::new(4, false).unwrap(),
                    }],
                },
            );
        }
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        for c in clients.iter_mut() {
            let _: Snapshot = recv_snapshot(c);
        }
        server.poll(200_000, Duration::ZERO).unwrap();
        assert_eq!(server.phase(), ServerPhase::Results);
        assert_eq!(server.last_result().unwrap().outcome, MatchOutcome::Draw);
        for &player in &players {
            server.ack_result(player, 200_000).unwrap();
        }
        assert_eq!(server.phase(), ServerPhase::Waiting);
        // Round 2: ready/start trực tiếp, input qua transport.
        let base = 100;
        for (i, &player) in players.iter().enumerate() {
            server
                .room_command(
                    command(
                        player,
                        RequestId(base + i as u64),
                        RoomAction::SetReady { ready: true },
                    ),
                    200_000,
                )
                .unwrap();
        }
        server
            .room_command(
                command(host, RequestId(base + 90), RoomAction::Start),
                200_000,
            )
            .unwrap();
        for &player in &players {
            server.ack_loading(player, 200_000).unwrap();
        }
        let round2 = round_u32(server.authority());
        assert_ne!(round1, round2);
        for (slot, c) in clients.iter_mut().enumerate() {
            send(
                c,
                &InputBatch {
                    round_id: round2,
                    player: slot as u8,
                    ack_snapshot: 0,
                    commands: vec![InputCommand {
                        seq: 1,
                        target_tick: server.authority().tick().unwrap_or(0).saturating_add(1),
                        bits: InputBits::new(4, false).unwrap(),
                    }],
                },
            );
        }
        let _ = server.poll(200_000, Duration::from_millis(200)).unwrap();
        for c in clients.iter_mut() {
            let _: Snapshot = recv_snapshot(c);
        }
        for c in &mut clients {
            let _ = c.writer.close();
        }
    }
}

#[test]
fn snapshot_dto_roundtrips_state_hash_via_boom_sync() {
    let setup = setup();
    let world = setup.world.clone();
    let mut a = Authority::new(ServerConfig::v1(), setup);
    let players = [PlayerId(1), PlayerId(2)];
    let _ = create_start(&mut a, &players, 0);
    a.advance_to(16).unwrap();
    let snapshot = a.snapshot(1, 7).unwrap().unwrap();
    assert_eq!(snapshot.rules_hash, a.rules_hash());
    assert_eq!(snapshot.round_id, round_u32(&a));
    let restored = boom_sync::from_snapshot(&snapshot, &world).unwrap();
    assert_eq!(Some(restored.hash()), a.state_hash());
    assert_eq!(restored.tick, a.tick().unwrap());
}

struct SlowWriter {
    inner: MemoryTransport,
    delay: Duration,
}

impl Transport for SlowWriter {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), boom_net::NetError> {
        std::thread::sleep(self.delay);
        self.inner.send_frame(frame)
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, boom_net::NetError> {
        self.inner.recv_frame()
    }

    fn close(&mut self) -> Result<(), boom_net::NetError> {
        self.inner.close()
    }

    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

struct ShutdownState {
    closed: Mutex<bool>,
    cv: Condvar,
}

impl ShutdownState {
    fn shutdown(&self) {
        *self.closed.lock().unwrap() = true;
        self.cv.notify_all();
    }

    fn wait(&self) {
        let mut closed = self.closed.lock().unwrap();
        while !*closed {
            closed = self.cv.wait(closed).unwrap();
        }
    }

    fn is_closed(&self) -> bool {
        *self.closed.lock().unwrap()
    }
}

struct BlockingReader {
    frames: Arc<Mutex<VecDeque<Vec<u8>>>>,
    state: Arc<ShutdownState>,
    finished: Arc<AtomicBool>,
}

impl Transport for BlockingReader {
    fn send_frame(&mut self, _frame: &[u8]) -> Result<(), boom_net::NetError> {
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, boom_net::NetError> {
        loop {
            if let Some(frame) = self.frames.lock().unwrap().pop_front() {
                return Ok(Some(frame));
            }
            let closed = self.state.closed.lock().unwrap();
            if *closed {
                self.finished.store(true, Ordering::SeqCst);
                return Ok(None);
            }
            let (_guard, _) = self
                .state
                .cv
                .wait_timeout(closed, Duration::from_millis(1))
                .unwrap();
        }
    }

    fn close(&mut self) -> Result<(), boom_net::NetError> {
        self.state.shutdown();
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.state.is_closed()
    }
}

struct BlockingWriter {
    state: Arc<ShutdownState>,
    finished: Arc<AtomicBool>,
}

impl Transport for BlockingWriter {
    fn send_frame(&mut self, _frame: &[u8]) -> Result<(), boom_net::NetError> {
        self.state.wait();
        self.finished.store(true, Ordering::SeqCst);
        Err(boom_net::NetError::Closed)
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, boom_net::NetError> {
        Ok(None)
    }

    fn close(&mut self) -> Result<(), boom_net::NetError> {
        self.state.shutdown();
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.state.is_closed()
    }
}

#[test]
fn closed_session_shuts_down_stuck_transport_and_threads() {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    let mut server = Server::new(a, h.clone());
    let state = Arc::new(ShutdownState {
        closed: Mutex::new(false),
        cv: Condvar::new(),
    });
    let frames = Arc::new(Mutex::new(VecDeque::new()));
    let reader_done = Arc::new(AtomicBool::new(false));
    let writer_done = Arc::new(AtomicBool::new(false));
    let shutdown_called = Arc::new(AtomicBool::new(false));

    let reader = BlockingReader {
        frames: frames.clone(),
        state: state.clone(),
        finished: reader_done.clone(),
    };
    let writer = BlockingWriter {
        state: state.clone(),
        finished: writer_done.clone(),
    };
    let state_for_shutdown = state.clone();
    let called = shutdown_called.clone();
    server
        .accept_with_shutdown(PlayerId(1), 0, reader, writer, move || {
            called.store(true, Ordering::SeqCst);
            state_for_shutdown.shutdown();
        })
        .unwrap();

    let msg = hello(h.rules_hash);
    let frame = encode_frame(&msg, &ProtocolLimits::v1()).unwrap();
    frames.lock().unwrap().push_back(frame);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    // 40 Ping vượt queue 32; session bị loại phải gọi shutdown độc lập writer đang kẹt.
    for i in 0..40u64 {
        let frame = encode_frame(&Ping(i), &ProtocolLimits::v1()).unwrap();
        frames.lock().unwrap().push_back(frame);
    }
    for _ in 0..100 {
        if server.session_count() == 0 {
            break;
        }
        let _ = server.poll(0, Duration::from_millis(20)).unwrap();
    }
    assert_eq!(server.session_count(), 0);
    assert!(shutdown_called.load(Ordering::SeqCst));
    let deadline = Instant::now() + Duration::from_secs(2);
    while (!reader_done.load(Ordering::SeqCst) || !writer_done.load(Ordering::SeqCst))
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        reader_done.load(Ordering::SeqCst),
        "reader thread chưa thoát"
    );
    assert!(
        writer_done.load(Ordering::SeqCst),
        "writer thread chưa thoát"
    );
}

#[test]
fn slow_writer_does_not_block_tick() {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    let mut server = Server::new(a, h.clone());

    let (mut slow_client_writer, slow_server_reader) =
        MemoryTransport::pair(MemoryConfig::default());
    let (slow_server_writer, _slow_client_reader) = MemoryTransport::pair(MemoryConfig::default());
    server
        .accept(
            PlayerId(1),
            0,
            slow_server_reader,
            SlowWriter {
                inner: slow_server_writer,
                delay: Duration::from_millis(200),
            },
        )
        .unwrap();
    let mut normal = wire_client(&mut server, PlayerId(2), 1);
    let msg = hello(h.rules_hash);
    let frame = encode_frame(&msg, &ProtocolLimits::v1()).unwrap();
    slow_client_writer.send_frame(&frame).unwrap();
    let normal_hello = hello(h.rules_hash);
    send(&mut normal, &normal_hello);

    let start = Instant::now();
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    assert!(start.elapsed() < Duration::from_millis(100));
    // Reader thread của client thường có thể tới sau lượt poll đầu; một lượt nữa xử lý nốt.
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let _: Welcome = recv(&mut normal);

    // Dựng trận trực tiếp (room DTO chưa có), rồi jump thời gian: writer chậm không được giữ tick.
    let players = [PlayerId(1), PlayerId(2)];
    let mut req = Requests::new(&players);
    let host = players[0];
    server
        .room_command(
            command(
                host,
                req.take(host),
                RoomAction::Create {
                    character: CharacterId::C00,
                },
            ),
            0,
        )
        .unwrap();
    let room = server.room_view().unwrap().id;
    server
        .room_command(
            command(
                players[1],
                req.take(players[1]),
                RoomAction::Join {
                    room,
                    character: CharacterId::C01,
                },
            ),
            0,
        )
        .unwrap();
    for &player in &players {
        server
            .room_command(
                command(
                    player,
                    req.take(player),
                    RoomAction::SetReady { ready: true },
                ),
                0,
            )
            .unwrap();
    }
    server
        .room_command(command(host, req.take(host), RoomAction::Start), 0)
        .unwrap();
    for &player in &players {
        server.ack_loading(player, 0).unwrap();
    }
    let start = Instant::now();
    server.poll(200_000, Duration::ZERO).unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(server.last_result().is_some());
}

#[test]
fn bounded_outbound_queue_disconnects_slow_client() {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    let mut server = Server::new(a, h.clone());
    let (mut client_writer, server_reader) = MemoryTransport::pair(MemoryConfig::default());
    let (server_writer, _client_reader) = MemoryTransport::pair(MemoryConfig::default());
    server
        .accept(
            PlayerId(1),
            0,
            server_reader,
            SlowWriter {
                inner: server_writer,
                delay: Duration::from_secs(1),
            },
        )
        .unwrap();
    let msg = hello(h.rules_hash);
    let frame = encode_frame(&msg, &ProtocolLimits::v1()).unwrap();
    client_writer.send_frame(&frame).unwrap();
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    assert_eq!(server.session_count(), 1);
    // Writer đang kẹt ở Welcome; 40 Pong vượt queue 32 → session bị ngắt, không chặn poll.
    for i in 0..40u64 {
        let frame = encode_frame(&Ping(i), &ProtocolLimits::v1()).unwrap();
        client_writer.send_frame(&frame).unwrap();
    }
    let start = Instant::now();
    for _ in 0..100 {
        if server.session_count() == 0 {
            break;
        }
        let _ = server.poll(0, Duration::from_millis(20)).unwrap();
    }
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(server.session_count(), 0);
}

#[test]
fn malformed_frame_closes_only_that_session() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    let mut req = Requests::new(&players);
    let host = players[0];
    server
        .room_command(
            command(
                host,
                req.take(host),
                RoomAction::Create {
                    character: CharacterId::C00,
                },
            ),
            0,
        )
        .unwrap();
    let room = server.room_view().unwrap().id;
    server
        .room_command(
            command(
                players[1],
                req.take(players[1]),
                RoomAction::Join {
                    room,
                    character: CharacterId::C01,
                },
            ),
            0,
        )
        .unwrap();
    for &player in &players {
        server
            .room_command(
                command(
                    player,
                    req.take(player),
                    RoomAction::SetReady { ready: true },
                ),
                0,
            )
            .unwrap();
    }
    server
        .room_command(command(host, req.take(host), RoomAction::Start), 0)
        .unwrap();
    for &player in &players {
        server.ack_loading(player, 0).unwrap();
    }
    let round = round_u32(server.authority());
    // Client xấu gửi envelope rác; client tốt vẫn gửi input hợp lệ cùng lượt.
    clients[1].writer.send_frame(b"bad envelope").unwrap();
    let target_tick = server.authority().tick().unwrap_or(0);
    let good = InputBatch {
        round_id: round,
        player: 0,
        ack_snapshot: 0,
        commands: vec![InputCommand {
            seq: 1,
            target_tick,
            bits: InputBits::new(4, false).unwrap(),
        }],
    };
    send(&mut clients[0], &good);
    for i in 0..100u64 {
        if server.session_count() == 1 && server.authority().applied_seq(players[0]) == Some(1) {
            break;
        }
        let _events = server.poll(16 + i * 16, Duration::from_millis(20)).unwrap();
    }
    assert_eq!(server.session_count(), 1);
    assert_eq!(server.authority().applied_seq(players[0]), Some(1));
    let snapshot: Snapshot = recv_snapshot(&mut clients[0]);
    assert_eq!(snapshot.round_id, round);
    assert!(snapshot.tick >= target_tick);
}

#[test]
fn malformed_payload_closes_only_that_session() {
    let players = [PlayerId(1), PlayerId(2), PlayerId(3)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    let mut req = Requests::new(&players);
    let host = players[0];
    server
        .room_command(
            command(
                host,
                req.take(host),
                RoomAction::Create {
                    character: CharacterId::C00,
                },
            ),
            0,
        )
        .unwrap();
    let room = server.room_view().unwrap().id;
    server
        .room_command(
            command(
                players[2],
                req.take(players[2]),
                RoomAction::Join {
                    room,
                    character: CharacterId::C01,
                },
            ),
            0,
        )
        .unwrap();
    for player in [players[0], players[2]] {
        server
            .room_command(
                command(
                    player,
                    req.take(player),
                    RoomAction::SetReady { ready: true },
                ),
                0,
            )
            .unwrap();
    }
    server
        .room_command(command(host, req.take(host), RoomAction::Start), 0)
        .unwrap();
    for player in [players[0], players[2]] {
        server.ack_loading(player, 0).unwrap();
    }
    let round = round_u32(server.authority());
    // Envelope và payload_len hợp lệ, nhưng byte payload cuối (bits) sai DTO.
    let bad = InputBatch {
        round_id: round,
        player: 1,
        ack_snapshot: 0,
        commands: vec![InputCommand {
            seq: 1,
            target_tick: 0,
            bits: InputBits::new(4, false).unwrap(),
        }],
    };
    let mut frame = encode_frame(&bad, &ProtocolLimits::v1()).unwrap();
    let last = frame.len() - 1;
    frame[last] = 0xFF;
    clients[1].writer.send_frame(&frame).unwrap();
    let target_tick = server.authority().tick().unwrap_or(0);
    let good = InputBatch {
        round_id: round,
        player: 0,
        ack_snapshot: 0,
        commands: vec![InputCommand {
            seq: 1,
            target_tick,
            bits: InputBits::new(4, false).unwrap(),
        }],
    };
    send(&mut clients[0], &good);
    for i in 0..100u64 {
        if server.session_count() == 2 && server.authority().applied_seq(players[0]) == Some(1) {
            break;
        }
        let _ = server.poll(16 + i * 16, Duration::from_millis(20)).unwrap();
    }
    assert_eq!(server.session_count(), 2);
    assert_eq!(server.authority().applied_seq(players[0]), Some(1));
    let snapshot: Snapshot = recv_snapshot(&mut clients[0]);
    assert_eq!(snapshot.round_id, round);
    assert!(snapshot.tick >= target_tick);
}

#[test]
fn tcp_reject_reaches_client_before_eof() {
    // Regression review #60 vòng 3: đóng session sau `Reject` phải để writer gửi xong frame, không
    // shutdown socket ngay (trước đây client TCP thường chỉ thấy EOF, mất lý do từ chối).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let a = Authority::new(ServerConfig::v1(), setup());
    let h = handshake_config(&a);
    let mut server = Server::new(a, h.clone());
    for round in 0..20u64 {
        let player = PlayerId(100 + round);
        let mut client = boom_net::TcpTransport::connect(addr).unwrap();
        server.accept_tcp(&listener, player, 0).unwrap();
        let frame = encode_frame(&hello(h.rules_hash ^ 1), &ProtocolLimits::v1()).unwrap();
        client.send_frame(&frame).unwrap();
        let start = Instant::now();
        while server.session_count() > 0 {
            let _ = server.poll(0, Duration::from_millis(20)).unwrap();
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "server không xử lý Hello"
            );
        }
        let frame = client
            .recv_frame()
            .unwrap()
            .expect("client phải nhận Reject trước EOF");
        let reject: Reject = decode_frame(&frame, &ProtocolLimits::v1()).unwrap();
        assert_eq!(reject.reason, RejectReason::RulesMismatch);
        assert!(
            matches!(client.recv_frame(), Ok(None) | Err(_)),
            "sau Reject là EOF"
        );
    }
}

#[test]
fn room_kind_before_auth_is_rejected_and_after_auth_works() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);

    // Chưa AuthOk: LobbyListRequest là kind phòng, server Reject rồi đóng.
    send(&mut clients[0], &LobbyListRequest {});
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut clients[0]);
    assert_eq!(reject.reason, RejectReason::SessionRejected);

    authenticate(&mut server, &[players[1]]);
    send(&mut clients[1], &LobbyListRequest {});
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let list: LobbyList = recv_lobby(&mut clients[1]);
    assert!(list.rooms.is_empty());
}

#[test]
fn wire_room_commands_are_idempotent_and_chat_is_room_scoped() {
    let players = [PlayerId(1), PlayerId(2), PlayerId(3)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    authenticate(&mut server, &players);

    send_room(
        &mut clients[1],
        1,
        RoomActionDto::Join {
            room: 1,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let rejected: RoomRejected = recv_of(&mut clients[1], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::RoomNotFound);

    send_room(
        &mut clients[0],
        1,
        RoomActionDto::Create {
            character: RoomCharacter::C00,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let state = recv_room_state(&mut clients[0]);
    let room = state.room_id;

    // Cùng request lặp: không tạo phòng/member lần hai.
    send_room(
        &mut clients[0],
        1,
        RoomActionDto::Create {
            character: RoomCharacter::C00,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let state = recv_room_state(&mut clients[0]);
    assert_eq!(state.members.len(), 1);

    send_room(
        &mut clients[1],
        1,
        RoomActionDto::Join {
            room,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let joined = recv_room_state(&mut clients[1]);
    assert_eq!(joined.members.len(), 2);

    // Client 3 không ở phòng: không nhận chat.
    send(
        &mut clients[0],
        &RoomChat {
            request: 1,
            text: "hello".into(),
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let msg: RoomChatMessage = recv_of(&mut clients[1], MessageKind::RoomChatMessage);
    assert_eq!(msg.room_id, room);
    assert_eq!(msg.text, "hello");
    while let Some(frame) = clients[2].reader.try_recv_frame().unwrap() {
        let (envelope, _) = read_envelope(&frame).unwrap();
        assert_ne!(
            envelope.kind,
            MessageKind::RoomChatMessage,
            "client ngoài phòng không được nhận chat"
        );
    }

    // Rate limit: gửi 6 tin trong cùng cửa sổ; chỉ tối đa MAX_CHAT_PER_S được phát.
    for request in 2..=7 {
        send(
            &mut clients[0],
            &RoomChat {
                request,
                text: "spam".into(),
            },
        );
    }
    for i in 0..50 {
        let _ = server.poll(i, Duration::from_millis(20)).unwrap();
    }
    let mut delivered = 0;
    while let Some(frame) = clients[1].reader.try_recv_frame().unwrap() {
        let (envelope, _) = read_envelope(&frame).unwrap();
        if envelope.kind == MessageKind::RoomChatMessage {
            delivered += 1;
        }
    }
    assert!(
        delivered <= ProtocolLimits::v1().max_chat_per_s as usize,
        "chat vượt rate bị phát {delivered} lần"
    );
}

#[test]
fn wire_room_full_lifecycle_two_and_four_clients() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        let (mut server, mut clients) = server_with_clients(&players);
        wire_handshake(&mut server, &mut clients);
        authenticate(&mut server, &players);

        send(&mut clients[0], &LobbyListRequest {});
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let _: LobbyList = recv_lobby(&mut clients[0]);

        send_room(
            &mut clients[0],
            1,
            RoomActionDto::Create {
                character: RoomCharacter::C00,
            },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let state = recv_room_state(&mut clients[0]);
        let room = state.room_id;
        let lobby: LobbyList = recv_lobby(&mut clients[0]);
        assert_eq!(lobby.rooms[0].member_count, 1);
        for (index, &player) in players.iter().enumerate().skip(1) {
            send_room(
                &mut clients[index],
                1,
                RoomActionDto::Join {
                    room,
                    character: RoomCharacter::C01,
                },
            );
            let _ = server.poll(0, Duration::from_millis(200)).unwrap();
            let joined = recv_room_state(&mut clients[index]);
            assert_eq!(joined.members.len(), index + 1);
            let lobby: LobbyList = recv_lobby(&mut clients[0]);
            assert_eq!(lobby.rooms[0].member_count, (index + 1) as u8);
            let _ = player;
        }
        for (index, _) in players.iter().enumerate() {
            send_room(
                &mut clients[index],
                2,
                RoomActionDto::SetReady { ready: true },
            );
            let _ = server.poll(0, Duration::from_millis(200)).unwrap();
            let _ = recv_room_state(&mut clients[0]);
        }
        send_room(&mut clients[0], 3, RoomActionDto::Start);
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let start: RoundStart = recv_of(&mut clients[0], MessageKind::RoundStart);
        assert_eq!(start.roster.len(), players.len());
        assert_eq!(start.mode, "battle_ffa");
        assert_eq!(start.map, "skeleton-test");
        let round = start.round_id;
        for (index, _) in players.iter().enumerate() {
            send(&mut clients[index], &LoadingAck { round_id: round });
        }
        for _ in 0..100 {
            let _ = server.poll(0, Duration::from_millis(10)).unwrap();
            if server.phase() == ServerPhase::Playing {
                break;
            }
        }
        assert_eq!(server.phase(), ServerPhase::Playing);

        for (index, _) in players.iter().enumerate() {
            send(
                &mut clients[index],
                &InputBatch {
                    round_id: u32::try_from(round).unwrap(),
                    player: index as u8,
                    ack_snapshot: 0,
                    commands: vec![InputCommand {
                        seq: 1,
                        target_tick: server.authority().tick().unwrap_or(0).saturating_add(1),
                        bits: InputBits::new(4, false).unwrap(),
                    }],
                },
            );
        }
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        for c in clients.iter_mut() {
            let _: Snapshot = recv_snapshot(c);
        }

        let _ = server.poll(200_000, Duration::ZERO).unwrap();
        assert_eq!(server.phase(), ServerPhase::Results);
        for c in clients.iter_mut() {
            let result: RoundResult = recv_of(c, MessageKind::RoundResult);
            assert_eq!(result.round_id, round);
        }
        for (index, _) in players.iter().enumerate() {
            send(&mut clients[index], &ResultAck { round_id: round });
        }
        for _ in 0..100 {
            let _ = server.poll(200_000, Duration::from_millis(20)).unwrap();
            if server.phase() == ServerPhase::Waiting {
                break;
            }
        }
        assert_eq!(server.phase(), ServerPhase::Waiting);
        let waiting = recv_room_state(&mut clients[0]);
        assert_eq!(waiting.room_id, room);
        assert_eq!(waiting.members.len(), players.len());

        // Trận 2: ready lại, start, loading ack; round ID tăng.
        for (index, _) in players.iter().enumerate() {
            send_room(
                &mut clients[index],
                4,
                RoomActionDto::SetReady { ready: true },
            );
            let _ = server.poll(200_000, Duration::from_millis(200)).unwrap();
        }
        send_room(&mut clients[0], 5, RoomActionDto::Start);
        let _ = server.poll(200_000, Duration::from_millis(200)).unwrap();
        let start2: RoundStart = recv_of(&mut clients[0], MessageKind::RoundStart);
        assert_ne!(start2.round_id, round);
        for (index, _) in players.iter().enumerate() {
            send(
                &mut clients[index],
                &LoadingAck {
                    round_id: start2.round_id,
                },
            );
        }
        for _ in 0..100 {
            let _ = server.poll(200_000, Duration::from_millis(10)).unwrap();
            if server.phase() == ServerPhase::Playing {
                break;
            }
        }
        assert_eq!(server.phase(), ServerPhase::Playing);
        for (index, _) in players.iter().enumerate() {
            send(
                &mut clients[index],
                &InputBatch {
                    round_id: u32::try_from(start2.round_id).unwrap(),
                    player: index as u8,
                    ack_snapshot: 0,
                    commands: vec![InputCommand {
                        seq: 1,
                        target_tick: server.authority().tick().unwrap_or(0).saturating_add(1),
                        bits: InputBits::new(4, false).unwrap(),
                    }],
                },
            );
        }
        let _ = server.poll(200_016, Duration::from_millis(200)).unwrap();
        for c in clients.iter_mut() {
            let _: Snapshot = recv_snapshot(c);
        }
        let _ = server.poll(400_000, Duration::ZERO).unwrap();
        assert_eq!(server.phase(), ServerPhase::Results);
        for c in clients.iter_mut() {
            let result: RoundResult = recv_of(c, MessageKind::RoundResult);
            assert_eq!(result.round_id, start2.round_id);
        }
        for (index, _) in players.iter().enumerate() {
            send(
                &mut clients[index],
                &ResultAck {
                    round_id: start2.round_id,
                },
            );
        }
        for _ in 0..100 {
            let _ = server.poll(400_000, Duration::from_millis(20)).unwrap();
            if server.phase() == ServerPhase::Waiting {
                break;
            }
        }
        assert_eq!(server.phase(), ServerPhase::Waiting);
        for c in clients.iter_mut() {
            let _ = c.writer.close();
        }
    }
}

#[test]
fn round_result_life_is_authoritative_not_outcome() {
    let players = [PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)];
    let mut a = authority();
    let _round = create_start(&mut a, &players, 0);
    // Player 4 mất kết nối quá timeout: bị loại/Dead, còn 3 người sống tới hết giờ → hòa.
    a.disconnect(players[3], 0).unwrap();
    a.advance_to(10_001).unwrap();
    a.advance_to(200_000).unwrap();
    assert_eq!(a.last_result().unwrap().outcome, MatchOutcome::Draw);
    assert_eq!(a.round_player_life(players[3]), Some(Life::Dead));
    for player in &players[..3] {
        assert!(
            !matches!(a.round_player_life(*player), Some(Life::Dead)),
            "player {player:?} còn sống"
        );
    }
}

#[test]
fn ack_with_wrong_round_does_not_advance_or_finish() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    authenticate(&mut server, &players);

    send_room(
        &mut clients[1],
        1,
        RoomActionDto::Join {
            room: 1,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let rejected: RoomRejected = recv_of(&mut clients[1], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::RoomNotFound);

    send_room(
        &mut clients[0],
        1,
        RoomActionDto::Create {
            character: RoomCharacter::C00,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let room = recv_room_state(&mut clients[0]).room_id;
    send_room(
        &mut clients[1],
        1,
        RoomActionDto::Join {
            room,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let _ = recv_room_state(&mut clients[1]);
    for (index, _) in players.iter().enumerate() {
        send_room(
            &mut clients[index],
            2,
            RoomActionDto::SetReady { ready: true },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let _ = recv_room_state(&mut clients[0]);
    }
    send_room(&mut clients[0], 3, RoomActionDto::Start);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let start: RoundStart = recv_of(&mut clients[0], MessageKind::RoundStart);

    send(
        &mut clients[0],
        &LoadingAck {
            round_id: start.round_id + 1,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    assert_eq!(server.phase(), ServerPhase::Loading);
    let rejected: RoomRejected = recv_of(&mut clients[0], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::NotWaiting);

    for (index, _) in players.iter().enumerate() {
        send(
            &mut clients[index],
            &LoadingAck {
                round_id: start.round_id,
            },
        );
    }
    for _ in 0..100 {
        let _ = server.poll(0, Duration::from_millis(10)).unwrap();
        if server.phase() == ServerPhase::Playing {
            break;
        }
    }
    assert_eq!(server.phase(), ServerPhase::Playing);
    // Ack lặp sau barrier idempotent.
    send(
        &mut clients[0],
        &LoadingAck {
            round_id: start.round_id,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    assert_eq!(server.phase(), ServerPhase::Playing);

    let _ = server.poll(200_000, Duration::ZERO).unwrap();
    assert_eq!(server.phase(), ServerPhase::Results);
    let _: RoundResult = recv_of(&mut clients[0], MessageKind::RoundResult);
    send(
        &mut clients[0],
        &ResultAck {
            round_id: start.round_id + 1,
        },
    );
    let _ = server.poll(200_000, Duration::from_millis(200)).unwrap();
    assert_eq!(server.phase(), ServerPhase::Results);
    let rejected: RoomRejected = recv_of(&mut clients[0], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::NotWaiting);

    for (index, _) in players.iter().enumerate() {
        send(
            &mut clients[index],
            &ResultAck {
                round_id: start.round_id,
            },
        );
    }
    for _ in 0..100 {
        let _ = server.poll(200_000, Duration::from_millis(10)).unwrap();
        if server.phase() == ServerPhase::Waiting {
            break;
        }
    }
    assert_eq!(server.phase(), ServerPhase::Waiting);
    // ResultAck lặp sau khi về phòng idempotent.
    send(
        &mut clients[0],
        &ResultAck {
            round_id: start.round_id,
        },
    );
    let _ = server.poll(200_000, Duration::from_millis(200)).unwrap();
    assert_eq!(server.phase(), ServerPhase::Waiting);
}

#[test]
fn expected_room_errors_are_rejected_without_killing_poll() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    authenticate(&mut server, &players);

    send_room(
        &mut clients[1],
        1,
        RoomActionDto::Join {
            room: 1,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let rejected: RoomRejected = recv_of(&mut clients[1], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::RoomNotFound);

    send_room(
        &mut clients[0],
        1,
        RoomActionDto::Create {
            character: RoomCharacter::C00,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let _ = recv_room_state(&mut clients[0]);

    // Create thứ hai: authority trả RoomExists, server phải RoomRejected và tiếp tục poll.
    send_room(
        &mut clients[0],
        2,
        RoomActionDto::Create {
            character: RoomCharacter::C00,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let rejected: RoomRejected = recv_of(&mut clients[0], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::AlreadyStarted);
    send(&mut clients[1], &LobbyListRequest {});
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let list: LobbyList = recv_lobby(&mut clients[1]);
    assert_eq!(list.rooms.len(), 1);

    // Join room lạ: RoomNotFound, không thoát poll.
    send_room(
        &mut clients[1],
        1,
        RoomActionDto::Join {
            room: 999,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let rejected: RoomRejected = recv_of(&mut clients[1], MessageKind::RoomRejected);
    assert_eq!(rejected.reason, RoomRejectReason::RoomNotFound);

    // Start lặp cùng request sau khi đã Start: duplicate idempotent, không đóng session.
    let room = list.rooms[0].room_id;
    send_room(
        &mut clients[1],
        2,
        RoomActionDto::Join {
            room,
            character: RoomCharacter::C01,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let _ = recv_room_state(&mut clients[1]);
    for (index, _) in players.iter().enumerate() {
        send_room(
            &mut clients[index],
            3,
            RoomActionDto::SetReady { ready: true },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    }
    send_room(&mut clients[0], 4, RoomActionDto::Start);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    send_room(&mut clients[0], 4, RoomActionDto::Start);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    send(&mut clients[0], &LobbyListRequest {});
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let _: LobbyList = recv_lobby(&mut clients[0]);
}

fn drain_client(c: &mut WireClient) {
    while c.reader.try_recv_frame().unwrap().is_some() {}
}

#[test]
fn disconnect_broadcasts_roster_waiting_and_loading() {
    let players = [PlayerId(1), PlayerId(2)];
    for loading in [false, true] {
        let (mut server, mut clients) = server_with_clients(&players);
        wire_handshake(&mut server, &mut clients);
        authenticate(&mut server, &players);
        send_room(
            &mut clients[0],
            1,
            RoomActionDto::Create {
                character: RoomCharacter::C00,
            },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let room = recv_room_state(&mut clients[0]).room_id;
        send_room(
            &mut clients[1],
            1,
            RoomActionDto::Join {
                room,
                character: RoomCharacter::C01,
            },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let _ = recv_room_state(&mut clients[1]);
        for (index, _) in players.iter().enumerate() {
            send_room(
                &mut clients[index],
                2,
                RoomActionDto::SetReady { ready: true },
            );
            let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        }
        if loading {
            send_room(&mut clients[0], 3, RoomActionDto::Start);
            let _ = server.poll(0, Duration::from_millis(200)).unwrap();
            let _: RoundStart = recv_of(&mut clients[0], MessageKind::RoundStart);
        } else {
            // Waiting: ready vẫn true; disconnect phải reset roster về 1 người.
            send_room(&mut clients[0], 3, RoomActionDto::SetReady { ready: false });
            let _ = server.poll(0, Duration::from_millis(200)).unwrap();
            let _ = recv_room_state(&mut clients[0]);
        }
        drain_client(&mut clients[0]);
        clients[1].writer.close().unwrap();
        clients[1].reader.close().unwrap();
        for _ in 0..500 {
            let _ = server.poll(0, Duration::from_millis(20)).unwrap();
            if server.session_count() == 1 {
                break;
            }
        }
        assert_eq!(server.session_count(), 1);
        let state = recv_room_state_members(&mut clients[0], 1);
        assert_eq!(state.members[0].player, players[0].0);
        let lobby = recv_lobby_member_count(&mut clients[0], 1);
        assert_eq!(lobby.rooms[0].member_count, 1);
        if loading {
            assert_eq!(server.phase(), ServerPhase::Waiting);
            assert!(!state.members[0].ready, "loading reset ready");
        }
    }
}

fn lossy_memory(seed: u64, loss_permille: u16) -> MemoryConfig {
    MemoryConfig {
        seed,
        faults: boom_net::MemoryFaults {
            loss_permille,
            delay_ms: 5,
            jitter_ms: 10,
            ..boom_net::MemoryFaults::none()
        },
        timing: boom_net::Timing::Virtual,
    }
}

fn lossy_handshake(server: &mut Server, clients: &mut [WireClient], h: &HandshakeConfig) {
    for c in clients.iter_mut() {
        let mut welcome = None;
        for _ in 0..300 {
            send(c, &hello(h.rules_hash));
            let _ = server.poll(0, Duration::from_millis(20)).unwrap();
            if let Some(msg) =
                try_recv_of::<Welcome>(c, MessageKind::Welcome, Duration::from_millis(250))
            {
                welcome = Some(msg);
                break;
            }
        }
        assert!(welcome.is_some(), "không nhận Welcome qua mạng lossy");
    }
}

fn retry_room_state(
    server: &mut Server,
    client: &mut WireClient,
    request: u64,
    action: RoomActionDto,
    now: u64,
) -> RoomState {
    for _ in 0..300 {
        send_room(client, request, action.clone());
        let _ = server.poll(now, Duration::from_millis(20)).unwrap();
        if let Some(state) =
            try_recv_of::<RoomState>(client, MessageKind::RoomState, Duration::from_millis(250))
        {
            return state;
        }
    }
    panic!("không nhận RoomState sau retry");
}

fn retry_ready(server: &mut Server, client: &mut WireClient, player: u64, request: u64, now: u64) {
    for _ in 0..300 {
        send_room(client, request, RoomActionDto::SetReady { ready: true });
        let _ = server.poll(now, Duration::from_millis(20)).unwrap();
        if let Some(state) =
            try_recv_of::<RoomState>(client, MessageKind::RoomState, Duration::from_millis(250))
            && state.members.iter().any(|m| m.player == player && m.ready)
        {
            return;
        }
    }
    panic!("ready không tới server sau retry");
}

fn retry_start(server: &mut Server, client: &mut WireClient, request: u64, now: u64) -> RoundStart {
    for _ in 0..300 {
        send_room(client, request, RoomActionDto::Start);
        let _ = server.poll(now, Duration::from_millis(20)).unwrap();
        if let Some(start) =
            try_recv_of::<RoundStart>(client, MessageKind::RoundStart, Duration::from_millis(250))
        {
            return start;
        }
    }
    panic!("Start/RoundStart không qua mạng lossy");
}

fn retry_loading(server: &mut Server, clients: &mut [WireClient], round: u64, now: u64) {
    for _ in 0..300 {
        for c in clients.iter_mut() {
            send(c, &LoadingAck { round_id: round });
        }
        let _ = server.poll(now, Duration::from_millis(20)).unwrap();
        if server.phase() == ServerPhase::Playing {
            return;
        }
    }
    panic!("LoadingAck không vượt barrier sau retry");
}

fn retry_input(
    server: &mut Server,
    clients: &mut [WireClient],
    players: &[PlayerId],
    round: u64,
    now: u64,
) {
    for attempt in 0..100 {
        for (index, _) in players.iter().enumerate() {
            send(
                &mut clients[index],
                &InputBatch {
                    round_id: u32::try_from(round).unwrap(),
                    player: index as u8,
                    ack_snapshot: 0,
                    commands: vec![InputCommand {
                        seq: 1,
                        target_tick: server.authority().tick().unwrap_or(0).saturating_add(1),
                        bits: InputBits::new(4, false).unwrap(),
                    }],
                },
            );
        }
        let _ = server
            .poll(
                now.saturating_add(34 * (attempt + 1)),
                Duration::from_millis(20),
            )
            .unwrap();
        if players
            .iter()
            .all(|player| server.authority().applied_seq(*player) == Some(1))
        {
            for c in clients.iter_mut() {
                let _: Snapshot = recv_of(c, MessageKind::Snapshot);
            }
            return;
        }
    }
    panic!("input không tới server sau retry");
}

fn retry_result_ack(server: &mut Server, clients: &mut [WireClient], round: u64, now: u64) {
    for _ in 0..300 {
        for c in clients.iter_mut() {
            send(c, &ResultAck { round_id: round });
        }
        let _ = server.poll(now, Duration::from_millis(20)).unwrap();
        if server.phase() == ServerPhase::Waiting {
            return;
        }
    }
    panic!("ResultAck không về Waiting sau retry");
}

#[test]
fn wire_room_full_lifecycle_with_delay_loss_seed() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        let setup = setup();
        let a = Authority::new(ServerConfig::v1(), setup);
        let h = handshake_config(&a);
        let mut server = Server::new(a, h.clone());
        let mut clients: Vec<WireClient> = players
            .iter()
            .enumerate()
            .map(|(slot, &player)| {
                wire_client_with_configs(
                    &mut server,
                    player,
                    slot as u8,
                    lossy_memory(0xBEEF_5EED, 100),
                    lossy_memory(0xBEEF_5EED, 0),
                )
            })
            .collect();
        lossy_handshake(&mut server, &mut clients, &h);
        authenticate(&mut server, &players);

        let mut lobby = None;
        for _ in 0..100 {
            send(&mut clients[0], &LobbyListRequest {});
            let _ = server.poll(0, Duration::from_millis(20)).unwrap();
            if let Some(list) = try_recv_of::<LobbyList>(
                &mut clients[0],
                MessageKind::LobbyList,
                Duration::from_millis(50),
            ) {
                lobby = Some(list);
                break;
            }
        }
        assert!(lobby.is_some(), "không nhận LobbyList lossy");

        let state = retry_room_state(
            &mut server,
            &mut clients[0],
            1,
            RoomActionDto::Create {
                character: RoomCharacter::C00,
            },
            0,
        );
        let room = state.room_id;
        for (index, _player) in players.iter().enumerate().skip(1) {
            let joined = retry_room_state(
                &mut server,
                &mut clients[index],
                1,
                RoomActionDto::Join {
                    room,
                    character: RoomCharacter::C01,
                },
                0,
            );
            assert_eq!(joined.members.len(), index + 1);
        }
        for (index, &player) in players.iter().enumerate() {
            retry_ready(&mut server, &mut clients[index], player.0, 2, 0);
        }
        let start = retry_start(&mut server, &mut clients[0], 3, 0);
        assert_eq!(start.roster.len(), players.len());
        let round = start.round_id;
        retry_loading(&mut server, &mut clients, round, 0);
        retry_input(&mut server, &mut clients, &players, round, 0);

        let _ = server.poll(200_000, Duration::ZERO).unwrap();
        for c in clients.iter_mut() {
            let result: RoundResult = recv_of(c, MessageKind::RoundResult);
            assert_eq!(result.round_id, round);
        }
        retry_result_ack(&mut server, &mut clients, round, 200_000);
        let waiting: RoomState = recv_room_state(&mut clients[0]);
        assert_eq!(waiting.members.len(), players.len());

        // Trận 2 qua cùng mạng lossy.
        for (index, &player) in players.iter().enumerate() {
            retry_ready(&mut server, &mut clients[index], player.0, 4, 200_000);
        }
        let start2 = retry_start(&mut server, &mut clients[0], 5, 200_000);
        assert_ne!(start2.round_id, round);
        retry_loading(&mut server, &mut clients, start2.round_id, 200_000);
        retry_input(
            &mut server,
            &mut clients,
            &players,
            start2.round_id,
            200_016,
        );
        let _ = server.poll(400_000, Duration::ZERO).unwrap();
        for c in clients.iter_mut() {
            let result: RoundResult = recv_of(c, MessageKind::RoundResult);
            assert_eq!(result.round_id, start2.round_id);
        }
        retry_result_ack(&mut server, &mut clients, start2.round_id, 400_000);
    }
}

#[test]
fn wire_round_result_alive_uses_authoritative_state_in_draw() {
    let players = [PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    authenticate(&mut server, &players);
    send_room(
        &mut clients[0],
        1,
        RoomActionDto::Create {
            character: RoomCharacter::C00,
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let room = recv_room_state(&mut clients[0]).room_id;
    for (index, _) in players.iter().enumerate().skip(1) {
        send_room(
            &mut clients[index],
            1,
            RoomActionDto::Join {
                room,
                character: RoomCharacter::C01,
            },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let _ = recv_room_state(&mut clients[index]);
    }
    for (index, _) in players.iter().enumerate() {
        send_room(
            &mut clients[index],
            2,
            RoomActionDto::SetReady { ready: true },
        );
        let _ = server.poll(0, Duration::from_millis(200)).unwrap();
        let _ = recv_room_state(&mut clients[0]);
    }
    send_room(&mut clients[0], 3, RoomActionDto::Start);
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let start: RoundStart = recv_of(&mut clients[0], MessageKind::RoundStart);
    for (index, _) in players.iter().enumerate() {
        send(
            &mut clients[index],
            &LoadingAck {
                round_id: start.round_id,
            },
        );
    }
    for _ in 0..100 {
        let _ = server.poll(0, Duration::from_millis(10)).unwrap();
        if server.phase() == ServerPhase::Playing {
            break;
        }
    }
    assert_eq!(server.phase(), ServerPhase::Playing);

    // Player 4 rời transport; quá disconnect timeout bị Dead, 3 người còn lại hết giờ → hòa.
    clients[3].writer.close().unwrap();
    for _ in 0..100 {
        let _ = server.poll(0, Duration::from_millis(10)).unwrap();
        if server.session_count() == 3 {
            break;
        }
    }
    let _ = server.poll(10_001, Duration::ZERO).unwrap();
    let _ = server.poll(200_000, Duration::ZERO).unwrap();
    assert_eq!(server.phase(), ServerPhase::Results);
    let result: RoundResult = recv_of(&mut clients[0], MessageKind::RoundResult);
    assert_eq!(result.outcome, boom_proto::RoomOutcome::Draw);
    let dead = result
        .results
        .iter()
        .find(|r| r.player == players[3].0)
        .expect("kết quả player 4");
    assert!(!dead.alive);
    assert!(
        result
            .results
            .iter()
            .filter(|r| r.player != players[3].0)
            .all(|r| r.alive)
    );
}
