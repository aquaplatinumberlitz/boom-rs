use std::collections::{BTreeMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use boom_content::map::{Terrain, is_walkable};
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, CharacterStatsTable, Dir, Life, Stat, World, mode_by_id};
use boom_proto::{InputBatch, InputBits, InputCommand};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand, RoundId};

use crate::{
    Authority, InputError, MatchOutcome, MatchSetup, ReplaySetup, ServerConfig, ServerPhase,
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
        replay: None,
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
use boom_proto::dto::auth::{Auth, AuthFail, AuthFailReason, AuthOk, AuthOp, Logout};
use boom_proto::dto::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use boom_proto::{
    Hello, InputRejectReason, InputRejected, LoadingAck, LobbyList, LobbyListRequest, Message,
    MessageKind, Ping, Pong, Reject, RejectReason, ResultAck, RoomAction as RoomActionDto,
    RoomCharacter, RoomChat, RoomChatMessage, RoomCommand as RoomCommandDto, RoomRejectReason,
    RoomRejected, RoomState, RoundResult, RoundStart, Snapshot, SnapshotRequest, Welcome,
    decode_frame, encode_frame, limits::ProtocolLimits, read_envelope,
};

use crate::auth::{AuthConfig, AuthService};
use crate::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use crate::{HandshakeConfig, Server};

struct WireClient {
    writer: MemoryTransport,
    reader: MemoryTransport,
}

/// Tổng số vòng chờ cho một yêu cầu trong kịch bản mạng lỗi.
///
/// Trần là **số vòng**, không phải thời gian thật: tốc độ máy và độ phân giải timer của hệ
/// điều hành (Windows `sleep(1 ms)` thực tế ~15.6 ms) không ảnh hưởng kết quả kiểm.
const WAIT_ROUNDS: usize = 200_000;

/// Số lần gửi tối đa cho một yêu cầu.
///
/// Gửi lại **chỉ** khi mạng thật sự nuốt frame — quyết định lấy từ PRNG theo seed nên biết
/// *tức thì*, không phải đoán theo hạn thời gian thật.
const SEND_ATTEMPTS: usize = 300;

/// Lấy frame kế tiếp nếu đúng loại `kind`; frame khác loại bị bỏ (không chặn).
fn take_kind<M: Message>(c: &mut WireClient, kind: MessageKind) -> Option<M> {
    let frame = c.reader.try_recv_frame().expect("đọc frame")?;
    let (envelope, _) = read_envelope(&frame).expect("đọc envelope");
    if envelope.kind == kind {
        Some(decode_frame(&frame, &ProtocolLimits::v1()).expect("giải mã frame"))
    } else {
        None
    }
}

/// Gửi frame trên chiều client→server; trả `true` nếu mạng đã nuốt frame (mất).
///
/// Quyết định mất lấy từ PRNG theo seed ⇒ biết **tức thì**, không cần chờ theo thời gian
/// thật. Đây là mấu chốt để test không phụ thuộc tốc độ máy: chỉ gửi lại khi frame thật sự
/// bị mất, nên server không dồn câu trả lời vô hạn (hàng đợi gửi đi của server có trần; đầy
/// thì bị coi như consumer chậm và session bị đóng — xem `Server::send_raw`).
fn send_tracked<M: Message>(c: &mut WireClient, message: &M) -> bool {
    let before = c.writer.dropped_frames();
    send(c, message);
    c.writer.dropped_frames() > before
}

/// Gửi yêu cầu tới server và chờ câu trả lời loại `kind`.
///
/// - Yêu cầu bị mạng nuốt → gửi lại ngay, không tốn ngân sách chờ.
/// - Câu trả lời bị mạng nuốt → gửi lại yêu cầu.
/// - Yêu cầu tới nơi → chờ theo **điều kiện**, trần số vòng (`budget`), mỗi vòng bơm server
///   bằng `poll` không chặn rồi `yield_now` (không ngủ theo timer hệ điều hành).
///
/// `accept` lọc câu trả lời: frame không thoả điều kiện bị bỏ và vẫn chờ tiếp, không gửi lại.
fn ask_where<Req: Message, Rep: Message, F: FnMut(&Rep) -> bool>(
    server: &mut Server,
    client: &mut WireClient,
    now_ms: u64,
    budget: &mut usize,
    message: &Req,
    kind: MessageKind,
    mut accept: F,
) -> Option<Rep> {
    for _ in 0..SEND_ATTEMPTS {
        let dropped_out = client.writer.dropped_frames();
        send(client, message);
        if client.writer.dropped_frames() > dropped_out {
            // Mạng nuốt yêu cầu: gửi lại ngay.
            continue;
        }
        // Số frame **server gửi** đã bị nuốt, đo sau khi gửi yêu cầu: server chưa xử lý yêu
        // cầu này nên mọi frame bị nuốt từ đây trở đi là câu trả lời của yêu cầu này.
        let dropped_in = client.reader.peer_dropped_frames();
        while *budget > 0 {
            *budget -= 1;
            server.poll(now_ms, Duration::ZERO).expect("poll server");
            if let Some(reply) = take_kind::<Rep>(client, kind)
                && accept(&reply)
            {
                return Some(reply);
            }
            if client.reader.peer_dropped_frames() > dropped_in {
                // Mạng nuốt câu trả lời: gửi lại yêu cầu.
                break;
            }
            std::thread::yield_now();
        }
        if *budget == 0 {
            // Hết ngân sách chờ: không gửi thêm (server sẽ dồn câu trả lời vô hạn).
            break;
        }
    }
    None
}

/// Như [`ask_where`] nhưng không lọc câu trả lời.
fn ask<Req: Message, Rep: Message>(
    server: &mut Server,
    client: &mut WireClient,
    now_ms: u64,
    budget: &mut usize,
    message: &Req,
    kind: MessageKind,
) -> Option<Rep> {
    ask_where(server, client, now_ms, budget, message, kind, |_| true)
}

/// Chờ luồng đọc của server **lấy hết** frame client đã gửi — **không tiến đồng hồ server**.
///
/// Đồng hồ server đứng yên ⇒ không tick nào chạy ⇒ lệnh input đang chờ trong hàng đợi
/// của authority **không bị retire** vì đồng hồ chạy trước khi lệnh tới server
/// (ADR 0009 §3.3: lệnh cũ hơn `server_tick - late` bị retire, không áp dụng).
///
/// [`MemoryTransport`] đếm frame **đưa lên đường** ([`MemoryTransport::sent_frames`]) và
/// frame **đã giao cho ứng dụng nhận** ([`MemoryTransport::peer_delivered_frames`]): mỗi
/// frame đưa lên đường tới nơi đúng một lần, nên hai số bằng nhau ⇔ mọi frame client gửi
/// đã tới tay luồng đọc của server. Trả `true` nếu đủ, `false` nếu hết ngân sách chờ.
fn wait_frames_delivered(
    server: &mut Server,
    clients: &mut [WireClient],
    now: u64,
    budget: &mut usize,
) -> bool {
    while *budget > 0 {
        if clients
            .iter()
            .all(|c| c.writer.peer_delivered_frames() >= c.writer.sent_frames())
        {
            return true;
        }
        *budget -= 1;
        server.poll(now, Duration::ZERO).expect("poll server");
        std::thread::yield_now();
    }
    clients
        .iter()
        .all(|c| c.writer.peer_delivered_frames() >= c.writer.sent_frames())
}

/// Hướng chắc chắn **đi được** từ ô người chơi đang đứng (ô kề bên là nền).
///
/// Bản đồ test có tường ở biên nên một hướng cố định có thể bị chặn ⇒ vị trí không đổi
/// dù lệnh đã áp dụng. Chọn hướng theo địa hình để **vị trí chắc chắn đổi** khi lệnh
/// được áp dụng thật.
fn walkable_dir(world: &World, x: i32, y: i32) -> u8 {
    let tile = world.rules().tile.0;
    let (cx, cy) = (x.div_euclid(tile), y.div_euclid(tile));
    // (mã hướng, delta ô) theo thứ tự ưu tiên: phải, xuống, trái, lên.
    for (code, (dx, dy)) in [
        (DIR_RIGHT, (1, 0)),
        (DIR_DOWN, (0, 1)),
        (DIR_LEFT, (-1, 0)),
        (DIR_UP, (0, -1)),
    ] {
        if world
            .terrain()
            .kind(cx + dx, cy + dy)
            .is_some_and(is_walkable)
        {
            return code;
        }
    }
    panic!("ô ({cx},{cy}) không có hướng nào đi được");
}

/// Rào `Ping` → `Pong`: chứng minh server đã **xử lý** batch input gửi trước đó.
///
/// Luồng đọc của server lấy frame theo thứ tự và server xử lý kênh vào theo thứ tự FIFO
/// ⇒ `Pong` (trả lời `Ping` gửi **sau** input) chứng minh batch input đã được xử lý.
///
/// `Ping` gửi sau khi batch input **đã tới nơi** ([`wait_frames_delivered`]) ⇒ nhờ đồng
/// hồ ảo của mạng đã chạy tới mốc giao của batch, `Ping` luôn tới **sau** batch, kể cả
/// khi mạng có jitter (thứ tự giao theo mốc giao, không theo thứ tự gửi).
///
/// Server **từ chối** input là lỗi thật ⇒ báo ngay kèm lý do thay vì chờ mãi.
fn wait_pong_barrier(server: &mut Server, client: &mut WireClient, now: u64, budget: &mut usize) {
    for _ in 0..SEND_ATTEMPTS {
        // Số frame **server gửi** đã bị nuốt, đo trước khi gửi `Ping`.
        let dropped_in = client.reader.peer_dropped_frames();
        if send_tracked(client, &Ping(now)) {
            // Mạng nuốt `Ping`: gửi lại ngay (server chưa thấy frame này).
            continue;
        }
        while *budget > 0 {
            *budget -= 1;
            server.poll(now, Duration::ZERO).expect("poll server");
            let Some(frame) = client.reader.try_recv_frame().expect("đọc frame") else {
                std::thread::yield_now();
                continue;
            };
            let (envelope, _) = read_envelope(&frame).expect("đọc envelope");
            match envelope.kind {
                MessageKind::Pong => return,
                MessageKind::InputRejected => {
                    let rejected: InputRejected =
                        decode_frame(&frame, &ProtocolLimits::v1()).expect("InputRejected");
                    panic!(
                        "server từ chối input: {:?} (seq đầu {}, số lệnh {}, tick server {})",
                        rejected.reason,
                        rejected.first_seq,
                        rejected.command_count,
                        rejected.server_tick
                    );
                }
                _ => {}
            }
            if client.reader.peer_dropped_frames() > dropped_in {
                // Mạng nuốt câu trả lời: gửi lại `Ping`.
                break;
            }
        }
        if *budget == 0 {
            // Hết ngân sách chờ: không gửi thêm.
            break;
        }
    }
    panic!("không nhận Pong (rào) sau khi gửi input");
}

/// Xin `Snapshot` cho tới khi `accept` đúng — bỏ các snapshot cũ đã queue trước đó.
///
/// `SnapshotRequest` idempotent nên gửi lại được. Yêu cầu bị mạng nuốt → gửi lại ngay;
/// câu trả lời bị nuốt → gửi lại yêu cầu. Hết ngân sách thì trả `None`.
fn wait_snapshot_where<F: FnMut(&Snapshot) -> bool>(
    server: &mut Server,
    client: &mut WireClient,
    now: u64,
    budget: &mut usize,
    request: &SnapshotRequest,
    mut accept: F,
) -> Option<Snapshot> {
    for _ in 0..SEND_ATTEMPTS {
        // Số frame **server gửi** đã bị nuốt, đo trước khi gửi yêu cầu.
        let dropped_in = client.reader.peer_dropped_frames();
        if send_tracked(client, request) {
            // Mạng nuốt yêu cầu: gửi lại ngay.
            continue;
        }
        while *budget > 0 {
            *budget -= 1;
            server.poll(now, Duration::ZERO).expect("poll server");
            if let Some(snapshot) = take_kind::<Snapshot>(client, MessageKind::Snapshot)
                && accept(&snapshot)
            {
                return Some(snapshot);
            }
            if client.reader.peer_dropped_frames() > dropped_in {
                // Mạng nuốt câu trả lời: gửi lại yêu cầu.
                break;
            }
            std::thread::yield_now();
        }
        if *budget == 0 {
            // Hết ngân sách chờ: không gửi thêm.
            break;
        }
    }
    None
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

const TEST_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

fn wire_client_with_configs(
    server: &mut Server,
    player: PlayerId,
    slot: u8,
    to_server: MemoryConfig,
    to_client: MemoryConfig,
) -> WireClient {
    wire_client_from(server, player, slot, TEST_IP, to_server, to_client)
}

fn wire_client_from(
    server: &mut Server,
    player: PlayerId,
    slot: u8,
    source_ip: IpAddr,
    to_server: MemoryConfig,
    to_client: MemoryConfig,
) -> WireClient {
    let (writer, server_reader) = MemoryTransport::pair(to_server);
    let (server_writer, reader) = MemoryTransport::pair(to_client);
    server
        .accept(player, slot, source_ip, server_reader, server_writer)
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
    for _ in 0..WAIT_ROUNDS {
        if let Some(frame) = c.reader.try_recv_frame().expect("đọc frame") {
            return frame;
        }
        std::thread::yield_now();
    }
    panic!("timeout chờ frame");
}

fn recv<M: Message>(c: &mut WireClient) -> M {
    let frame = recv_raw(c);
    decode_frame(&frame, &ProtocolLimits::v1()).unwrap()
}

fn recv_of<M: Message>(c: &mut WireClient, kind: MessageKind) -> M {
    for _ in 0..WAIT_ROUNDS {
        if let Some(message) = take_kind::<M>(c, kind) {
            return message;
        }
        std::thread::yield_now();
    }
    panic!("timeout chờ {kind:?}")
}

fn send_room(c: &mut WireClient, request: u64, action: RoomActionDto) {
    send(c, &RoomCommandDto { request, action });
}

fn test_credentials(player: PlayerId) -> (String, String) {
    (format!("player{}", player.0), "password123".to_owned())
}

/// Đọc một frame trả lời của bước đăng nhập.
///
/// - `Some(Ok(AuthOk))` / `Some(Err(AuthFail))`: server đã trả lời auth.
/// - `None`: frame không phải trả lời auth (bị bỏ) hoặc chưa có frame nào.
///
/// `Reject` là lỗi giao thức (ví dụ `Auth` lặp sau `AuthOk` ⇒ server đóng phiên): báo ngay
/// thay vì chờ mãi, để lỗi hiện rõ thay vì thành "không nhận LobbyList" ở bước sau.
fn recv_auth_response(client: &mut WireClient) -> Option<Result<AuthOk, AuthFailReason>> {
    let frame = client.reader.try_recv_frame().expect("đọc frame")?;
    let (envelope, _) = read_envelope(&frame).expect("đọc envelope");
    match envelope.kind {
        MessageKind::AuthOk => Some(Ok(
            decode_frame(&frame, &ProtocolLimits::v1()).expect("AuthOk")
        )),
        MessageKind::AuthFail => {
            let fail: AuthFail = decode_frame(&frame, &ProtocolLimits::v1()).expect("AuthFail");
            Some(Err(fail.reason))
        }
        MessageKind::Reject => {
            let reject: Reject = decode_frame(&frame, &ProtocolLimits::v1()).expect("Reject");
            panic!("server từ chối phiên lúc đăng nhập: {:?}", reject.reason);
        }
        _ => None,
    }
}

/// Đăng nhập một client, chịu được mạng mất frame mà **không** gửi lặp `Auth`.
///
/// `Auth` là frame **không idempotent**: server coi `Auth` lặp sau `AuthOk` là lỗi giao thức
/// (`Server::handle_frame`) ⇒ gửi `Reject` rồi **đóng phiên**, mọi yêu cầu sau đó không được
/// trả lời. Vì vậy chỉ gửi lại khi mạng **thật sự** nuốt frame (biết tức thì qua
/// [`MemoryTransport::dropped_frames`]), không gửi lại theo hạn thời gian thật.
fn auth_attempt(
    server: &mut Server,
    client: &mut WireClient,
    op: AuthOp,
    username: &str,
    password: &str,
) -> Result<AuthOk, AuthFailReason> {
    let mut budget = WAIT_ROUNDS;
    let auth = Auth {
        op,
        username: username.to_owned(),
        password: password.to_owned(),
    };
    for _ in 0..SEND_ATTEMPTS {
        if send_tracked(client, &auth) {
            // Mạng nuốt `Auth`: gửi lại ngay (server chưa thấy frame nào).
            continue;
        }
        while budget > 0 {
            budget -= 1;
            server
                .poll(server.now_ms(), Duration::ZERO)
                .expect("poll server");
            if let Some(result) = recv_auth_response(client) {
                return result;
            }
            std::thread::yield_now();
        }
        // Hết ngân sách chờ: không gửi thêm (frame đã tới server ⇒ không gửi lặp `Auth`).
        break;
    }
    panic!("không nhận AuthOk/AuthFail sau retry");
}

fn authenticate_player(server: &mut Server, client: &mut WireClient, player: PlayerId) -> AuthOk {
    let (username, password) = test_credentials(player);
    auth_attempt(server, client, AuthOp::Register, &username, &password).unwrap()
}

fn authenticate(server: &mut Server, clients: &mut [WireClient], players: &[PlayerId]) {
    for (index, &player) in players.iter().enumerate() {
        let ok = authenticate_player(server, &mut clients[index], player);
        // Định danh người chơi trong phòng là `account_id` (ADR 0009 Addendum B §B.2); DB tạm
        // mới mỗi test nên account id cấp theo thứ tự đăng ký — test dùng chung số thứ tự
        // cho cả định danh kết nối lẫn định danh người chơi trong phòng.
        assert_eq!(
            ok.account_id, player.0,
            "account id phải trùng định danh kết nối để test dùng chung một số thứ tự"
        );
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
    server_with_clients_config(players, u64::MAX, u64::MAX)
}

fn server_with_clients_config(
    players: &[PlayerId],
    ping_interval_ms: u64,
    liveness_timeout_ms: u64,
) -> (Server, Vec<WireClient>) {
    let setup = setup();
    let a = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&a);
    // SQLite thật trong tempdir (ADR 0011 §10); `keep` để file sống qua thời gian test.
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.keep().join("auth.sqlite3");
    let store = SqliteAccountStore::open(&db_path, SqliteOptions::default()).unwrap();
    let auth = AuthService::new(
        AuthConfig::default(),
        format!("test-instance-{}", players.len()),
        Box::new(OsRandomSource),
    )
    .unwrap();
    let mut server = Server::with_auth_and_timers(
        a,
        h,
        auth,
        Box::new(store),
        0,
        ping_interval_ms,
        liveness_timeout_ms,
    )
    .unwrap();
    let clients: Vec<WireClient> = players
        .iter()
        .enumerate()
        .map(|(slot, &player)| wire_client(&mut server, player, slot as u8))
        .collect();
    (server, clients)
}

fn wire_handshake_at(server: &mut Server, clients: &mut [WireClient], now_ms: u64) -> Vec<Welcome> {
    let h = {
        let setup = setup();
        let a = Authority::new(ServerConfig::v1(), setup);
        handshake_config(&a)
    };
    let mut welcomes = Vec::new();
    for c in clients.iter_mut() {
        let msg = hello(h.rules_hash);
        send(c, &msg);
        let _ = server.poll(now_ms, Duration::from_millis(100)).unwrap();
        let welcome: Welcome = recv(c);
        welcomes.push(welcome);
    }
    welcomes
}

fn wire_handshake_capture(server: &mut Server, clients: &mut [WireClient]) -> Vec<Welcome> {
    wire_handshake_at(server, clients, 0)
}

fn wire_handshake(server: &mut Server, clients: &mut [WireClient]) {
    let _ = wire_handshake_capture(server, clients);
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
    authenticate(&mut server, &mut clients, &players);
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
fn wire_khong_gui_snapshot_khi_phong_dang_loading_barrier() {
    // ADR 0009 §4.3: "Server gửi snapshot đầu sau khi round bắt đầu" — lúc phòng còn ở loading
    // barrier thì **không** có `Snapshot` nào trên dây; vừa qua barrier thì có.
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    authenticate(&mut server, &mut clients, &players);
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
    let _ = recv_room_state(&mut clients[0]);
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
    let round = start.round_id;
    assert_eq!(server.phase(), ServerPhase::Loading);

    // Vài nhịp server (chu kỳ snapshot 16 ms): vẫn phải **không** có Snapshot nào.
    for now in [16u64, 32, 48, 64] {
        let _ = server.poll(now, Duration::from_millis(50)).unwrap();
    }
    assert_eq!(server.phase(), ServerPhase::Loading);
    // Rào bằng helper chờ-theo-điều-kiện của nhánh này ([`wait_snapshot_where`]): client
    // **xin** `Snapshot` rồi chờ theo điều kiện, trần **số vòng** (`WAIT_ROUNDS`) — không
    // phụ thuộc hạn thời gian thật (độ phân giải timer Windows ~15,6 ms).
    //
    // Ở pha loading barrier server **không** được gửi `Snapshot` (ADR 0009 §4.3) — kể cả
    // khi client xin ⇒ hết ngân sách chờ ⇒ `None`. Mọi frame `Snapshot` đã nằm sẵn trong
    // hàng đợi (server phát sai ngay lúc `Start`) cũng bị đọc ra ở đây ⇒ assert đỏ đúng
    // như trước, và cửa sổ quan sát dài hơn hẳn cửa sổ 50 ms cũ.
    for c in clients.iter_mut() {
        let mut budget = WAIT_ROUNDS;
        let request = SnapshotRequest {
            round_id: u32::try_from(round).unwrap(),
            last_snapshot_seq: 0,
        };
        assert!(
            wait_snapshot_where(&mut server, c, 64, &mut budget, &request, |_| true).is_none(),
            "server không được gửi Snapshot khi phòng còn ở loading barrier"
        );
    }

    // Qua barrier: snapshot của lượt phải tới, và lượt đã chạy (`tick > 0`).
    for c in clients.iter_mut() {
        send(c, &LoadingAck { round_id: round });
    }
    for _ in 0..100 {
        let _ = server.poll(80, Duration::from_millis(20)).unwrap();
        if server.phase() == ServerPhase::Playing {
            break;
        }
    }
    assert_eq!(server.phase(), ServerPhase::Playing);
    for c in clients.iter_mut() {
        let snapshot: Snapshot = recv_of(c, MessageKind::Snapshot);
        assert_eq!(u64::from(snapshot.round_id), round);
        assert!(snapshot.tick > 0, "snapshot đầu của lượt phải có tick > 0");
    }
}

#[test]
fn memory_transport_two_and_four_clients_two_rounds() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        let (mut server, mut clients) = server_with_clients(&players);
        wire_handshake(&mut server, &mut clients);
        authenticate(&mut server, &mut clients, &players);
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
        .accept_with_shutdown(PlayerId(1), 0, TEST_IP, reader, writer, move || {
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
    // Chờ hai luồng thoát theo **số vòng**, không theo hạn thời gian thật: tín hiệu
    // shutdown đã bắn nên luồng thoát ngay khi được xếp lịch; trần số vòng giữ test
    // không phụ thuộc tốc độ máy mà vẫn dừng khi lỗi thật.
    for _ in 0..WAIT_ROUNDS {
        if reader_done.load(Ordering::SeqCst) && writer_done.load(Ordering::SeqCst) {
            break;
        }
        std::thread::yield_now();
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
            TEST_IP,
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
            TEST_IP,
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
    authenticate(&mut server, &mut clients, &players);
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
    authenticate(&mut server, &mut clients, &players);
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

    authenticate_player(&mut server, &mut clients[1], players[1]);
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
    authenticate(&mut server, &mut clients, &players);

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
    assert_eq!(msg.from_name, "player1");
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
        authenticate(&mut server, &mut clients, &players);

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
        assert_eq!(lobby.rooms[0].name, "player1");
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
    authenticate(&mut server, &mut clients, &players);

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
    authenticate(&mut server, &mut clients, &players);

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
        authenticate(&mut server, &mut clients, &players);
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
        let mut budget = WAIT_ROUNDS;
        let welcome = ask::<Hello, Welcome>(
            server,
            c,
            0,
            &mut budget,
            &hello(h.rules_hash),
            MessageKind::Welcome,
        );
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
    let mut budget = WAIT_ROUNDS;
    let state = ask::<RoomCommandDto, RoomState>(
        server,
        client,
        now,
        &mut budget,
        &RoomCommandDto { request, action },
        MessageKind::RoomState,
    );
    state.unwrap_or_else(|| panic!("không nhận RoomState sau retry"))
}

fn retry_ready(server: &mut Server, client: &mut WireClient, player: u64, request: u64, now: u64) {
    let mut budget = WAIT_ROUNDS;
    let ready = ask_where::<RoomCommandDto, RoomState, _>(
        server,
        client,
        now,
        &mut budget,
        &RoomCommandDto {
            request,
            action: RoomActionDto::SetReady { ready: true },
        },
        MessageKind::RoomState,
        |state| state.members.iter().any(|m| m.player == player && m.ready),
    );
    assert!(ready.is_some(), "ready không tới server sau retry");
}

fn retry_start(server: &mut Server, client: &mut WireClient, request: u64, now: u64) -> RoundStart {
    let mut budget = WAIT_ROUNDS;
    let start = ask::<RoomCommandDto, RoundStart>(
        server,
        client,
        now,
        &mut budget,
        &RoomCommandDto {
            request,
            action: RoomActionDto::Start,
        },
        MessageKind::RoundStart,
    );
    start.unwrap_or_else(|| panic!("Start/RoundStart không qua mạng lossy"))
}

fn retry_loading(server: &mut Server, clients: &mut [WireClient], round: u64, now: u64) {
    let mut budget = WAIT_ROUNDS;
    for _ in 0..SEND_ATTEMPTS {
        let mut lost = false;
        for c in clients.iter_mut() {
            lost |= send_tracked(c, &LoadingAck { round_id: round });
        }
        if lost {
            // Một `LoadingAck` bị mất: gửi lại cả loạt (ack lặp là idempotent).
            continue;
        }
        while budget > 0 {
            budget -= 1;
            server.poll(now, Duration::ZERO).expect("poll server");
            if server.phase() == ServerPhase::Playing {
                return;
            }
            std::thread::yield_now();
        }
        // Hết ngân sách chờ: không gửi thêm.
        break;
    }
    panic!("LoadingAck không vượt barrier sau retry");
}

/// Gửi input cho mọi người chơi và **chứng minh lệnh đã được áp dụng**.
///
/// Không chỉ kiểm `applied_seq`: lệnh bị *retire* (quá hạn) cũng làm `applied_seq` tăng mà
/// vị trí **không đổi** (xem `late_input_is_retired_and_not_applied`), nên phải thấy **vị
/// trí người chơi đổi** và hướng đã áp dụng trong `Snapshot`.
///
/// Các bước — không bước nào phụ thuộc lịch chạy thread hay đồng hồ thật:
///
/// 1. Gửi `InputBatch` (seq = 1) với hướng **chắc chắn đi được** từ ô người chơi đứng.
///    Mạng nuốt frame thì gửi lại cả loạt (server dedup theo sequence).
/// 2. **Đồng hồ server đứng yên** chờ luồng đọc của server lấy đủ frame: không tick nào
///    chạy ⇒ lệnh đang chờ không bị retire vì đồng hồ chạy trước khi lệnh tới server.
/// 3. Rào bằng `Ping` → `Pong` cho **từng** client: luồng đọc lấy frame theo thứ tự và
///    server xử lý kênh vào theo thứ tự FIFO ⇒ `Pong` chứng minh batch input **đã được
///    server xử lý** (mỗi kết nối một hàng đợi riêng nên phải rào từng client).
/// 4. Tiến tới deadline tick kế tiếp tính từ rules và mốc bắt đầu round.
///    Assert chỉ đúng một tick chạy, không dùng số lượt poll để tiến clock.
/// 5. Kiểm state server **và** `Snapshot`: vị trí **đã đổi**, hướng đúng và ack đúng sequence.
fn retry_input(
    server: &mut Server,
    clients: &mut [WireClient],
    players: &[PlayerId],
    world: &World,
    round: u64,
    now: u64,
) {
    let mut budget = WAIT_ROUNDS;
    for _ in 0..SEND_ATTEMPTS {
        // Vị trí trước lệnh + hướng chắc chắn đi được (để vị trí chắc chắn đổi khi áp dụng).
        let mut before: Vec<(i32, i32)> = Vec::with_capacity(players.len());
        let mut dirs: Vec<u8> = Vec::with_capacity(players.len());
        for player in players {
            let (x, y) = server
                .authority()
                .player_position(*player)
                .expect("người chơi phải có vị trí trong trận");
            before.push((x, y));
            dirs.push(walkable_dir(world, x, y));
        }

        // 1. Gửi input.
        let mut lost = false;
        for (index, _) in players.iter().enumerate() {
            let batch = InputBatch {
                round_id: u32::try_from(round).unwrap(),
                player: index as u8,
                ack_snapshot: 0,
                commands: vec![InputCommand {
                    seq: 1,
                    // Áp dụng ở tick hiện tại: tick kế tiếp là tick áp dụng lệnh này.
                    target_tick: server.authority().tick().unwrap_or(0),
                    bits: InputBits::new(dirs[index], false).unwrap(),
                }],
            };
            lost |= send_tracked(&mut clients[index], &batch);
        }
        if lost {
            // Input bị mất: gửi lại cả loạt (server dedup theo sequence).
            continue;
        }

        // 2. Chờ server nhận đủ input — đồng hồ server đứng yên (không tick nào chạy).
        if !wait_frames_delivered(server, clients, now, &mut budget) {
            // Hết ngân sách chờ: không gửi thêm.
            break;
        }

        // 3. Rào `Ping` → `Pong` cho **từng** client: mỗi kết nối có hàng đợi vào riêng,
        //    nên chỉ rào client 0 không bảo đảm batch của client khác đã được xử lý.
        for client in clients.iter_mut() {
            wait_pong_barrier(server, client, now, &mut budget);
        }

        // 4. Tiến đúng một tick: tick này áp dụng lệnh input đang chờ.
        let tick_before = server.authority().tick().expect("tick đang chơi");
        let clock = now + 1_000 / u64::from(world.rules().ticks_per_second);
        server.poll(clock, Duration::ZERO).expect("poll tick input");
        let tick_after = server.authority().tick().expect("tick sau input");
        assert_eq!(tick_after, tick_before + 1, "chỉ tiến đúng một tick");

        // 5. Chứng minh lệnh **được áp dụng**: vị trí đổi, hướng đúng, ack đúng sequence.
        for (index, _) in players.iter().enumerate() {
            // State server (authority) trước khi kiểm snapshot: vị trí phải đã đổi.
            let after = server
                .authority()
                .player_position(players[index])
                .expect("người chơi phải có vị trí trong trận");
            assert_ne!(
                after, before[index],
                "lệnh input của người chơi slot {index} không được áp dụng (vị trí không đổi)"
            );
            let request = SnapshotRequest {
                round_id: u32::try_from(round).unwrap(),
                last_snapshot_seq: 0,
            };
            let snapshot = wait_snapshot_where(
                server,
                &mut clients[index],
                clock,
                &mut budget,
                &request,
                |snapshot| {
                    snapshot.round_id == u32::try_from(round).unwrap()
                        && snapshot.tick == tick_after
                        && snapshot
                            .players
                            .iter()
                            .any(|p| p.id == index as u8 && (p.x, p.y) != before[index])
                },
            )
            .unwrap_or_else(|| {
                panic!(
                    "lệnh input của người chơi slot {index} không được áp dụng (vị trí không đổi)"
                )
            });
            let state = snapshot
                .players
                .iter()
                .find(|p| p.id == index as u8)
                .expect("snapshot phải có người chơi");
            assert_eq!((state.x, state.y), after, "snapshot phải khớp authority");
            assert_eq!(
                state.facing, dirs[index],
                "hướng người chơi slot {index} phải là hướng đã gửi"
            );
            let ack = snapshot
                .acks
                .iter()
                .find(|a| a.player == index as u8)
                .expect("snapshot phải có ack input");
            assert_eq!(ack.last_seq, 1, "sequence 1 phải được giải quyết");
        }
        return;
    }
    panic!("input không tới server sau retry");
}

fn retry_result_ack(server: &mut Server, clients: &mut [WireClient], round: u64, now: u64) {
    let mut budget = WAIT_ROUNDS;
    for _ in 0..SEND_ATTEMPTS {
        let mut lost = false;
        for c in clients.iter_mut() {
            lost |= send_tracked(c, &ResultAck { round_id: round });
        }
        if lost {
            // `ResultAck` bị mất: gửi lại cả loạt (ack lặp là idempotent).
            continue;
        }
        while budget > 0 {
            budget -= 1;
            server.poll(now, Duration::ZERO).expect("poll server");
            if server.phase() == ServerPhase::Waiting {
                return;
            }
            std::thread::yield_now();
        }
        // Hết ngân sách chờ: không gửi thêm.
        break;
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
        // Giữ bản sao world để biết ô nào đi được (xác nhận input thật sự làm người chơi di chuyển).
        let world = setup.world.clone();
        let a = Authority::new(ServerConfig::v1(), setup);
        let h = handshake_config(&a);
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.keep().join("auth.sqlite3");
        let store = SqliteAccountStore::open(&db_path, SqliteOptions::default()).unwrap();
        let auth = AuthService::new(
            AuthConfig::default(),
            format!("lossy-{}", players.len()),
            Box::new(OsRandomSource),
        )
        .unwrap();
        let mut server = Server::with_auth_and_timers(
            a,
            h.clone(),
            auth,
            Box::new(store),
            0,
            u64::MAX,
            u64::MAX,
        )
        .unwrap();
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
        authenticate(&mut server, &mut clients, &players);
        // Chốt phiên còn sống sau bước auth: `Auth` lặp sẽ khiến server đóng phiên, và mọi
        // yêu cầu sau đó không được trả lời (báo lỗi ở đây rõ hơn "không nhận LobbyList").
        assert_eq!(
            server.session_count(),
            players.len(),
            "mọi phiên phải còn sau bước auth"
        );

        let lobby = {
            let mut budget = WAIT_ROUNDS;
            ask::<LobbyListRequest, LobbyList>(
                &mut server,
                &mut clients[0],
                0,
                &mut budget,
                &LobbyListRequest {},
                MessageKind::LobbyList,
            )
        };
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
        for c in clients.iter_mut() {
            drain_client(c);
        }
        for (index, &player) in players.iter().enumerate() {
            retry_ready(&mut server, &mut clients[index], player.0, 2, 0);
        }
        let start = retry_start(&mut server, &mut clients[0], 3, 0);
        assert_eq!(start.roster.len(), players.len());
        let round = start.round_id;
        retry_loading(&mut server, &mut clients, round, 0);
        retry_input(&mut server, &mut clients, &players, &world, round, 0);

        let _ = server.poll(200_000, Duration::ZERO).unwrap();
        for c in clients.iter_mut() {
            let result: RoundResult = recv_of(c, MessageKind::RoundResult);
            assert_eq!(result.round_id, round);
        }
        retry_result_ack(&mut server, &mut clients, round, 200_000);
        let waiting: RoomState = recv_room_state(&mut clients[0]);
        assert_eq!(waiting.members.len(), players.len());
        for c in clients.iter_mut() {
            drain_client(c);
        }

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
            &world,
            start2.round_id,
            200_000,
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
    authenticate(&mut server, &mut clients, &players);
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

fn setup_for_replay() -> MatchSetup {
    let mut setup = setup();
    setup.replay = Some(ReplaySetup {
        rules_path: std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/rules/skeleton-test.toml"),
        profile_id: "skeleton-test".to_owned(),
        checkpoints: vec![1, 16, 32, 64, 128, 256],
    });
    setup
}

fn replay_config(dir: &std::path::Path) -> ServerConfig {
    let mut config = ServerConfig::v1();
    config.replay_dir = Some(dir.to_path_buf());
    config
}

fn verify_replay_output(path: &std::path::Path) -> std::process::Output {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let _ = target;
    // Luôn qua cargo để binary replay được rebuild theo schema hiện tại; không dùng binary cũ.
    Command::new("cargo")
        .current_dir(&workspace)
        .args(["run", "-q", "-p", "boom-replay", "--", "verify"])
        .arg(path)
        .output()
        .expect("chạy boom-replay")
}

fn verify_replay_with_cli(path: &std::path::Path) {
    let output = verify_replay_output(path);
    assert!(
        output.status.success(),
        "boom-replay verify thất bại: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn server_writes_round_replay_that_boom_replay_verifies() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut a = Authority::new(replay_config(dir.path()), setup_for_replay());
        let round = create_start(&mut a, &players, 0);
        let tick = a.tick().unwrap();
        // Input deterministic theo seed, gửi trùng, rồi một batch ngoài cửa sổ.
        let mut rng = 0xC0FFEE_u64 ^ players.len() as u64;
        let dirs = [Dir::Up, Dir::Right, Dir::Down, Dir::Left];
        for &player in &players {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let dir = dirs[usize::try_from(rng >> 32).unwrap_or(0) % dirs.len()];
            let bomb = rng & 1 == 0;
            let batch = input_batch(&a, player, 1, tick, dir, bomb);
            a.submit_input(player, &batch, 0).unwrap();
            a.submit_input(player, &batch, 0).unwrap();
            let mut window =
                input_batch(&a, player, 2, tick.saturating_add(10_000), Dir::Left, false);
            window.commands[0].target_tick = tick.saturating_add(10_000);
            assert!(a.submit_input(player, &window, 0).is_err());
        }
        a.advance_to(200_000).unwrap();
        assert_eq!(a.phase(), ServerPhase::Results);
        let replay_path = dir.path().join(format!("round-{}.toml", round.0));
        assert!(
            replay_path.exists(),
            "thiếu replay {}",
            replay_path.display()
        );
        assert!(replay_path.with_extension("golden").exists());
        verify_replay_with_cli(&replay_path);
        // Sửa input (giữ envelope TOML) phải làm verify fail.
        let mut mutated = std::fs::read_to_string(&replay_path).unwrap();
        let prefix = "dir = \"";
        let start = mutated.find(prefix).expect("replay phải có input dir") + prefix.len();
        let end = start + mutated[start..].find('"').expect("dir kết thúc");
        let replacement = if &mutated[start..end] == "left" {
            "right"
        } else {
            "left"
        };
        mutated.replace_range(start..end, replacement);
        assert!(mutated.contains(&format!("dir = \"{replacement}\"")));
        let bad_path = dir.path().join("tampered.toml");
        std::fs::write(&bad_path, mutated).unwrap();
        std::fs::copy(
            replay_path.with_extension("golden"),
            bad_path.with_extension("golden"),
        )
        .unwrap();
        assert!(!verify_replay_output(&bad_path).status.success());
    }
}

#[test]
fn replay_is_not_written_when_dir_is_disabled() {
    let mut a = Authority::new(ServerConfig::v1(), setup_for_replay());
    let players = [PlayerId(1), PlayerId(2)];
    let _ = create_start(&mut a, &players, 0);
    a.advance_to(200_000).unwrap();
    assert_eq!(a.phase(), ServerPhase::Results);
}

#[test]
fn replay_publish_never_overwrites_existing_round_files() {
    let dir = tempfile::tempdir().unwrap();
    let old_replay = dir.path().join("round-1.toml");
    let old_golden = dir.path().join("round-1.golden");
    std::fs::write(&old_replay, b"old replay").unwrap();
    std::fs::write(&old_golden, b"old golden").unwrap();
    let players = [PlayerId(1), PlayerId(2)];
    let mut a = Authority::new(replay_config(dir.path()), setup_for_replay());
    create_start(&mut a, &players, 0);
    a.advance_to(200_000).unwrap();
    assert_eq!(std::fs::read(&old_replay).unwrap(), b"old replay");
    assert_eq!(std::fs::read(&old_golden).unwrap(), b"old golden");
    let new_replays: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            (name.starts_with("round-1-") && name.ends_with(".toml")).then_some(path)
        })
        .collect();
    assert_eq!(new_replays.len(), 1);
    verify_replay_with_cli(&new_replays[0]);
}

#[test]
fn replay_verifies_disconnect_timeout_one_and_all_players() {
    for players in [
        vec![PlayerId(1), PlayerId(2)],
        vec![PlayerId(1), PlayerId(2), PlayerId(3), PlayerId(4)],
    ] {
        for all in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut a = Authority::new(replay_config(dir.path()), setup_for_replay());
            let round = create_start(&mut a, &players, 0);
            let disconnected: &[PlayerId] = if all { &players } else { &players[1..] };
            for &player in disconnected {
                a.disconnect(player, 0).unwrap();
            }
            a.advance_to(9_000).unwrap();
            a.advance_to(10_001).unwrap();
            let replay_path = dir.path().join(format!("round-{}.toml", round.0));
            assert!(
                replay_path.exists(),
                "phải publish replay disconnect ({all}): {}",
                replay_path.display()
            );
            verify_replay_with_cli(&replay_path);
        }
    }
}

#[test]
fn replay_write_error_does_not_break_round_or_touch_existing_path() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("replay-dir-is-file");
    std::fs::write(&not_a_dir, b"sentinel").unwrap();
    let players = [PlayerId(1), PlayerId(2)];
    let mut config = replay_config(&not_a_dir);
    config.replay_dir = Some(not_a_dir.clone());
    let mut a = Authority::new(config, setup_for_replay());
    create_start(&mut a, &players, 0);
    a.advance_to(200_000).unwrap();
    assert_eq!(a.phase(), ServerPhase::Results);
    assert_eq!(std::fs::read(&not_a_dir).unwrap(), b"sentinel");
}

#[test]
fn wire_auth_welcome_fingerprint_gates_and_second_auth() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    let welcomes = wire_handshake_capture(&mut server, &mut clients);
    assert_eq!(welcomes.len(), 2);
    assert!(!welcomes[0].server_version.is_empty());
    assert_eq!(welcomes[0].content_sha256, [7; 32]);
    assert_eq!(welcomes[0].wire_schema, 1);

    // Auth trước Hello bị từ chối.
    let mut early = wire_client(&mut server, PlayerId(3), 2);
    send(
        &mut early,
        &Auth {
            op: AuthOp::Login,
            username: "early".into(),
            password: "password123".into(),
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut early);
    assert_eq!(reject.reason, RejectReason::SessionRejected);

    // Auth vượt biên username trả InvalidInput, không vào auth store.
    send(
        &mut clients[1],
        &Auth {
            op: AuthOp::Login,
            username: "a".repeat(65),
            password: "password123".into(),
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let fail: AuthFail = recv(&mut clients[1]);
    assert_eq!(fail.reason, AuthFailReason::InvalidInput);

    // Kind phòng trước AuthOk bị từ chối.
    send(&mut clients[1], &LobbyListRequest {});
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut clients[1]);
    assert_eq!(reject.reason, RejectReason::SessionRejected);

    let ok = auth_attempt(
        &mut server,
        &mut clients[0],
        AuthOp::Register,
        "wireuser",
        "password123",
    )
    .unwrap();
    assert!(ok.account_id > 0);
    assert_eq!(ok.display_name, "wireuser");
    assert_eq!(ok.token.len(), 43);
    let debug = format!("{ok:?}");
    assert!(!debug.contains(&ok.token));
    assert!(debug.contains("<redacted>"));

    // Auth lần hai sau AuthOk là lỗi giao thức.
    send(
        &mut clients[0],
        &Auth {
            op: AuthOp::Login,
            username: "wireuser".into(),
            password: "password123".into(),
        },
    );
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut clients[0]);
    assert_eq!(reject.reason, RejectReason::SessionRejected);
}

#[test]
fn wire_auth_login_active_session_disconnect_logout_and_liveness() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    let (username, password) = test_credentials(players[0]);
    auth_attempt(
        &mut server,
        &mut clients[0],
        AuthOp::Register,
        &username,
        &password,
    )
    .unwrap();

    let reason = auth_attempt(
        &mut server,
        &mut clients[1],
        AuthOp::Login,
        &username,
        "wrong-password",
    )
    .unwrap_err();
    assert_eq!(reason, AuthFailReason::InvalidCredentials);
    let reason = auth_attempt(
        &mut server,
        &mut clients[1],
        AuthOp::Login,
        &username,
        &password,
    )
    .unwrap_err();
    assert_eq!(reason, AuthFailReason::AccountHasActiveSession);
    let reason = auth_attempt(
        &mut server,
        &mut clients[1],
        AuthOp::Register,
        &username,
        &password,
    )
    .unwrap_err();
    assert_eq!(reason, AuthFailReason::UsernameUnavailable);

    // Client đầu đóng socket: phiên được giải phóng, login lại được (T4-A1).
    clients[0].writer.close().unwrap();
    clients[0].reader.close().unwrap();
    for _ in 0..300 {
        let _ = server.poll(0, Duration::from_millis(20)).unwrap();
        if server.session_count() == 1 {
            break;
        }
    }
    let ok = auth_attempt(
        &mut server,
        &mut clients[1],
        AuthOp::Login,
        &username,
        &password,
    )
    .unwrap();
    assert_eq!(ok.display_name, username);

    // Logout thu hồi đúng phiên và đóng; login lại được.
    send(&mut clients[1], &Logout {});
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    assert_eq!(server.session_count(), 0);
    let mut next = wire_client(&mut server, PlayerId(3), 2);
    wire_handshake(&mut server, std::slice::from_mut(&mut next));
    auth_attempt(&mut server, &mut next, AuthOp::Login, &username, &password).unwrap();
}

#[test]
fn wire_auth_liveness_timeout_releases_session() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients_config(&players, 100, 250);
    wire_handshake(&mut server, &mut clients);
    let (username, password) = test_credentials(players[0]);
    authenticate_player(&mut server, &mut clients[0], players[0]);
    assert_eq!(server.session_count(), 2);

    // Quá 250 ms không Pong: phiên bị giải phóng, không chờ TTL 24 h.
    let _ = server.poll(300, Duration::ZERO).unwrap();
    assert_eq!(server.session_count(), 1);

    let mut next = wire_client(&mut server, PlayerId(3), 2);
    wire_handshake_at(&mut server, std::slice::from_mut(&mut next), 300);
    auth_attempt(&mut server, &mut next, AuthOp::Login, &username, &password).unwrap();
}

#[test]
fn server_boot_cleanup_revokes_previous_instance_session() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.keep().join("boot.sqlite3");
    let username = "bootuser";
    let password = "password123";
    {
        let mut store = SqliteAccountStore::open(&db_path, SqliteOptions::default()).unwrap();
        let mut auth = AuthService::new(
            AuthConfig::default(),
            "old-instance",
            Box::new(OsRandomSource),
        )
        .unwrap();
        auth.register(
            &mut store,
            username,
            password,
            "old-connection",
            "old-conn",
            0,
        )
        .unwrap();
    }

    let store = SqliteAccountStore::open(&db_path, SqliteOptions::default()).unwrap();
    let auth = AuthService::new(
        AuthConfig::default(),
        "new-instance",
        Box::new(OsRandomSource),
    )
    .unwrap();
    let setup = setup();
    let authority = Authority::new(ServerConfig::v1(), setup);
    let h = handshake_config(&authority);
    let mut server = Server::with_auth(authority, h, auth, Box::new(store), 0).unwrap();
    let mut client = wire_client(&mut server, PlayerId(1), 0);
    wire_handshake(&mut server, std::slice::from_mut(&mut client));
    auth_attempt(&mut server, &mut client, AuthOp::Login, username, password).unwrap();
}

#[test]
fn wire_auth_pong_extends_liveness_then_timeout_closes() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients_config(&players, 100, 500);
    wire_handshake(&mut server, &mut clients);
    authenticate_player(&mut server, &mut clients[0], players[0]);

    let _ = server.poll(150, Duration::ZERO).unwrap();
    let ping: Ping = recv_of(&mut clients[0], MessageKind::Ping);
    assert_eq!(ping.0, 150);
    send(&mut clients[0], &Pong(ping.0));
    let _ = server.poll(300, Duration::from_millis(100)).unwrap();
    assert_eq!(server.session_count(), 2);
    // Pong cuối ở 300: quá 500 ms nữa không Pong ⇒ đóng.
    let _ = server.poll(900, Duration::ZERO).unwrap();
    assert_eq!(server.session_count(), 1);
}

#[test]
fn auth_rate_limit_is_per_source_ip_not_per_connection() {
    let players: Vec<PlayerId> = (1..=6).map(PlayerId).collect();
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    for (index, _) in players.iter().enumerate().take(5) {
        authenticate_player(&mut server, &mut clients[index], players[index]);
    }
    let (username, password) = test_credentials(players[5]);
    let reason = auth_attempt(
        &mut server,
        &mut clients[5],
        AuthOp::Register,
        &username,
        &password,
    )
    .unwrap_err();
    assert_eq!(reason, AuthFailReason::RateLimited);

    // IP khác không bị ảnh hưởng bởi bucket của 127.0.0.1.
    let mut other = wire_client_from(
        &mut server,
        PlayerId(7),
        6,
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
        MemoryConfig::default(),
        MemoryConfig::default(),
    );
    wire_handshake_at(&mut server, std::slice::from_mut(&mut other), 0);
    auth_attempt(
        &mut server,
        &mut other,
        AuthOp::Register,
        "other-ip-user",
        "password123",
    )
    .unwrap();
}

#[test]
fn unauthenticated_connection_has_handshake_auth_deadline() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    server.set_auth_deadline_ms(100);
    // Một client chưa Hello, một client đã Welcome nhưng chưa Auth.
    wire_handshake_at(&mut server, std::slice::from_mut(&mut clients[1]), 0);
    let _ = server.poll(150, Duration::ZERO).unwrap();
    assert_eq!(server.session_count(), 0);
}

#[test]
fn repeated_hello_is_allowed_before_auth_but_not_after() {
    let players = [PlayerId(1), PlayerId(2)];
    let (mut server, mut clients) = server_with_clients(&players);
    wire_handshake(&mut server, &mut clients);
    let h = handshake_config(server.authority());

    // Trước AuthOk: Hello lặp trả lại Welcome và không reset trạng thái auth.
    send(&mut clients[0], &hello(h.rules_hash));
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let _: Welcome = recv(&mut clients[0]);
    let (username, password) = test_credentials(players[0]);
    auth_attempt(
        &mut server,
        &mut clients[0],
        AuthOp::Register,
        &username,
        &password,
    )
    .unwrap();

    // Sau AuthOk: Hello là lỗi giao thức, đóng kết nối (Addendum A).
    send(&mut clients[0], &hello(h.rules_hash));
    let _ = server.poll(0, Duration::from_millis(200)).unwrap();
    let reject: Reject = recv(&mut clients[0]);
    assert_eq!(reject.reason, RejectReason::SessionRejected);
}
