//! Test end-to-end headless: client mạng + `boom-server` qua `MemoryTransport`.
//!
//! Kịch bản: hai client handshake vào server, đăng nhập thật, vào phòng, đánh bom đi lại.
//! Server là authority (ADR 0005); client chỉ dự đoán di chuyển của chính mình và reconcile khi
//! snapshot về. Server dùng `Server::with_auth` + SQLite tempfile (ADR 0011 §10) nên client phải
//! đi đúng bước `Welcome` → `Auth` → `AuthOk` trước khi gửi input (ADR 0009 Addendum A §A.1).
//!
//! Lỗi mạng mô phỏng theo seed (`boom-net`): trễ, mất, trùng, đảo thứ tự. Cùng seed và cùng lệnh
//! gửi cho cùng thứ tự giao trên mọi OS.
//!
//! Ngoài ra có fixture [`fixture`] với **lịch giao theo tick của server**: `MemoryTransport` có
//! đồng hồ ảo riêng nên độ trễ không chứng minh được "input tới trễ nhiều tick server"; fixture
//! mới gắn đồng hồ mạng với thời gian server để đo được trễ thật, mất, trùng, đảo thứ tự và
//! hội tụ của dự đoán.

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr};
use std::thread;
use std::time::Duration;

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, Dir, PlayerInput, Rules, Stat, World, mode_by_id};
use boom_net::{MemoryConfig, MemoryFaults, MemoryTransport, Timing, Transport};
use boom_proto::dto::auth::AuthFailReason;
use boom_proto::{ProtocolLimits, RejectReason, RoundEvent, Welcome, decode_frame, encode_frame};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand};
use boom_server::auth::{AuthConfig, AuthService};
use boom_server::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use boom_server::{
    Authority, CharacterStatsTable, HandshakeConfig, MatchSetup, Server, ServerConfig,
};

use super::fixture::{ACCOUNT_PASSWORD, LATE_THRESHOLD_MS, ScheduledFaults, ScheduledHarness};
use super::{ClientSession, Credentials, NetClient, NetError, hello_for_profile};

/// Profile rules cố định cho test (khác `data/rules/v1-dev.toml` nhưng cùng schema).
const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

/// Thời gian chờ message khi bơm hàng đợi đọc.
const PUMP_TIMEOUT: Duration = Duration::from_millis(20);

/// Số bước mô phỏng trong một kịch bản.
const STEPS: u64 = 30;

/// IP nguồn giả của kết nối trong test (bucket rate limit theo nguồn, ADR 0011 §5).
const TEST_SOURCE_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

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
    /// Thư mục tạm chứa DB auth: khai sau `server` nên được xoá **sau** khi store đóng.
    _db_dir: tempfile::TempDir,
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
                // Tên map cho client dựng bản đồ; test không ghi replay.
                map: "skeleton-test".to_owned(),
                replay: None,
            },
        );
        let handshake = HandshakeConfig {
            rules_profile_id: profile.profile_id.clone(),
            rules_hash: rules.profile_hash,
            content_sha256: [0; 32],
            content_epoch: 1,
            snapshot_interval_ms: 50,
        };
        // Auth thật: SQLite trên file tạm + `AuthService` của server (ADR 0011 §5–§6, §10).
        // Server chặn mọi kind trước `AuthOk` nên client bắt buộc phải đăng nhập.
        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("auth.sqlite3");
        let store = SqliteAccountStore::open(&db_path, SqliteOptions::default()).expect("mở store");
        let auth = AuthService::new(
            AuthConfig::default(),
            "test-instance",
            Box::new(OsRandomSource),
        )
        .expect("dịch vụ auth");
        let mut server = Server::with_auth(authority, handshake, auth, Box::new(store), 0)
            .expect("dựng server có auth");
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
                .accept(player, slot, TEST_SOURCE_IP, server_reader, server_writer)
                .expect("accept");
            // Handshake + auth cần server poll (vòng IO của server), nên client chờ ở luồng riêng
            // còn server poll tới khi xong.
            let client_rules = rules.clone();
            let client_hello = hello.clone();
            let credentials =
                Credentials::register(format!("test-{index}"), ACCOUNT_PASSWORD.to_owned());
            let handle = thread::spawn(move || {
                let mut net = NetClient::new(
                    ClientSession::new(client_reader, client_writer),
                    client_rules,
                    mode_by_id("battle_ffa").expect("mode"),
                );
                net.handshake(&client_hello).expect("handshake");
                net.authenticate(&credentials).expect("auth");
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
            _db_dir: db_dir,
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
            client.net.resend().expect("gửi cửa sổ lệnh");
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
            // Tên map cho client dựng bản đồ; test không ghi replay.
            map: "skeleton-test".to_owned(),
            replay: None,
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
        .accept(PlayerId(1), 0, TEST_SOURCE_IP, server_reader, server_writer)
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

// --- Auth trên dây (ADR 0009 Addendum A §A.1) ---

#[test]
fn client_dang_nhap_that_va_nhan_auth_ok() {
    // Server thật (`Server::with_auth` + SQLite tempfile): client đi đúng thứ tự
    // `Welcome` → `Auth` → `AuthOk` và nhận token phiên.
    let harness = Harness::new(MemoryFaults::none(), MemoryFaults::none(), 20261021);
    for (index, client) in harness.clients.iter().enumerate() {
        let account = client.net.account().expect("phải đăng nhập được");
        assert_eq!(
            account.display_name,
            format!("test-{index}"),
            "tên hiển thị phải đúng tên vừa đăng ký"
        );
        assert!(
            !account.token.is_empty(),
            "server phải cấp token phiên (ADR 0009 Addendum A §A.2)"
        );
        assert!(
            client
                .net
                .server_version()
                .is_some_and(|version| !version.is_empty()),
            "`Welcome` phải mang build id của server (ADR 0011 §8.3)"
        );
    }
}

#[test]
fn client_dang_nhap_sai_bao_auth_fail() {
    // Mật khẩu sai ⇒ server trả `AuthFail` (không lộ enumeration, ADR 0011 §5) và **không**
    // đóng phiên: client được gửi lại `Auth` trên cùng kết nối (ADR 0009 Addendum A §A.1).
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
            map: "skeleton-test".to_owned(),
            replay: None,
        },
    );
    let handshake = HandshakeConfig {
        rules_profile_id: profile.profile_id.clone(),
        rules_hash: rules.profile_hash,
        content_sha256: [0; 32],
        content_epoch: 1,
        snapshot_interval_ms: 50,
    };
    let db_dir = tempfile::tempdir().expect("tempdir");
    let store = SqliteAccountStore::open(
        &db_dir.path().join("auth.sqlite3"),
        SqliteOptions::default(),
    )
    .expect("mở store");
    let auth = AuthService::new(
        AuthConfig::default(),
        "test-instance",
        Box::new(OsRandomSource),
    )
    .expect("dịch vụ auth");
    let mut server =
        Server::with_auth(authority, handshake, auth, Box::new(store), 0).expect("dựng server");
    let (client_writer, server_reader) = MemoryTransport::pair(MemoryConfig::default());
    let (server_writer, client_reader) = MemoryTransport::pair(MemoryConfig::default());
    server
        .accept(PlayerId(1), 0, TEST_SOURCE_IP, server_reader, server_writer)
        .expect("accept");

    let client_rules = rules.clone();
    let client_hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
    let handle = thread::spawn(move || {
        let mut net = NetClient::new(
            ClientSession::new(client_reader, client_writer),
            client_rules,
            mode_by_id("battle_ffa").expect("mode"),
        );
        net.handshake(&client_hello).expect("handshake");
        net.authenticate(&Credentials::login("khong-ton-tai", "matkhau sai"))
    });
    while !handle.is_finished() {
        server.poll(0, Duration::from_millis(1)).expect("poll");
    }
    match handle.join().expect("join luồng client") {
        Err(NetError::AuthFailed(reason)) => {
            assert_eq!(reason, AuthFailReason::InvalidCredentials);
        }
        other => panic!("phải báo AuthFail, thấy {other:?}"),
    }
}

#[test]
fn tai_khoan_fixture_dang_ky_that_bang_sqlite() {
    // Fixture đăng ký tài khoản thật (Argon2id + SQLite tempfile) nên tài khoản đã có phiên
    // sống trong DB: đăng nhập lại phải bị từ chối `AccountHasActiveSession` (ADR 0011 §6).
    // Nếu store là giả (trong RAM) thì đăng nhập lại sẽ thành công ⇒ test bắt được.
    let mut harness = ScheduledHarness::new(ScheduledFaults::none(), ScheduledFaults::none(), 5);
    let error = harness
        .login_again(0)
        .expect_err("tài khoản đã có phiên thì không được đăng nhập lại");
    assert_eq!(
        error.reason,
        boom_server::auth::AuthFailure::AccountHasActiveSession
    );
}

// --- Client đơn (không qua server): áp snapshot thẳng để kiểm xử lý round và kết quả. ---

/// Client mạng độc lập: test đẩy message trực tiếp và đọc frame client gửi ra.
struct SoloClient {
    net: NetClient<MemoryTransport>,
    /// Endpoint phía server của chiều server→client (đẩy message cho client).
    server_to_client: MemoryTransport,
    /// Endpoint phía server của chiều client→server (đọc frame client gửi).
    client_to_server: MemoryTransport,
}

impl SoloClient {
    /// Dựng client và hoàn tất handshake (server giả trả `Welcome`).
    fn new() -> Self {
        let profile = RulesProfile::from_toml(RULES).expect("profile rules");
        let rules = Rules::from_profile(&profile).expect("rules");
        let (mut server_to_client, client_reader) = MemoryTransport::pair(MemoryConfig::default());
        let (client_writer, mut client_to_server) = MemoryTransport::pair(MemoryConfig::default());
        let mut net = NetClient::new(
            ClientSession::new(client_reader, client_writer),
            rules.clone(),
            mode_by_id("battle_ffa").expect("mode"),
        );
        let welcome = Welcome {
            session_id: "solo".into(),
            player_id: 0,
            wire_schema: boom_proto::limits::WIRE_SCHEMA,
            state_schema: 1,
            server_version: "boom-server-test".to_owned(),
            rules_profile_id: profile.profile_id.clone(),
            rules_hash: rules.profile_hash,
            content_sha256: [0; 32],
            content_epoch: 1,
            snapshot_interval_ms: 50,
            protocol_limits_version: ProtocolLimits::v1().revision,
        };
        server_to_client
            .send_frame(&encode_frame(&welcome, &ProtocolLimits::v1()).expect("encode welcome"))
            .expect("gửi welcome");
        net.handshake(&hello_for_profile(
            &profile,
            &rules,
            [0; 32],
            "boom-client-test",
        ))
        .expect("handshake");
        // Đọc `Hello` client đã gửi để hàng đợi chiều client→server còn trống cho batch.
        let _hello_frame = client_to_server.recv_frame().expect("đọc hello");
        Self {
            net,
            server_to_client,
            client_to_server,
        }
    }

    /// Đẩy một snapshot cho client.
    fn push_snapshot(&mut self, snapshot: &boom_proto::Snapshot) {
        self.server_to_client
            .send_frame(&encode_frame(snapshot, &ProtocolLimits::v1()).expect("encode snapshot"))
            .expect("gửi snapshot");
        self.net.pump(PUMP_TIMEOUT).expect("bơm message");
    }

    /// Đọc batch input client gửi (nếu có).
    fn take_batch(&mut self) -> Option<boom_proto::InputBatch> {
        let frame = self.client_to_server.try_recv_frame().expect("đọc frame")?;
        Some(decode_frame(&frame, &ProtocolLimits::v1()).expect("decode input batch"))
    }

    /// Đẩy một `InputRejected` cho client.
    fn push_rejected(&mut self, rejected: &boom_proto::InputRejected) {
        self.server_to_client
            .send_frame(
                &encode_frame(rejected, &ProtocolLimits::v1()).expect("encode input rejected"),
            )
            .expect("gửi input rejected");
        self.net.pump(PUMP_TIMEOUT).expect("bơm message");
    }
}

/// Dựng state của server (hai người chơi) để đóng gói snapshot.
fn solo_state(seed: u64) -> boom_core::SimulationState {
    let profile = RulesProfile::from_toml(RULES).expect("profile rules");
    let rules = Rules::from_profile(&profile).expect("rules");
    let world = World::new(
        super::fixture::terrain(),
        rules,
        mode_by_id("battle_ffa").expect("mode"),
    )
    .expect("world");
    boom_core::SimulationState::new_round(
        &world,
        &[
            boom_core::PlayerSetup {
                id: 0,
                stats: stats(),
            },
            boom_core::PlayerSetup {
                id: 1,
                stats: stats(),
            },
        ],
        seed,
    )
    .expect("round")
}

/// Đóng gói snapshot từ state của server.
fn solo_snapshot(
    state: &boom_core::SimulationState,
    round_id: u32,
    seq: u32,
) -> boom_proto::Snapshot {
    boom_sync::to_snapshot(
        state,
        &boom_sync::SnapshotContext {
            snapshot_seq: seq,
            round_id,
            state_schema: 1,
            rules_hash: 0,
            content_epoch: 1,
            acks: Vec::new(),
        },
    )
    .expect("snapshot")
}

#[test]
fn snapshot_round_cu_khong_lam_lui_metadata_client() {
    let mut client = SoloClient::new();
    let state = solo_state(7);
    // Round 2, snapshot_seq 5: client áp và ghi nhớ.
    client.push_snapshot(&solo_snapshot(&state, 2, 5));
    client
        .net
        .predict(PlayerInput {
            dir: Dir::Right,
            place_bomb: false,
        })
        .expect("dự đoán");
    client.net.resend().expect("gửi cửa sổ lệnh");
    let batch = client.take_batch().expect("phải có batch input");
    assert_eq!(batch.round_id, 2);
    assert_eq!(batch.ack_snapshot, 5);

    // Packet của round cũ tới trễ (seq lớn hơn nhưng round cũ): client bỏ qua, không rollback.
    client.push_snapshot(&solo_snapshot(&state, 1, 9));
    // Snapshot cũ hơn của round hiện tại: cũng bỏ qua.
    client.push_snapshot(&solo_snapshot(&state, 2, 4));
    client
        .net
        .predict(PlayerInput {
            dir: Dir::Down,
            place_bomb: false,
        })
        .expect("dự đoán");
    client.net.resend().expect("gửi cửa sổ lệnh");
    let batch = client.take_batch().expect("phải có batch input");
    assert_eq!(
        batch.round_id, 2,
        "packet round cũ không được đổi round của client"
    );
    assert_eq!(
        batch.ack_snapshot, 5,
        "metadata snapshot chỉ cập nhật khi snapshot được áp"
    );
}

#[test]
fn input_rejected_cua_round_cu_khong_xoa_lenh_round_moi() {
    let mut client = SoloClient::new();
    let state = solo_state(7);
    // Round 2: client có một lệnh chưa ack.
    client.push_snapshot(&solo_snapshot(&state, 2, 5));
    client
        .net
        .predict(PlayerInput {
            dir: Dir::Right,
            place_bomb: false,
        })
        .expect("dự đoán");
    assert_eq!(client.net.pending_len(), 1);

    // Reject của round cũ: không được xóa lệnh của round mới.
    client.push_rejected(&boom_proto::InputRejected {
        round_id: 1,
        reason: boom_proto::InputRejectReason::BadSequence,
        applied_seq: 1,
        first_seq: 1,
        command_count: 1,
        server_tick: 3,
    });
    assert_eq!(
        client.net.pending_len(),
        1,
        "reject của round cũ không được retire lệnh của round mới"
    );

    // Reject của round hiện tại: được áp dụng như ack.
    client.push_rejected(&boom_proto::InputRejected {
        round_id: 2,
        reason: boom_proto::InputRejectReason::BadSequence,
        applied_seq: 1,
        first_seq: 1,
        command_count: 1,
        server_tick: 3,
    });
    assert_eq!(
        client.net.pending_len(),
        0,
        "reject của round hiện tại phải retire lệnh đã giải quyết"
    );
}

#[test]
fn client_khong_du_doan_va_khong_gui_khi_snapshot_bao_ket_qua() {
    let mut client = SoloClient::new();
    let mut state = solo_state(7);
    // Trước khi kết thúc: client dự đoán và gửi bình thường.
    client.push_snapshot(&solo_snapshot(&state, 1, 1));
    client
        .net
        .predict(PlayerInput {
            dir: Dir::Right,
            place_bomb: false,
        })
        .expect("dự đoán");
    client.net.resend().expect("gửi cửa sổ lệnh");
    assert!(
        client.take_batch().is_some(),
        "phải gửi lệnh khi lượt còn chơi"
    );

    // Server báo kết quả lượt.
    state.phase = boom_core::Phase::Ended(boom_core::Outcome::Winner(0));
    client.push_snapshot(&solo_snapshot(&state, 1, 2));
    assert!(
        matches!(client.net.phase(), Some(boom_core::Phase::Ended(_))),
        "client phải biết lượt đã kết thúc"
    );
    assert_eq!(
        client.net.pending_len(),
        0,
        "lệnh chưa ack của lượt đã kết thúc không còn ý nghĩa"
    );

    // Tick tiếp sau kết quả: client không lỗi, không sinh lệnh và không gửi gì.
    client
        .net
        .predict(PlayerInput {
            dir: Dir::Right,
            place_bomb: false,
        })
        .expect("dự đoán sau kết quả không được lỗi");
    client.net.resend().expect("gửi cửa sổ lệnh");
    assert!(
        client.take_batch().is_none(),
        "client không được gửi lệnh sau khi lượt kết thúc"
    );
    // Client vẫn nhận message (bơm không lỗi) và giữ kết quả của server.
    client.net.pump(PUMP_TIMEOUT).expect("vẫn nhận message");
    let confirmed = client.net.confirmed_state().expect("có state confirmed");
    assert_eq!(
        confirmed.phase,
        boom_core::Phase::Ended(boom_core::Outcome::Winner(0))
    );
}

// --- Fixture có lịch giao theo tick của server (finding 5). ---

/// Số bước trong kịch bản lỗi mạng.
const SCHEDULED_STEPS: u32 = 40;

/// Chạy kịch bản hai client đổi hướng và nhấn bom trên fixture có lịch giao theo tick server.
fn chay_scheduled(
    out_faults: ScheduledFaults,
    in_faults: ScheduledFaults,
    seed: u64,
) -> ScheduledHarness {
    let mut harness = ScheduledHarness::new(out_faults, in_faults, seed);
    harness.start_round();
    for step in 0..SCHEDULED_STEPS {
        // Đổi hướng và nhấn bom theo chu kỳ: input thay đổi liên tục, không phải đi thẳng một
        // hướng, để dự đoán thật sự bị lệch khi mạng trễ/mất.
        let dirs = if step % 8 < 4 {
            [Dir::Right, Dir::Down]
        } else {
            [Dir::Left, Dir::Up]
        };
        let bombs = [step % 5 == 0, step % 7 == 0];
        harness.tick(dirs, bombs);
    }
    harness
}

#[test]
fn du_doan_hoi_tu_server_khi_mat_tre_va_dao_thu_tu() {
    // Chiều gửi input: trễ 50 ms (3 tick ở 60 Hz) + jitter + mất + đảo thứ tự — trong ngân sách
    // `INPUT_LATE` của giao thức nhưng đủ để dự đoán bị lệch. Chiều nhận: trùng + trễ nhỏ +
    // đảo thứ tự (snapshot cũ tới sau snapshot mới).
    let out_faults = ScheduledFaults {
        loss_permille: 50,
        reorder_permille: 200,
        delay_ms: 50,
        jitter_ms: 30,
        reorder_window_ms: 20,
        ..ScheduledFaults::none()
    };
    let in_faults = ScheduledFaults {
        duplicate_permille: 50,
        reorder_permille: 100,
        delay_ms: 20,
        jitter_ms: 30,
        reorder_window_ms: 10,
        ..ScheduledFaults::none()
    };
    let mut harness = chay_scheduled(out_faults, in_faults, 20261020);

    // 1. Lỗi mạng thực sự xảy ra (không rỗng): mất, trễ, đảo thứ tự.
    let stats = harness.net_stats();
    assert!(stats.lost > 0, "phải có frame bị mất: {stats:?}");
    assert!(stats.delayed > 0, "phải có frame bị trễ: {stats:?}");
    assert!(
        stats.max_late_ms >= LATE_THRESHOLD_MS,
        "input phải tới trễ nhiều tick server: {stats:?}"
    );
    assert!(
        stats.reordered > 0,
        "phải có frame tới đảo thứ tự: {stats:?}"
    );

    // 2. Dự đoán thực sự bị lệch (vì mạng trễ/mất) và phải được correction sửa lại.
    assert!(
        harness.mismatches() > 0,
        "phải có lần vị trí dự đoán lệch vị trí server"
    );
    assert!(
        harness.corrections() > 0,
        "phải có lần client phải tính lại dự đoán khi snapshot về"
    );

    // 3. Event server không được phát lặp (dedup theo `(round_id, EventId)`).
    for index in 0..2 {
        let mut seen: Vec<(u32, u16)> = Vec::new();
        for event in harness.events(index) {
            let key = (event.id.tick, event.id.seq);
            assert!(!seen.contains(&key), "event bị phát hai lần: {key:?}");
            seen.push(key);
        }
    }

    // 3. Drain tới mốc đồng bộ rồi so vị trí dự đoán với server ở cùng tick.
    harness.drain();
    for index in 0..2 {
        let client = harness.client(index);
        let confirmed = client.net.confirmed_state().expect("có state confirmed");
        let predicted = client.net.state().expect("có state dự đoán");
        let expected = harness
            .server_hash(confirmed.tick)
            .expect("server đã chạy qua tick này");
        assert_eq!(
            confirmed.hash(),
            expected,
            "state confirmed phải khớp state server tại cùng tick"
        );
        let server_position = harness
            .server_position(client.player, confirmed.tick)
            .expect("có vị trí server");
        let player = predicted.player(client.slot).expect("có người chơi");
        assert_eq!(
            (player.x.0, player.y.0),
            server_position,
            "sau correction, vị trí dự đoán phải khớp vị trí server tại cùng tick"
        );
        // Người chơi đã thật sự di chuyển (không đứng yên ở ô spawn).
        let start = harness
            .start_position(client.player)
            .expect("có vị trí đầu lượt");
        assert_ne!(
            (player.x.0, player.y.0),
            start,
            "kịch bản phải có chuyển động thật"
        );
    }
}

#[test]
fn duplicate_input_thuc_su_den_server_va_khong_ap_hai_lan() {
    // Chiều gửi input bị nhân bản: cùng một lệnh tới server nhiều lần (client resend cửa sổ
    // chưa ack là chủ đích, ADR 0009 §3.2). Server phải dedup theo sequence.
    let faults = ScheduledFaults {
        duplicate_permille: 500,
        ..ScheduledFaults::none()
    };
    let mut duplicated = chay_scheduled(faults, ScheduledFaults::none(), 4242);
    let mut reference = chay_scheduled(ScheduledFaults::none(), ScheduledFaults::none(), 4242);

    // Lệnh trùng thực sự tới server.
    assert!(
        duplicated.duplicate_commands() > 0,
        "phải có lệnh trùng sequence tới server: {} batch",
        duplicated.input_batches()
    );

    // Dùng lệnh trùng, state server phải y hệt kịch bản không trùng: lệnh không được áp hai
    // lần.
    duplicated.drain();
    reference.drain();
    let tick = duplicated
        .client(0)
        .net
        .confirmed_state()
        .expect("có state confirmed")
        .tick;
    assert_eq!(
        duplicated.server_hash(tick),
        reference.server_hash(tick),
        "lệnh trùng không được làm state server khác kịch bản không trùng"
    );
    for index in 0..2 {
        let client = duplicated.client(index);
        let confirmed = client.net.confirmed_state().expect("có state confirmed");
        let expected = duplicated
            .server_hash(confirmed.tick)
            .expect("có hash của tick");
        assert_eq!(
            confirmed.hash(),
            expected,
            "state confirmed phải khớp state server tại cùng tick"
        );
        assert_eq!(
            client.net.pending_len(),
            0,
            "mọi lệnh phải được server ack sau khi drain"
        );
    }
}

// --- Luồng phòng end-to-end với server thật (ADR 0009 Addendum B) ---

use super::fixture::{ROOM_MAP_ID, RoomHarness};
use boom_proto::{RoomAction as RoomActionDto, RoomCharacter, RoomRejectReason};

/// Số bước bơm tối đa khi chờ một mốc trạng thái phòng.
const ROOM_WAIT_STEPS: u32 = 600;

/// Bật luồng tự động cho từng client **theo thứ tự** và chờ mỗi client vào được phòng.
///
/// Bật tuần tự (không bật cả lô) để thứ tự vào phòng khớp thứ tự kết nối: server cấp slot
/// phòng theo thứ tự vào, mà slot phòng cũng là id người chơi trong state trận.
fn join_all(harness: &mut RoomHarness, count: usize) {
    for index in 0..count {
        harness.enable_auto(index, count);
        assert!(
            harness.wait_for(index, |client| client.net.room().room().is_some(), ROOM_WAIT_STEPS),
            "client {index} phải vào được phòng"
        );
    }
    // Mọi client sẵn sàng: phòng chỉ start khi đủ người **và** mọi người ready
    // (SCOPE_V1 §3). Luồng tự động bật ready ngay khi vào phòng.
    assert!(
        harness.wait_for_all(
            |client| client.net.room().my_member().is_some_and(|member| member.ready),
            ROOM_WAIT_STEPS
        ),
        "mọi client phải sẵn sàng"
    );
    for index in 0..count {
        assert!(harness.is_ready(index), "client {index} phải sẵn sàng");
    }
}

/// Chơi thật trong lượt: client gửi input (dự đoán + gửi cửa sổ lệnh chưa ack).
fn play(harness: &mut RoomHarness, count: usize, steps: u32) {
    for step in 0..steps {
        for index in 0..count {
            let dir = if step % 8 < 4 { Dir::Right } else { Dir::Down };
            let client = harness.client_mut(index);
            client
                .net
                .predict(PlayerInput {
                    dir,
                    place_bomb: false,
                })
                .expect("dự đoán");
            client.net.resend().expect("gửi cửa sổ lệnh");
        }
        harness.pump(1);
    }
}

/// Nhảy tới hết thời gian lượt để server tự quyết kết quả.
///
/// Thời gian lượt lấy từ rules profile (`round.duration_ms`), không hard-code trong test.
fn run_out_round(harness: &mut RoomHarness) {
    let duration_ms = harness.round_duration_ms();
    let now = harness.now_ms();
    harness.advance_to(now + duration_ms + 10_000);
    harness.pump(40);
}

/// Chạy trọn luồng V1: vào phòng → ready → start → `RoundStart` → chơi → `RoundResult` → ack →
/// về phòng → trận 2.
fn chay_luong_phong(count: usize, faults: ScheduledFaults, seed: u64) -> RoomHarness {
    let mut harness = RoomHarness::new(count, faults, faults, seed);
    join_all(&mut harness, count);

    // Mọi người ready → host start → `RoundStart` tới mọi client.
    assert!(
        harness.wait_for_all(|client| client.net.room().round().is_some(), ROOM_WAIT_STEPS),
        "mọi client phải nhận RoundStart"
    );
    let round_id = harness
        .client(0)
        .net
        .room()
        .round()
        .expect("có kế hoạch round")
        .round_id;
    // Mọi client cùng nhận một round: cùng ID, cùng seed, cùng map/mode.
    for index in 0..count {
        let plan = harness
            .client(index)
            .net
            .room()
            .round()
            .expect("có kế hoạch round")
            .clone();
        assert_eq!(plan.round_id, round_id, "mọi client phải cùng round");
        assert_eq!(plan.map, ROOM_MAP_ID);
        assert_eq!(plan.mode, "battle_ffa");
        assert_eq!(plan.roster.len(), count, "roster phải đủ người chơi");
        assert!(
            harness.client(index).net.room().loading_acked() == Some(round_id),
            "client {index} phải ack loading"
        );
    }
    let seed = harness
        .client(0)
        .net
        .room()
        .round()
        .expect("có kế hoạch round")
        .seed;
    for index in 1..count {
        assert_eq!(
            harness
                .client(index)
                .net
                .room()
                .round()
                .expect("có kế hoạch round")
                .seed,
            seed,
            "mọi client phải cùng seed"
        );
    }

    // Chơi thật trong lượt rồi để server quyết kết quả.
    play(&mut harness, count, 20);
    run_out_round(&mut harness);

    // Event của round đến client và không bị phát lại (dedup theo `(round_id, EventId)`).
    for index in 0..count {
        let events = harness.client(index).events();
        assert!(
            !events.is_empty(),
            "client {index} phải nhận event của round (đổi hướng lần đầu)"
        );
        let mut seen: Vec<(u32, u16)> = Vec::new();
        for event in events {
            let key = (event.id.tick, event.id.seq);
            assert!(!seen.contains(&key), "event bị phát hai lần: {key:?}");
            seen.push(key);
        }
    }

    // Kết quả do server quyết: mọi client cùng nhận và cùng ack.
    for index in 0..count {
        let result = harness
            .client(index)
            .net
            .room()
            .result()
            .expect("phải nhận kết quả round")
            .clone();
        assert_eq!(result.round_id, round_id, "kết quả phải của round đang chơi");
        assert_eq!(
            result.results.len(),
            count,
            "kết quả phải có đủ người chơi"
        );
        assert_eq!(
            harness.client(index).net.room().result_acked(),
            Some(round_id),
            "client {index} phải ack kết quả"
        );
    }

    // Về phòng và vào được trận 2.
    let ok = harness.wait_for_all(
        |client| {
            client
                .net
                .room()
                .round()
                .is_some_and(|plan| plan.round_id > round_id)
        },
        ROOM_WAIT_STEPS,
    );
    if !ok {
        for index in 0..count {
            let client = harness.client(index);
            eprintln!(
                "DEBUG client {index}: room={:?} ready={:?} round={:?} result={:?} reject={:?} err={:?}",
                client.net.room().room().map(|r| (r.phase, r.members.len())),
                client.net.room().my_member().map(|m| m.ready),
                client.net.room().round().map(|p| p.round_id),
                client.net.room().result().map(|r| r.round_id),
                client.net.room().rejection().map(|r| r.reason),
                client.net.room_error(),
            );
        }
    }
    assert!(ok, "mọi client phải vào được trận 2");
    harness
}

#[test]
fn hai_client_trong_luong_phong_day_du() {
    let harness = chay_luong_phong(2, ScheduledFaults::none(), 20261020);
    assert_eq!(harness.client_count(), 2);
    assert!(
        harness.client(0).net.room().is_host(),
        "client tạo phòng phải là host"
    );
    assert!(!harness.client(1).net.room().is_host());
}

#[test]
fn bon_client_trong_luong_phong_day_du() {
    let harness = chay_luong_phong(4, ScheduledFaults::none(), 20261021);
    assert_eq!(harness.client_count(), 4);
    for index in 0..4 {
        let client = harness.client(index);
        let room = client.net.room().room().expect("client phải ở trong phòng");
        assert_eq!(room.members.len(), 4, "phòng phải đủ 4 người");
        // Slot phòng là id người chơi trong state trận: thành viên của client phải khớp
        // slot/player mà server cấp.
        let player = client.player.0;
        assert!(
            room.members
                .iter()
                .any(|member| member.slot == client.slot && member.player == player),
            "client {index} phải có mặt trong phòng với slot/player của mình"
        );
        let plan = client.net.room().round().expect("có kế hoạch round");
        assert!(
            plan.roster
                .iter()
                .any(|member| member.slot == client.slot && member.player == player),
            "roster của round phải chứa client {index}"
        );
    }
}

#[test]
fn luong_phong_chay_duoc_khi_mang_tre_mat_trung() {
    // Mạng có trễ, mất, trùng và đảo thứ tự: luồng phải vẫn chạy trọn vẹn nhờ client gửi lại
    // lệnh phòng (cùng request ID → server dedup).
    let faults = ScheduledFaults {
        loss_permille: 50,
        duplicate_permille: 50,
        reorder_permille: 100,
        delay_ms: 20,
        jitter_ms: 30,
        reorder_window_ms: 10,
    };
    let harness = chay_luong_phong(2, faults, 20261022);
    assert!(
        harness.client(0).net.room().result().is_some(),
        "phải nhận kết quả dù mạng mất frame"
    );
}

#[test]
fn request_lap_khong_ap_hai_lan() {
    let mut harness = RoomHarness::new(2, ScheduledFaults::none(), ScheduledFaults::none(), 4242);
    // Client 0 tạo phòng rồi gửi lại **cùng** request ID: server dedup nên không tạo phòng
    // thứ hai và không báo từ chối.
    let request = harness
        .client_mut(0)
        .net
        .room_command(RoomActionDto::Create {
            character: RoomCharacter::C00,
        })
        .expect("tạo phòng");
    harness.pump(20);
    harness
        .client_mut(0)
        .net
        .resend_room_command(
            request,
            RoomActionDto::Create {
                character: RoomCharacter::C00,
            },
        )
        .expect("gửi lại lệnh");
    harness.pump(20);

    let room = harness
        .client(0)
        .net
        .room()
        .room()
        .expect("phải có phòng")
        .clone();
    assert_eq!(room.members.len(), 1, "lệnh lặp không được tạo phòng thứ hai");
    assert!(
        harness.client(0).net.room().rejection().is_none(),
        "lệnh lặp cùng request ID không được bị từ chối"
    );

    // Cùng hành động nhưng request ID **mới** thì server từ chối (đã có phòng): chứng minh
    // lệnh lặp ở trên thực sự được dedup, không phải vì lệnh bị bỏ qua.
    harness
        .client_mut(0)
        .net
        .room_command(RoomActionDto::Create {
            character: RoomCharacter::C00,
        })
        .expect("tạo phòng lần hai");
    harness.pump(20);
    let rejection = *harness
        .client(0)
        .net
        .room()
        .rejection()
        .expect("lệnh mới phải bị từ chối");
    assert_eq!(rejection.reason, RoomRejectReason::AlreadyStarted);
    assert_eq!(
        harness
            .client(0)
            .net
            .room()
            .room()
            .expect("phải có phòng")
            .members
            .len(),
        1,
        "phòng vẫn chỉ có một người"
    );
}

#[test]
fn chat_chi_toi_thanh_vien_phong() {
    let mut harness = RoomHarness::new(3, ScheduledFaults::none(), ScheduledFaults::none(), 777);
    // Hai client vào phòng; client thứ ba đứng ở sảnh (không vào phòng nào).
    harness.enable_auto(0, 2);
    assert!(
        harness.wait_for(0, |client| client.net.room().room().is_some(), ROOM_WAIT_STEPS),
        "client 0 phải vào được phòng"
    );
    harness.enable_auto(1, 2);
    assert!(
        harness.wait_for(1, |client| client.net.room().room().is_some(), ROOM_WAIT_STEPS),
        "client 1 phải vào được phòng"
    );
    assert!(
        !harness.in_room(2),
        "client 2 không được vào phòng khi chưa bật luồng tự động"
    );

    // Client ngoài phòng gửi chat: server từ chối, không có tin nào phát ra.
    harness
        .client_mut(2)
        .net
        .send_chat("ngoi phong")
        .expect("gửi chat");
    harness.pump(20);
    assert!(
        harness.client(2).net.room().chat().is_empty(),
        "chat của người ngoài phòng không được phát ra"
    );
    assert!(
        harness.client(2).net.room().rejection().is_some(),
        "server phải từ chối chat của người không ở trong phòng"
    );

    // Chat của thành viên trong phòng tới mọi thành viên trong phòng.
    harness
        .client_mut(0)
        .net
        .send_chat("xin chao")
        .expect("gửi chat");
    harness.pump(20);
    for index in 0..2 {
        let chat = harness.client(index).net.room().chat();
        assert!(
            chat.iter().any(|message| message.text == "xin chao"),
            "client {index} phải nhận chat của phòng"
        );
    }
    assert!(
        harness.client(2).net.room().chat().is_empty(),
        "chat của phòng không được tới client ngoài phòng"
    );
}
