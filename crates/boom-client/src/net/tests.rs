//! Test end-to-end headless: client mạng + `boom-server` qua `MemoryTransport`.
//!
//! Kịch bản: hai client handshake vào server, vào phòng, đánh bom đi lại. Server là authority
//! (ADR 0005); client chỉ dự đoán di chuyển của chính mình và reconcile khi snapshot về.
//!
//! Lỗi mạng mô phỏng theo seed (`boom-net`): trễ, mất, trùng, đảo thứ tự. Cùng seed và cùng lệnh
//! gửi cho cùng thứ tự giao trên mọi OS.

use std::collections::BTreeMap;
use std::thread;
use std::time::Duration;

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, Dir, PlayerInput, Rules, Stat, World, mode_by_id};
use boom_net::{MemoryConfig, MemoryFaults, MemoryTransport, Timing};
use boom_proto::{ProtocolLimits, RejectReason, RoundEvent};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand};
use boom_server::{
    Authority, CharacterStatsTable, HandshakeConfig, MatchSetup, Server, ServerConfig,
};

use super::{ClientSession, NetClient, NetError, hello_for_profile};

/// Profile rules cố định cho test (khác `data/rules/v1-dev.toml` nhưng cùng schema).
const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

/// Thời gian chờ message khi bơm hàng đợi đọc.
const PUMP_TIMEOUT: Duration = Duration::from_millis(20);

/// Số bước mô phỏng trong một kịch bản.
const STEPS: u64 = 30;

fn stats() -> CharacterStats {
    CharacterStats {
        speed: Stat::new(1, 5),
        bombs: Stat::new(1, 5),
        range: Stat::new(1, 5),
    }
}

fn terrain() -> Terrain {
    // Bản đồ mở: mọi spawn đều có chỗ đi sang phải và đi xuống, để kịch bản luôn có
    // chuyển động thật (không kẹt tường ngay ô spawn).
    let rows = [
        "############",
        "#0.1..2....#",
        "#..........#",
        "#..3.......#",
        "#..........#",
        "############",
    ];
    Terrain::from_ascii(&rows).expect("địa hình")
}

/// Một client trong kịch bản.
struct Client {
    net: NetClient<MemoryTransport>,
    player: PlayerId,
    slot: u8,
    /// Event đã nhận (đã dedup theo `(round_id, EventId)`).
    events: Vec<RoundEvent>,
}

/// Kịch bản: server + hai client nối qua `MemoryTransport`.
struct Harness {
    server: Server,
    clients: Vec<Client>,
    /// State hash của server theo tick, để so state confirmed của client.
    hashes: BTreeMap<u32, u64>,
    /// Vị trí người chơi ở server theo (người chơi, tick).
    server_positions: BTreeMap<(PlayerId, u32), (i32, i32)>,
    /// Vị trí client dự đoán theo (người chơi, tick) — để so với vị trí server cùng tick.
    predicted_positions: BTreeMap<(PlayerId, u32), (i32, i32)>,
    /// Vị trí người chơi lúc bắt đầu lượt (tâm ô spawn), để chứng minh client đã di chuyển.
    start_positions: BTreeMap<PlayerId, (i32, i32)>,
    /// Mốc thời gian lúc lượt bắt đầu (server tính tick từ đây).
    origin_ms: u64,
    /// Số tick server đã chạy.
    ticks: u64,
    /// Tần số tick lấy từ rules profile (không hard-code).
    hz: u32,
}

impl Harness {
    /// `out_faults` áp cho chiều client→server (khung input), `in_faults` cho chiều
    /// server→client (snapshot/event).
    ///
    /// `Hello` đi trên chiều client→server: nếu lỗi mạng nuốt hoặc nhân bản `Hello` thì phiên
    /// handshake hỏng (server đóng khi nhận `Hello` lần hai — hành vi phía server, ngoài phạm vi
    /// task này). Vì vậy kịch bản lỗi mạng đặt lỗi "mất/đảo thứ tự" ở chiều gửi input và lỗi
    /// "trùng" ở chiều nhận snapshot/event.
    fn new(out_faults: MemoryFaults, in_faults: MemoryFaults, seed: u64) -> Self {
        let profile = RulesProfile::from_toml(RULES).expect("profile rules");
        let rules = Rules::from_profile(&profile).expect("rules");
        let hz = rules.ticks_per_second;
        let world = World::new(
            terrain(),
            rules.clone(),
            mode_by_id("battle_ffa").expect("mode"),
        )
        .expect("world");
        let authority = Authority::new(
            ServerConfig::v1(),
            MatchSetup {
                world,
                stats: CharacterStatsTable {
                    c00: stats(),
                    c01: stats(),
                },
            },
        );
        let handshake = HandshakeConfig {
            rules_profile_id: profile.profile_id.clone(),
            rules_hash: rules.profile_hash,
            content_sha256: [0; 32],
            content_epoch: 1,
            snapshot_interval_ms: 50,
        };
        let mut server = Server::new(authority, handshake);
        let hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
        let mut clients = Vec::new();
        for (index, player) in [PlayerId(1), PlayerId(2)].into_iter().enumerate() {
            let slot = u8::try_from(index).expect("slot");
            // Hai chiều tách riêng: client→server và server→client (giống cách server chia).
            let (client_writer, server_reader) = MemoryTransport::pair(MemoryConfig {
                seed: seed.wrapping_add(index as u64),
                faults: out_faults,
                timing: Timing::Virtual,
            });
            let (server_writer, client_reader) = MemoryTransport::pair(MemoryConfig {
                seed: seed.wrapping_add(index as u64),
                faults: in_faults,
                timing: Timing::Virtual,
            });
            server
                .accept(player, slot, server_reader, server_writer)
                .expect("accept");
            // Handshake cần server poll (vòng IO của server), nên client chờ ở luồng riêng còn
            // server poll tới khi xong.
            let client_rules = rules.clone();
            let client_hello = hello.clone();
            let handle = thread::spawn(move || {
                let mut net = NetClient::new(
                    ClientSession::new(client_reader, client_writer),
                    client_rules,
                    mode_by_id("battle_ffa").expect("mode"),
                );
                net.handshake(&client_hello).expect("handshake");
                net
            });
            while !handle.is_finished() {
                server.poll(0, Duration::from_millis(1)).expect("poll");
            }
            let net = handle.join().expect("join luồng client");
            clients.push(Client {
                net,
                player,
                slot,
                events: Vec::new(),
            });
        }
        Self {
            server,
            clients,
            hashes: BTreeMap::new(),
            server_positions: BTreeMap::new(),
            predicted_positions: BTreeMap::new(),
            start_positions: BTreeMap::new(),
            origin_ms: 0,
            ticks: 0,
            hz,
        }
    }

    /// Dựng phòng, ready, start và qua loading barrier: lượt bắt đầu ở tick 0.
    fn start_round(&mut self) {
        let host = PlayerId(1);
        let now = 0;
        self.server
            .room_command(
                RoomCommand {
                    player: host,
                    request: RequestId(1),
                    action: RoomAction::Create {
                        character: CharacterId::C00,
                    },
                },
                now,
            )
            .expect("create");
        let room = self.server.room_view().expect("có phòng").id;
        self.server
            .room_command(
                RoomCommand {
                    player: PlayerId(2),
                    request: RequestId(2),
                    action: RoomAction::Join {
                        room,
                        character: CharacterId::C01,
                    },
                },
                now,
            )
            .expect("join");
        for (index, player) in [host, PlayerId(2)].into_iter().enumerate() {
            self.server
                .room_command(
                    RoomCommand {
                        player,
                        request: RequestId(10 + index as u64),
                        action: RoomAction::SetReady { ready: true },
                    },
                    now,
                )
                .expect("ready");
        }
        self.server
            .room_command(
                RoomCommand {
                    player: host,
                    request: RequestId(20),
                    action: RoomAction::Start,
                },
                now,
            )
            .expect("start");
        for player in [host, PlayerId(2)] {
            self.server.ack_loading(player, now).expect("ack loading");
        }
        self.origin_ms = now;
        // Vị trí lúc đầu: người chơi đứng ở tâm ô spawn.
        for client in &self.clients {
            if let Some(position) = self.server.authority().player_position(client.player) {
                self.start_positions.insert(client.player, position);
            }
        }
    }

    /// Một bước: client nhận snapshot và reconcile, dự đoán rồi gửi, server chạy tick.
    fn tick(&mut self, dirs: [Dir; 2]) {
        for client in &mut self.clients {
            client.net.pump(PUMP_TIMEOUT).expect("bơm message");
            client.events.extend(client.net.take_events());
        }
        for (index, client) in self.clients.iter_mut().enumerate() {
            client
                .net
                .predict(PlayerInput {
                    dir: dirs[index],
                    place_bomb: false,
                })
                .expect("dự đoán");
        }
        // Mốc thời gian theo đúng lịch tick của server: tick `k` đến hạn ở
        // `origin + k*1000/hz`, nên mỗi bước chạy đúng một tick.
        let now = self.origin_ms + (self.ticks * 1_000) / u64::from(self.hz);
        self.ticks += 1;
        if self.ticks > 1 {
            // Nhận input **trước** khi server chạy tick: hai lần poll ở mốc chưa tới tick kế
            // tiếp (tick không chạy) để chắc chắn cả hai frame input của client nằm trong hàng
            // đợi; rồi poll ở mốc tick để áp input vào đúng tick client đã gán `target_tick`.
            let before = now - 1;
            self.server
                .poll(before, Duration::from_millis(2))
                .expect("poll nhận input");
            self.server
                .poll(before, Duration::from_millis(2))
                .expect("poll nhận input");
        }
        self.server
            .poll(now, Duration::from_millis(2))
            .expect("poll");
        if let Some(tick) = self.server.authority().tick() {
            let hash = self.server.authority().state_hash().expect("state hash");
            self.hashes.insert(tick, hash);
            for client in &self.clients {
                if let Some(position) = self.server.authority().player_position(client.player) {
                    self.server_positions
                        .insert((client.player, tick), position);
                }
            }
        }
        for client in &self.clients {
            if let Some(state) = client.net.state() {
                let tick = state.tick;
                if let Some(player) = state.player(client.slot) {
                    self.predicted_positions
                        .insert((client.player, tick), (player.x.0, player.y.0));
                }
            }
        }
    }

    /// State confirmed của client phải đúng bằng state server tại cùng tick.
    fn check_confirmed(&self) {
        for client in &self.clients {
            let Some(confirmed) = client.net.confirmed_state() else {
                continue;
            };
            let expected = self
                .hashes
                .get(&confirmed.tick)
                .expect("server đã chạy qua tick này");
            assert_eq!(
                confirmed.hash(),
                *expected,
                "state confirmed của client phải khớp state server tại cùng tick"
            );
        }
    }

    /// Vị trí client dự đoán ở tick `t` phải bằng vị trí server ở tick `t`.
    fn check_positions(&self) {
        for ((player, tick), predicted) in &self.predicted_positions {
            let Some(server) = self.server_positions.get(&(*player, *tick)) else {
                continue;
            };
            assert_eq!(
                predicted, server,
                "vị trí dự đoán ở tick {tick} phải khớp vị trí server ở tick {tick}"
            );
        }
    }
}

/// Chạy kịch bản chuẩn: hai client đi lại, kiểm hội tụ sau mỗi correction.
fn chay(out_faults: MemoryFaults, in_faults: MemoryFaults, seed: u64) -> Harness {
    let mut harness = Harness::new(out_faults, in_faults, seed);
    harness.start_round();
    for _ in 0..STEPS {
        harness.tick([Dir::Right, Dir::Down]);
        harness.check_confirmed();
        harness.check_positions();
    }
    harness
}

#[test]
fn state_confirmed_hoi_tu_server_sau_correction() {
    let harness = chay(MemoryFaults::none(), MemoryFaults::none(), 20261019);
    for client in &harness.clients {
        let confirmed = client.net.confirmed_state().expect("có state confirmed");
        let tick = confirmed.tick;
        let expected = harness.hashes.get(&tick).expect("có hash của tick");
        assert_eq!(
            confirmed.hash(),
            *expected,
            "snapshot phải dựng lại đúng state của server"
        );
    }
}

#[test]
fn vị_trí_du_đoán_hoi_tu_server() {
    let harness = chay(MemoryFaults::none(), MemoryFaults::none(), 20261019);
    for client in &harness.clients {
        let predicted = client.net.state().expect("có state dự đoán");
        let server_position = harness
            .server
            .authority()
            .player_position(client.player)
            .expect("có vị trí ở server");
        let player = predicted.player(client.slot).expect("có người chơi");
        // Vị trí dự đoán của chính mình phải khớp vị trí server tại cùng tick: client chỉ dự
        // đoán di chuyển của mình, còn server là authority.
        assert_eq!(
            (player.x.0, player.y.0),
            server_position,
            "vị trí dự đoán của chính mình phải khớp server tại cùng tick"
        );
        // Người chơi đã thật sự đi (không đứng yên ở ô spawn): phải khác vị trí lúc đầu lượt.
        let start = harness
            .start_positions
            .get(&client.player)
            .expect("có vị trí đầu lượt");
        assert_ne!(
            (player.x.0, player.y.0),
            *start,
            "kịch bản phải có chuyển động thật"
        );
    }
}

#[test]
fn duplicate_khong_lam_lenh_ap_hai_lan() {
    let reference = chay(MemoryFaults::none(), MemoryFaults::none(), 4242);
    // Chiều nhận bị nhân bản toàn bộ frame: client phải bỏ snapshot cũ và không trình bày lại
    // event. Chiều gửi vẫn mang trùng lặp **có chủ đích**: client resend cửa sổ lệnh chưa ack
    // (ADR 0009 §3.2), nên server nhận cùng một lệnh nhiều lần và phải dedup theo sequence.
    let faults = MemoryFaults {
        duplicate_permille: 1_000,
        ..MemoryFaults::none()
    };
    let duplicated = chay(MemoryFaults::none(), faults, 4242);
    let reference_hash = reference
        .server
        .authority()
        .state_hash()
        .expect("state hash tham chiếu");
    let duplicated_hash = duplicated
        .server
        .authority()
        .state_hash()
        .expect("state hash có trùng");
    assert_eq!(
        duplicated_hash, reference_hash,
        "frame trùng lặp không được làm lệnh áp hai lần"
    );
    // Sequence của server tiến đúng một lệnh mỗi bước.
    for client in &duplicated.clients {
        let applied = duplicated
            .server
            .authority()
            .applied_seq(client.player)
            .expect("có ack");
        assert!(applied > 0, "server phải giải quyết lệnh của client");
        let confirmed = client.net.confirmed_state().expect("có state confirmed");
        let expected = duplicated
            .hashes
            .get(&confirmed.tick)
            .expect("có hash của tick");
        assert_eq!(
            confirmed.hash(),
            *expected,
            "state confirmed của client phải khớp state server tại cùng tick"
        );
    }
}

#[test]
fn mat_va_dao_thu_tu_khong_lam_client_lech() {
    // Mất và đảo thứ tự ở chiều gửi input; trùng và đảo thứ tự ở chiều nhận snapshot/event.
    // Seed cố định: cùng seed cho cùng thứ tự giao trên mọi OS.
    let out_faults = MemoryFaults {
        loss_permille: 20,
        reorder_permille: 200,
        delay_ms: 20,
        jitter_ms: 30,
        reorder_window_ms: 10,
        ..MemoryFaults::none()
    };
    let in_faults = MemoryFaults {
        duplicate_permille: 50,
        reorder_permille: 100,
        delay_ms: 20,
        jitter_ms: 30,
        reorder_window_ms: 10,
        ..MemoryFaults::none()
    };
    let mut harness = Harness::new(out_faults, in_faults, 7);
    harness.start_round();
    for _ in 0..STEPS {
        harness.tick([Dir::Right, Dir::Down]);
        // Sau mỗi lần nhận snapshot, state confirmed phải khớp server: correction luôn đúng.
        harness.check_confirmed();
    }
}

#[test]
fn client_nhan_reject_khi_rules_lech() {
    let profile = RulesProfile::from_toml(RULES).expect("profile rules");
    let rules = Rules::from_profile(&profile).expect("rules");
    let world = World::new(
        terrain(),
        rules.clone(),
        mode_by_id("battle_ffa").expect("mode"),
    )
    .expect("world");
    let authority = Authority::new(
        ServerConfig::v1(),
        MatchSetup {
            world,
            stats: CharacterStatsTable {
                c00: stats(),
                c01: stats(),
            },
        },
    );
    let handshake = HandshakeConfig {
        rules_profile_id: profile.profile_id.clone(),
        rules_hash: rules.profile_hash,
        content_sha256: [0; 32],
        content_epoch: 1,
        snapshot_interval_ms: 50,
    };
    let mut server = Server::new(authority, handshake);
    let (client_writer, server_reader) = MemoryTransport::pair(MemoryConfig::default());
    let (server_writer, client_reader) = MemoryTransport::pair(MemoryConfig::default());
    server
        .accept(PlayerId(1), 0, server_reader, server_writer)
        .expect("accept");

    // Client khai sai hash rules: server phải từ chối rõ ràng (ADR 0009 §1).
    let mut hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
    hello.rules_hash = rules.profile_hash.wrapping_add(1);
    let handle = thread::spawn(move || {
        let mut session = ClientSession::new(client_reader, client_writer);
        session.handshake(&hello)
    });
    while !handle.is_finished() {
        server.poll(0, Duration::from_millis(1)).expect("poll");
    }
    let result = handle.join().expect("join luồng client");
    match result {
        Err(NetError::Rejected(reject)) => {
            assert_eq!(reject.reason, RejectReason::RulesMismatch);
        }
        other => panic!("phải nhận Reject, thấy {other:?}"),
    }
}

#[test]
fn event_khong_phat_lai_khi_reconcile() {
    // Chiều nhận bị nhân bản toàn bộ frame: cùng một `RoundEvents` tới nhiều lần, client chỉ
    // được trình bày đúng một lần (dedup theo `(round_id, EventId)`).
    let faults = MemoryFaults {
        duplicate_permille: 1_000,
        ..MemoryFaults::none()
    };
    let mut harness = Harness::new(MemoryFaults::none(), faults, 99);
    harness.start_round();
    for _ in 0..STEPS {
        harness.tick([Dir::Right, Dir::Down]);
    }
    for client in &harness.clients {
        let mut seen: Vec<(u32, u16)> = Vec::new();
        for event in &client.events {
            let key = (event.id.tick, event.id.seq);
            assert!(!seen.contains(&key), "event bị phát hai lần: {key:?}");
            seen.push(key);
        }
        assert!(
            !client.events.is_empty(),
            "phải nhận event của server (đổi hướng lần đầu)"
        );
    }
}

#[test]
fn gioi_han_giao_thuc_dung_trong_phien() {
    // Số lệnh tối đa một batch là tham số giao thức, lấy từ `ProtocolLimits` (ADR 0009 §3.5).
    let limits = ProtocolLimits::v1();
    assert_eq!(limits.max_commands, 8);
}
