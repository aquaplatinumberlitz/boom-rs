use std::collections::BTreeMap;

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, Dir, Stat, World, mode_by_id};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand, RoundId};

use crate::{
    Authority, CharacterStatsTable, InputError, InternalInputBatch, InternalInputCommand,
    MatchOutcome, MatchSetup, ServerConfig, ServerPhase,
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

fn create_start(a: &mut Authority, players: &[PlayerId], now: u64) -> RoundId {
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
    let round = a.room_view().unwrap().round.unwrap().id;
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
    let batch = InternalInputBatch {
        player: players[0],
        round: a.room_view().unwrap().round.unwrap().id,
        commands: vec![InternalInputCommand {
            seq: 1,
            target_tick: 0,
            dir: Dir::Right,
            place_bomb: false,
        }],
    };
    let ack = a.submit_input(batch.clone(), 0).unwrap();
    assert_eq!(ack.applied_seq, 0);
    let hash = a.state_hash().unwrap();
    // Cùng seq nhưng payload khác trong pending là BadSequence, state không đổi.
    let mut altered = batch.clone();
    altered.commands[0].target_tick = 1;
    assert!(matches!(
        a.submit_input(altered, 0),
        Err(InputError::BadSequence { seq: 1 })
    ));
    assert_eq!(a.state_hash(), Some(hash));

    a.advance_to(16).unwrap();
    let after_apply = a.state_hash().unwrap();
    let dup = a.submit_input(batch, 16).unwrap();
    assert_eq!(dup.applied_seq, 1);
    assert!(dup.retired.is_empty());
    assert_eq!(a.state_hash(), Some(after_apply));
    a.advance_to(32).unwrap();
    assert!(a.player_position(players[0]).is_some());

    let before_window = a.state_hash().unwrap();
    let window = InternalInputBatch {
        player: players[1],
        round: a.room_view().unwrap().round.unwrap().id,
        commands: vec![InternalInputCommand {
            seq: 1,
            target_tick: 100,
            dir: Dir::Left,
            place_bomb: false,
        }],
    };
    assert!(matches!(
        a.submit_input(window, 32),
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
    let batch = InternalInputBatch {
        player: players[0],
        round: a.room_view().unwrap().round.unwrap().id,
        commands: vec![InternalInputCommand {
            seq: 1,
            target_tick: tick.saturating_sub(20),
            dir: Dir::Right,
            place_bomb: false,
        }],
    };
    let ack = a.submit_input(batch, 200).unwrap();
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
    let round = a.room_view().unwrap().round.unwrap().id;
    // Bỏ seq 1, gửi seq 2; gap timeout phải retire seq 1 rồi áp seq 2.
    let batch = InternalInputBatch {
        player: players[0],
        round,
        commands: vec![InternalInputCommand {
            seq: 2,
            target_tick: 2,
            dir: Dir::Right,
            place_bomb: false,
        }],
    };
    a.submit_input(batch, 0).unwrap();
    let events = a.advance_to(200).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::InternalServerEvent::InputRetired { seq: 1, .. }))
    );
    let ack = a
        .submit_input(
            InternalInputBatch {
                player: players[0],
                round,
                commands: vec![InternalInputCommand {
                    seq: 3,
                    target_tick: 20,
                    dir: Dir::Left,
                    place_bomb: false,
                }],
            },
            200,
        )
        .unwrap();
    assert_eq!(ack.applied_seq, 2);
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
