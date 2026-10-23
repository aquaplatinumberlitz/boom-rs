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
use std::time::{Duration, Instant};

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{
    CharacterStats, CharacterStatsTable, Dir, PlayerInput, Rules, Stat, World, mode_by_id,
};
use boom_net::{MemoryConfig, MemoryCounters, MemoryFaults, MemoryTransport, Timing, Transport};
use boom_proto::dto::auth::AuthFailReason;
use boom_proto::{
    MessageKind, ProtocolLimits, RejectReason, RoomAction as RoomActionDto, RoomCharacter,
    RoundEvent, RoundMember, RoundStart, Welcome, decode_frame, encode_frame, read_envelope,
};
use boom_room::{PlayerId, RoomView};
use boom_server::auth::{AuthConfig, AuthService};
use boom_server::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use boom_server::{Authority, HandshakeConfig, MatchSetup, Server, ServerConfig, ServerPhase};

use super::fixture::{
    ACCOUNT_PASSWORD, EnqueuedReader, FlushWriter, LATE_THRESHOLD_MS, ReaderProgress,
    ScheduledFaults, ScheduledHarness, SharedWriter,
};
use super::{ClientMessage, ClientSession, Credentials, NetClient, NetError, hello_for_profile};

/// Profile rules cố định cho test (khác `data/rules/v1-dev.toml` nhưng cùng schema).
const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

/// Thời gian chờ message khi bơm hàng đợi đọc.
const PUMP_TIMEOUT: Duration = Duration::from_millis(20);

/// Số vòng bơm tối đa cho luồng phòng (server xử lý lệnh → client nhận → client ack).
const ROOM_PUMP_ROUNDS: u32 = 32;

/// Trần số vòng đồng bộ client với server (mỗi vòng xin snapshot mới rồi chờ).
///
/// Điều kiện dừng là "state confirmed của mọi client khớp tick server" — không phải hằng số
/// thời gian ⇒ máy chậm vẫn xanh.
const SYNC_ROUNDS: u32 = 2_000;

/// Số bước mô phỏng trong một kịch bản.
const STEPS: u64 = 30;

/// IP nguồn giả của kết nối trong test (bucket rate limit theo nguồn, ADR 0011 §5).
const TEST_SOURCE_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Gốc định danh **kết nối** trong kịch bản này.
///
/// Cố tình khác `account_id` (mà SQLite cấp từ 1) và khác slot trận (0, 1, …): định danh
/// kết nối **không** phải slot trận (ADR 0009 Addendum B §B.2), nên test phải bắt được lỗi
/// "lấy slot từ `Welcome`".
const CONNECTION_ID_BASE: u64 = 900;

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
    net: NetClient<SharedWriter<MemoryTransport>>,
    /// Định danh người chơi trong phòng/round (= `AuthOk.account_id`).
    ///
    /// Server bind mọi trường `player` trên dây từ giá trị này (ADR 0009 Addendum B §B.2).
    player: PlayerId,
    /// Định danh **kết nối** trong phiên (khoá session phía server).
    ///
    /// **Không** phải slot trận: slot lấy từ roster theo `player`.
    connection: PlayerId,
    /// Slot trận của client (lấy từ `RoundStart.roster` theo `player`).
    ///
    /// `None` = client chưa nhận `RoundStart` nên chưa biết slot của mình.
    slot: Option<u8>,
    /// Event đã nhận (đã dedup theo `(round_id, EventId)`).
    events: Vec<RoundEvent>,
    /// Bộ đếm chiều **client → server**: biết luồng đọc của server đã lấy frame chưa.
    out: MemoryCounters,
    /// Bộ đếm chiều **server → client**: biết luồng đọc của client đã lấy frame chưa.
    inbound: MemoryCounters,
    inbound_progress: ReaderProgress,
    out_progress: ReaderProgress,
    writer_progress: ReaderProgress,
    marker_writer: SharedWriter<MemoryTransport>,
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
        for index in 0..2 {
            // Định danh **kết nối** trong phiên: **không** phải slot trận.
            let connection = PlayerId(u64::try_from(index).expect("id") + CONNECTION_ID_BASE);
            // `Welcome.player_id` = định danh kết nối: fixture cố tình chọn giá trị **khác**
            // slot trận (server bind slot từ roster theo `account_id`) để chứng minh client
            // không lấy slot từ `Welcome` (ADR 0009 Addendum B §B.2).
            let connection_id = u8::try_from(index).expect("id") + 1;
            // Hai chiều tách riêng: client→server và server→client (giống cách server chia).
            let (client_writer, server_reader) = MemoryTransport::pair(MemoryConfig {
                seed: seed.wrapping_add(index as u64),
                faults: out_faults,
                timing: Timing::Virtual,
            });
            // Bộ đếm của chiều client→server: test biết chắc server đã lấy frame input
            // (không đoán theo hạn thời gian thật) — xem `Harness::wait_input_delivered`.
            let out = client_writer.counters();
            let client_writer = SharedWriter::new(client_writer);
            let marker_writer = client_writer.clone();
            let (server_writer, client_reader) = MemoryTransport::pair(MemoryConfig {
                seed: seed.wrapping_add(index as u64),
                faults: in_faults,
                timing: Timing::Virtual,
            });
            // Bộ đếm của chiều server→client: test biết chắc client đã lấy frame server gửi
            // (không đoán theo hạn thời gian thật) — xem `Harness::pump_clients`.
            let in_counters = client_reader.counters();
            let (client_reader, inbound_progress) = EnqueuedReader::new(client_reader);
            let (server_reader, out_progress) = EnqueuedReader::new(server_reader);
            let (server_writer, writer_progress) = FlushWriter::new(server_writer);
            server
                .accept(
                    connection,
                    connection_id,
                    TEST_SOURCE_IP,
                    server_reader,
                    server_writer,
                )
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
            // Định danh người chơi trong phòng/round = `account_id` (server bind từ phiên).
            let account_id = net.account().expect("client đã đăng nhập").account_id;
            clients.push(Client {
                net,
                player: PlayerId(account_id),
                connection,
                slot: None,
                events: Vec::new(),
                out,
                inbound: in_counters,
                inbound_progress,
                out_progress,
                writer_progress,
                marker_writer,
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
    ///
    /// Mọi lệnh phòng đi qua **wire**: client gửi `RoomCommand`, server bind `player` từ phiên
    /// (`AuthOk.account_id`) rồi gửi `RoomState`/`RoundStart`; client lấy slot trận của mình từ
    /// mục `RoundStart.roster` có `player == account_id` (ADR 0009 Addendum B §B.2).
    ///
    /// Lệnh bị mất/đảo thứ tự trên đường thì **gửi lại cùng request ID**: server dedup theo
    /// request ID nên lệnh không áp hai lần. Điều kiện chờ đo trên **server** (authority là
    /// nguồn sự thật), không đo trên bản `RoomState` của client: mạng có thể đảo thứ tự
    /// `RoomState` nên bản client có thể cũ hơn bản server.
    fn start_round(&mut self) {
        let create = RoomActionDto::Create {
            character: RoomCharacter::C00,
        };
        let request = self.clients[0]
            .net
            .room_command(create.clone())
            .expect("gửi lệnh tạo phòng");
        assert!(
            self.room_command_until(0, request, create, |view| view.members.len() == 1),
            "server phải tạo được phòng"
        );
        let room = self.server.room_view().expect("có phòng").id.0;
        let join = RoomActionDto::Join {
            room,
            character: RoomCharacter::C01,
        };
        let request = self.clients[1]
            .net
            .room_command(join.clone())
            .expect("gửi lệnh vào phòng");
        assert!(
            self.room_command_until(1, request, join, |view| view.members.len() == 2),
            "server phải cho client 1 vào phòng"
        );
        let mut ready_commands = Vec::new();
        for index in 0..2 {
            let ready = RoomActionDto::SetReady { ready: true };
            let request = self.clients[index]
                .net
                .room_command(ready.clone())
                .expect("gửi lệnh ready");
            ready_commands.push((index, request, ready));
        }
        assert!(
            self.room_commands_until(&ready_commands, |view| {
                view.members.iter().all(|member| member.ready)
            }),
            "server phải thấy mọi người ready"
        );
        let request = self.clients[0]
            .net
            .room_command(RoomActionDto::Start)
            .expect("gửi lệnh start");
        // `RoundStart` mang roster: client lấy slot trận của mình từ đó. Lệnh `Start` bị mất
        // thì gửi lại cùng request ID: server coi là trùng và gửi lại `RoundStart`.
        assert!(
            self.wait_for_round_slots(0, request),
            "mọi client phải nhận RoundStart và biết slot trận của mình"
        );
        for client in &mut self.clients {
            client.slot = Some(
                client
                    .net
                    .player_slot()
                    .expect("client phải biết slot trận của mình"),
            );
        }
        // Client phải có gốc dự đoán **khớp tick** của server trước khi chơi.
        self.sync_with_server();
        self.origin_ms = 0;
        // Vị trí lúc đầu: người chơi đứng ở tâm ô spawn.
        for client in &self.clients {
            if let Some(position) = self.server.authority().player_position(client.player) {
                self.start_positions.insert(client.player, position);
            }
        }
    }

    /// Đồng bộ state confirmed của client với state hiện tại của server.
    ///
    /// Server chạy tick **đầu tiên** ngay khi loading barrier qua, còn client nhận được
    /// snapshot gửi lúc phòng còn ở loading (tick 0) — snapshot đó **đã cũ hơn** state
    /// server. Nếu client dùng nó làm gốc dự đoán thì lệnh client gán `target_tick` sẽ
    /// trễ một tick so với server: client điều khiển nhân vật lệch một tick suốt vòng
    /// chơi.
    ///
    /// Client xin snapshot mới (`SnapshotRequest`, ADR 0009 §4.3) để lấy state hiện tại
    /// của server: gốc dự đoán khớp đúng tick server đang ở, nên lệnh đầu tiên (target
    /// tick = tick gốc) được server áp đúng ở transition kế tiếp.
    ///
    /// Đồng hồ của harness cũng phải khớp lịch tick của server: server tính tick từ lúc
    /// loading barrier qua (`tick_origin_ms`), nên `ticks` của harness bắt đầu bằng số
    /// tick server đã chạy.
    fn sync_with_server(&mut self) {
        for _ in 0..SYNC_ROUNDS {
            let playing = self.server.authority().phase() == ServerPhase::Playing;
            let tick = self.server.authority().tick();
            let synced = playing
                && tick.is_some_and(|tick| {
                    self.clients.iter().all(|client| {
                        client
                            .net
                            .confirmed_state()
                            .is_some_and(|state| state.tick == tick)
                    })
                });
            if synced {
                self.ticks = u64::from(tick.expect("đã kiểm ở trên"));
                return;
            }
            for client in &mut self.clients {
                client.net.request_snapshot().expect("xin snapshot");
            }
            self.pump_once();
        }
        panic!("client không đồng bộ được state với server");
    }

    /// Gửi lệnh phòng của client `index` và bơm cho tới khi server **xác nhận** điều kiện.
    ///
    /// Lệnh bị mất trên đường thì gửi lại **cùng request ID**: server dedup theo request ID nên
    /// lệnh không áp hai lần (ADR 0009 Addendum B §B.2).
    fn room_command_until(
        &mut self,
        index: usize,
        request: u64,
        action: RoomActionDto,
        done: impl Fn(&RoomView) -> bool,
    ) -> bool {
        let commands = [(index, request, action)];
        self.room_commands_until(&commands, done)
    }

    /// Bơm cho tới khi server **xác nhận** điều kiện, gửi lại các lệnh phòng đã gửi.
    ///
    /// Lệnh bị mất trên đường thì gửi lại **cùng request ID**: server dedup theo request ID nên
    /// lệnh không áp hai lần (ADR 0009 Addendum B §B.2).
    fn room_commands_until(
        &mut self,
        commands: &[(usize, u64, RoomActionDto)],
        done: impl Fn(&RoomView) -> bool,
    ) -> bool {
        for _ in 0..ROOM_PUMP_ROUNDS {
            if self.server.room_view().is_some_and(|view| done(&view)) {
                return true;
            }
            for (index, request, action) in commands {
                self.clients[*index]
                    .net
                    .resend_room_command(*request, action.clone())
                    .expect("gửi lệnh phòng");
            }
            self.pump_once();
        }
        self.server.room_view().is_some_and(|view| done(&view))
    }

    /// Bơm cho tới khi **mọi** client biết slot trận của mình (đã nhận `RoundStart`).
    ///
    /// `RoundStart` mang roster: client lấy slot trận của mình từ mục có `player ==
    /// account_id` (ADR 0009 Addendum B §B.2). `RoundStart` bị mất/đảo thứ tự trên đường thì
    /// gửi lại lệnh `Start` **cùng request ID**: server coi là trùng và gửi lại `RoundStart`.
    fn wait_for_round_slots(&mut self, index: usize, request: u64) -> bool {
        for _ in 0..ROOM_PUMP_ROUNDS {
            if self
                .clients
                .iter()
                .all(|client| client.net.player_slot().is_some())
            {
                return true;
            }
            self.clients[index]
                .net
                .resend_room_command(request, RoomActionDto::Start)
                .expect("gửi lại lệnh start");
            self.pump_once();
        }
        self.clients
            .iter()
            .all(|client| client.net.player_slot().is_some())
    }

    /// Ghi lại hash/điểm của state server tại tick hiện tại.
    ///
    /// Gọi sau **mọi** lần poll server (kể cả lúc dựng phòng): client giữ state confirmed
    /// theo snapshot đã nhận, nên hash của mọi tick server đã chạy đều phải có trong bản đồ
    /// để đối chiếu. Lúc dựng phòng, server đã gửi snapshot đầu tiên (tick 0) **trước** khi
    /// lượt chạy tick, nên hash của tick đó phải được ghi lại ngay — bỏ qua thì client giữ
    /// state ở tick cũ mà bản đồ không có tick đó.
    fn record_state(&mut self) {
        let Some(tick) = self.server.authority().tick() else {
            return;
        };
        let hash = self.server.authority().state_hash().expect("state hash");
        self.hashes.insert(tick, hash);
        for client in &self.clients {
            if let Some(position) = self.server.authority().player_position(client.player) {
                self.server_positions
                    .insert((client.player, tick), position);
            }
        }
    }

    /// Một vòng bơm: server xử lý lệnh trước, rồi client nhận `RoomState`/`RoundStart` và
    /// gửi `LoadingAck`.
    ///
    /// Đồng hồ server giữ nguyên 0 nên lượt chưa chạy tick trong lúc dựng phòng.
    fn pump_once(&mut self) {
        self.poll_server(0);
        self.record_state();
        self.pump_clients();
    }

    /// Chờ server **xử lý hết** frame client đã gửi (đồng hồ server đứng yên ở `now`).
    ///
    /// Chờ ReaderProgress đạt sent_frames: mỗi frame còn sống, kể cả bản trùng, đã enqueue.
    ///
    /// Rào reader xác nhận enqueue và marker Ping/Pong rào writer, clock giữ tại now.
    fn poll_server(&mut self, now: u64) {
        for client in &self.clients {
            client.out_progress.wait_for(client.out.sent_frames());
        }
        self.server.poll(now, Duration::ZERO).expect("poll server");
        for index in 0..self.clients.len() {
            let client = &mut self.clients[index];
            let mut completed = false;
            for _ in 0..SYNC_ROUNDS {
                let before = client.out.sent_frames();
                let marker = client.writer_progress.count();
                client
                    .marker_writer
                    .send_frame(
                        &encode_frame(&boom_proto::Ping(u64::MAX), &ProtocolLimits::v1())
                            .expect("ping"),
                    )
                    .expect("marker");
                let sent = client.out.sent_frames();
                client.out_progress.wait_for(sent);
                self.server.poll(now, Duration::ZERO).expect("poll marker");
                if sent > before {
                    client.writer_progress.wait_for(marker + sent - before);
                    completed = true;
                    break;
                }
            }
            assert!(completed, "mọi marker đã bị mất");
        }
    }

    /// Bơm hàng đợi đọc của mọi client tới khi **mọi frame server gửi** đã được xử lý.
    ///
    /// Điều kiện dựa trên **bộ đếm frame** của `MemoryTransport` (không đo thời gian thật).
    /// Chờ enqueue đủ rồi drain một lần, không gửi retry theo số vòng chờ thread.
    fn pump_clients(&mut self) {
        for client in &mut self.clients {
            client
                .inbound_progress
                .wait_for(client.inbound.peer_sent_frames());
            client.net.pump(Duration::ZERO).expect("bơm message");
            client.events.extend(client.net.take_events());
        }
    }

    /// Một bước: client nhận snapshot và reconcile, dự đoán rồi gửi, server chạy tick.
    fn tick(&mut self, dirs: [Dir; 2]) {
        self.pump_clients();
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
            // Nhận input **trước** khi server chạy tick:
            // 1. Chờ luồng đọc của server **lấy đủ** frame client đã gửi (đồng hồ server đứng
            //    yên ở `before` ⇒ không tick nào chạy, không lệch tick nào).
            // 2. Poll **có chờ** ⇒ chắc chắn batch input đã vào kênh vào và được server xử lý
            //    trước khi tick kế tiếp chạy ⇒ lệnh áp dụng đúng tick client đã gán
            //    `target_tick`, không phải tick sau (lệch một tick làm vị trí dự đoán lệch).
            let before = now - 1;
            self.poll_server(before);
        }
        self.poll_server(now);
        self.record_state();
        for client in &self.clients {
            if let Some(state) = client.net.state() {
                let tick = state.tick;
                let slot = client.slot.expect("client phải biết slot trận của mình");
                if let Some(player) = state.player(slot) {
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
            let expected = self.hashes.get(&confirmed.tick).unwrap_or_else(|| {
                panic!(
                    "server đã chạy qua tick này: confirmed tick={} hashes={:?}",
                    confirmed.tick,
                    self.hashes.keys().collect::<Vec<_>>()
                )
            });
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
fn slot_tran_lay_tu_roster_theo_account_id_khong_phai_welcome_player_id() {
    // `Welcome.player_id` là **định danh kết nối** trong phiên, **không** phải slot trận:
    // slot trận là `slot` của mục `RoundStart.roster` có `player == AuthOk.account_id`
    // (ADR 0009 Addendum B §B.2). Fixture cố tình chọn `Welcome.player_id` khác slot trận
    // để test này bắt được lỗi "lấy slot từ `Welcome`".
    let mut harness = Harness::new(MemoryFaults::none(), MemoryFaults::none(), 20261023);
    harness.start_round();
    for client in &harness.clients {
        let connection = client.net.connection_id().expect("có định danh kết nối");
        let slot = client.slot.expect("client phải biết slot trận của mình");
        assert_ne!(
            connection, slot,
            "định danh kết nối (`Welcome.player_id`) phải khác slot trận"
        );
        // Mục roster của client là mục có `player == account_id`; slot của mục đó mới là
        // slot trận của client.
        let plan = client.net.round_plan().expect("có kế hoạch round");
        let member = plan
            .roster
            .iter()
            .find(|member| member.player == client.player.0)
            .expect("roster phải chứa client theo account_id");
        assert_eq!(
            member.slot, slot,
            "slot trận phải lấy từ roster theo `account_id`"
        );
        // Định danh kết nối phía server cũng khác định danh người chơi trong phòng.
        assert_ne!(
            client.connection, client.player,
            "định danh kết nối phải khác `account_id`"
        );
    }
    // Client điều khiển **đúng nhân vật** của mình: sau khi đi, vị trí nhân vật ở slot của
    // client phải khớp vị trí server (nếu client lấy nhầm slot của người khác thì lệch).
    for _ in 0..STEPS {
        harness.tick([Dir::Right, Dir::Down]);
    }
    for client in &harness.clients {
        let slot = client.slot.expect("client phải biết slot trận của mình");
        let predicted = client.net.state().expect("có state dự đoán");
        let player = predicted
            .player(slot)
            .expect("client phải điều khiển được nhân vật ở slot của mình");
        let server_position = harness
            .server
            .authority()
            .player_position(client.player)
            .expect("có vị trí ở server");
        assert_eq!(
            (player.x.0, player.y.0),
            server_position,
            "client phải điều khiển đúng nhân vật của mình (slot từ roster)"
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
        let slot = client.slot.expect("client phải biết slot trận của mình");
        let player = predicted.player(slot).expect("có người chơi");
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
fn giai_ma_khoi_du_lieu_server_dong_goi() {
    // Server đóng gói event bằng `boom_sync::to_round_events` (tên loại và tên trường lấy
    // từ `EventKind::name()`/`EventKind::fields()` của core); client giải mã bằng
    // `decode_event`. Hai bên phải dùng **cùng một bảng tên**: mọi loại event V1 đi qua
    // vòng đóng gói → giải mã phải ra đúng event core.
    use boom_core::{
        Cell, EntityId, EventId as CoreEventId, EventKind, ItemKind, Outcome, SimulationEvent,
    };
    let events = vec![
        SimulationEvent {
            id: CoreEventId { tick: 1, seq: 0 },
            kind: EventKind::PlayerTurned { player: 2 },
        },
        SimulationEvent {
            id: CoreEventId { tick: 1, seq: 1 },
            kind: EventKind::BombPlaced {
                bomb: EntityId(7),
                player: 1,
                cell: Cell::new(3, 4),
                range: 5,
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 2, seq: 0 },
            kind: EventKind::BombExploded {
                bomb: EntityId(7),
                owner: 1,
                cell: Cell::new(3, 4),
                chained: true,
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 2, seq: 1 },
            kind: EventKind::BlockDestroyed {
                cell: Cell::new(5, 6),
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 2, seq: 2 },
            kind: EventKind::ItemRevealed {
                item: EntityId(9),
                kind: ItemKind(2),
                cell: Cell::new(5, 6),
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 2, seq: 3 },
            kind: EventKind::ItemDestroyed {
                item: EntityId(9),
                cell: Cell::new(5, 6),
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 3, seq: 0 },
            kind: EventKind::ItemPicked {
                item: EntityId(9),
                kind: ItemKind(2),
                player: 0,
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 4, seq: 0 },
            kind: EventKind::PlayerTrapped { player: 1, by: 0 },
        },
        SimulationEvent {
            id: CoreEventId { tick: 5, seq: 0 },
            kind: EventKind::PlayerDied {
                player: 1,
                by: Some(0),
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 5, seq: 1 },
            kind: EventKind::PlayerDied {
                player: 0,
                by: None,
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 6, seq: 0 },
            kind: EventKind::RoundEnded {
                outcome: Outcome::Winner(1),
            },
        },
        SimulationEvent {
            id: CoreEventId { tick: 6, seq: 1 },
            kind: EventKind::RoundEnded {
                outcome: Outcome::Draw,
            },
        },
    ];
    let wire = boom_sync::to_round_events(11, &events);
    assert_eq!(wire.events.len(), events.len());
    for (original, encoded) in events.iter().zip(wire.events.iter()) {
        let decoded = super::decode_event(encoded).expect("phải giải mã được");
        assert_eq!(&decoded, original, "giải mã phải ra đúng event core");
    }
}

#[test]
fn giai_ma_bo_event_khong_hieu_va_thieu_truong() {
    // Event tên lạ hoặc thiếu trường thì **bỏ**: consumer không đoán giá trị
    // (`EventKind` là `non_exhaustive`, ADR 0009 §2).
    let id = boom_proto::EventId { tick: 1, seq: 0 };
    assert!(
        super::decode_event(&RoundEvent {
            id,
            name: "event_la".to_owned(),
            fields: Vec::new(),
        })
        .is_none(),
        "tên loại lạ phải bị bỏ"
    );
    assert!(
        super::decode_event(&RoundEvent {
            id,
            name: "player_died".to_owned(),
            fields: Vec::new(),
        })
        .is_none(),
        "thiếu trường phải bị bỏ"
    );
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

/// Định danh người chơi trong phòng/round của client đơn (= `AuthOk.account_id`).
const SOLO_ACCOUNT_ID: u64 = 1;

/// Slot trận của client đơn trong roster của `RoundStart`.
const SOLO_SLOT: u8 = 0;

/// Định danh **kết nối** trong phiên (`Welcome.player_id`).
///
/// Cố tình khác [`SOLO_SLOT`]: định danh kết nối **không** phải slot trận
/// (ADR 0009 Addendum B §B.2), nên test bắt được lỗi "lấy slot từ `Welcome`".
const SOLO_CONNECTION_ID: u8 = 7;

/// Client mạng độc lập: test đẩy message trực tiếp và đọc frame client gửi ra.
struct SoloClient {
    net: NetClient<MemoryTransport>,
    /// Endpoint phía server của chiều server→client (đẩy message cho client).
    server_to_client: MemoryTransport,
    /// Endpoint phía server của chiều client→server (đọc frame client gửi).
    client_to_server: MemoryTransport,
    inbound_progress: ReaderProgress,
}

impl SoloClient {
    /// Dựng client và hoàn tất handshake (server giả trả `Welcome`) rồi đăng nhập.
    ///
    /// Client phải đăng nhập trước khi vào phòng: `AuthOk.account_id` là **định danh người
    /// chơi trong phòng/round** mà server ghi vào mọi trường `player` trên dây
    /// (ADR 0009 Addendum B §B.2).
    fn new() -> Self {
        Self::with_retry_interval(super::ROOM_RETRY_INTERVAL)
    }

    /// Dựng client với **nhịp gửi lại** tuỳ chọn.
    ///
    /// Nhịp gửi lại là tham số vận hành (design): lệnh phòng và ack phòng có **hai đồng hồ
    /// riêng** nhưng cùng một khoảng cách tối thiểu. Test cần nhịp **khác 0** để bắt được
    /// lỗi "hai nhịp dùng chung đồng hồ" (nhịp 0 ⇒ hai nhịp luôn trùng nhau nên lỗi bị che).
    fn with_retry_interval(interval: Duration) -> Self {
        let profile = RulesProfile::from_toml(RULES).expect("profile rules");
        let rules = Rules::from_profile(&profile).expect("rules");
        let (mut server_to_client, client_reader) = MemoryTransport::pair(MemoryConfig::default());
        let (client_writer, mut client_to_server) = MemoryTransport::pair(MemoryConfig::default());
        let (client_reader, inbound_progress) = EnqueuedReader::new(client_reader);
        let mut net = NetClient::with_room_retry_interval(
            ClientSession::new(client_reader, client_writer),
            rules.clone(),
            mode_by_id("battle_ffa").expect("mode"),
            interval,
        );
        let welcome = Welcome {
            session_id: "solo".into(),
            player_id: SOLO_CONNECTION_ID,
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
        let limits = ProtocolLimits::v1();
        server_to_client
            .send_frame(&encode_frame(&welcome, &limits).expect("encode welcome"))
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
        // Đăng nhập: server giả trả `AuthOk` với `account_id` cố định.
        let auth_ok = boom_proto::dto::auth::AuthOk {
            account_id: SOLO_ACCOUNT_ID,
            display_name: "solo".into(),
            token: "solo-token".into(),
            expires_at_ms: u64::MAX,
        };
        server_to_client
            .send_frame(&encode_frame(&auth_ok, &limits).expect("encode auth ok"))
            .expect("gửi auth ok");
        net.authenticate(&Credentials::login("solo", "matkhau123"))
            .expect("đăng nhập");
        let _auth_frame = client_to_server.recv_frame().expect("đọc auth");
        Self {
            net,
            server_to_client,
            client_to_server,
            inbound_progress,
        }
    }

    /// `RoundStart` mà mục roster của client (`player == account_id`) có slot `slot`.
    fn round_start_of(round_id: u64, slot: u8) -> RoundStart {
        let profile = RulesProfile::from_toml(RULES).expect("profile rules");
        let rules = Rules::from_profile(&profile).expect("rules");
        RoundStart {
            round_id,
            seed: 7,
            rules_profile_id: profile.profile_id.clone(),
            rules_hash: rules.profile_hash,
            content_sha256: [0; 32],
            map: "skeleton-test".to_owned(),
            mode: "battle_ffa".to_owned(),
            roster: vec![RoundMember {
                player: SOLO_ACCOUNT_ID,
                slot,
                character: RoomCharacter::C00,
                name: "a".into(),
            }],
            start_tick: 0,
        }
    }

    /// Đẩy `RoundStart` cho client (không đọc frame client gửi: `RoundStart` cũ thì client
    /// **không** gửi gì).
    fn push_round_start_raw(&mut self, start: &RoundStart) {
        self.server_to_client
            .send_frame(&encode_frame(start, &ProtocolLimits::v1()).expect("encode round start"))
            .expect("gửi round start");
        self.settle();
    }

    /// Đẩy `RoundStart` cho client: roster mang slot trận của client theo `account_id`.
    ///
    /// Client lấy slot trận của mình từ mục roster có `player == account_id`; định danh
    /// kết nối (`Welcome.player_id`) không được dùng làm slot.
    fn push_round_start(&mut self, round_id: u64) {
        self.push_round_start_raw(&Self::round_start_of(round_id, SOLO_SLOT));
        // Client gửi `LoadingAck` sau khi nhận `RoundStart`: đọc để hàng đợi chiều
        // client→server còn trống cho batch input.
        let _ack_frame = self.client_to_server.recv_frame().expect("đọc loading ack");
    }

    /// Đẩy `RoomState` (phòng chờ, client chưa ready) cho client.
    ///
    /// Client bật luồng tự động thì coi đây là phòng đã vào: lệnh phòng `SetReady` được gửi
    /// và **đang chờ** xác nhận (server không trả `RoomState` nào nữa) ⇒ nhịp gửi lại lệnh
    /// phòng chạy liên tục.
    fn push_room_state(&mut self) {
        let state = boom_proto::RoomState {
            room_id: 1,
            host: SOLO_ACCOUNT_ID,
            phase: boom_proto::RoomPhase::Waiting,
            members: vec![boom_proto::RoomMember {
                player: SOLO_ACCOUNT_ID,
                slot: 0,
                character: RoomCharacter::C00,
                ready: false,
                name: "a".into(),
            }],
            round: None,
        };
        self.server_to_client
            .send_frame(&encode_frame(&state, &ProtocolLimits::v1()).expect("encode room state"))
            .expect("gửi room state");
        self.settle();
    }

    /// Đẩy `RoundResult` cho client (kết quả do server quyết).
    fn push_round_result(&mut self, round_id: u64) {
        let result = boom_proto::RoundResult {
            round_id,
            outcome: boom_proto::RoomOutcome::Winner(SOLO_ACCOUNT_ID),
            results: vec![boom_proto::PlayerResult {
                player: SOLO_ACCOUNT_ID,
                slot: SOLO_SLOT,
                rank: 1,
                alive: true,
            }],
        };
        self.server_to_client
            .send_frame(&encode_frame(&result, &ProtocolLimits::v1()).expect("encode round result"))
            .expect("gửi round result");
        self.settle();
    }

    /// Đẩy một snapshot cho client.
    fn push_snapshot(&mut self, snapshot: &boom_proto::Snapshot) {
        self.server_to_client
            .send_frame(&encode_frame(snapshot, &ProtocolLimits::v1()).expect("encode snapshot"))
            .expect("gửi snapshot");
        self.settle();
    }

    /// Đọc batch input client gửi (nếu có), bỏ qua frame khác (ack, lệnh phòng).
    fn take_batch(&mut self) -> Option<boom_proto::InputBatch> {
        while let Some(frame) = self.client_to_server.try_recv_frame().expect("đọc frame") {
            let (envelope, _) = read_envelope(&frame).expect("đọc envelope");
            if envelope.kind != MessageKind::InputBatch {
                continue;
            }
            return Some(decode_frame(&frame, &ProtocolLimits::v1()).expect("decode input batch"));
        }
        None
    }

    /// Kind của các frame client đã gửi (đọc hết hàng đợi chiều client→server).
    fn drain_kinds(&mut self) -> Vec<MessageKind> {
        let mut kinds = Vec::new();
        while let Some(frame) = self.client_to_server.try_recv_frame().expect("đọc frame") {
            let (envelope, _) = read_envelope(&frame).expect("đọc envelope");
            kinds.push(envelope.kind);
        }
        kinds
    }

    /// Đẩy một `InputRejected` cho client.
    fn push_rejected(&mut self, rejected: &boom_proto::InputRejected) {
        self.server_to_client
            .send_frame(
                &encode_frame(rejected, &ProtocolLimits::v1()).expect("encode input rejected"),
            )
            .expect("gửi input rejected");
        self.settle();
    }

    /// Chờ client **xử lý** message server vừa đẩy.
    ///
    /// ReaderProgress xác nhận frame đã enqueue; pump(ZERO) xử lý hết queue sau rào.
    fn settle(&mut self) {
        self.inbound_progress
            .wait_for(self.server_to_client.counters().sent_frames());
        self.net.pump(Duration::ZERO).expect("bơm message");
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
    // Round 2: client nhận `RoundStart` (biết slot trận của mình từ roster) rồi mới áp
    // snapshot của round đó.
    client.push_round_start(2);
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
    // Round 2: client nhận `RoundStart` trước khi áp snapshot của round đó.
    client.push_round_start(2);
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
    // Round 1: client nhận `RoundStart` (biết slot trận của mình từ roster) rồi mới áp
    // snapshot của round đó.
    client.push_round_start(1);
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
        let slot = client.slot;
        let player = predicted.player(slot).expect("có người chơi");
        assert_eq!(
            (player.x.0, player.y.0),
            server_position,
            "sau correction, vị trí dự đoán phải khớp vị trí server tại cùng tick"
        );
        // Người chơi đã thật sự di chuyển (không đứng yên ở ô spawn).
        //
        // So **mọi** tick của lượt chứ không chỉ tick cuối: kịch bản đổi hướng liên tục
        // (sang phải rồi trái, xuống rồi lên) nên vị trí ở tick cuối có thể trùng đúng vị trí
        // đầu lượt dù suốt lượt người chơi đã đi.
        let start = harness
            .start_position(client.player)
            .expect("có vị trí đầu lượt");
        let moved = (0..=confirmed.tick).any(|tick| {
            harness
                .server_position(client.player, tick)
                .is_some_and(|position| position != start)
        });
        assert!(
            moved,
            "kịch bản phải có chuyển động thật (vị trí đầu lượt {start:?})"
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
use boom_proto::RoomRejectReason;

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
            harness.wait_for(
                index,
                |client| client.net.room().room().is_some(),
                ROOM_WAIT_STEPS
            ),
            "client {index} phải vào được phòng"
        );
    }
    // Mọi client sẵn sàng: phòng chỉ start khi đủ người **và** mọi người ready
    // (SCOPE_V1 §3). Luồng tự động bật ready ngay khi vào phòng.
    assert!(
        harness.wait_for_all(
            |client| client
                .net
                .room()
                .my_member()
                .is_some_and(|member| member.ready),
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
fn chay_luong_phong(
    count: usize,
    out_faults: ScheduledFaults,
    in_faults: ScheduledFaults,
    seed: u64,
) -> RoomHarness {
    let mut harness = RoomHarness::new(count, out_faults, in_faults, seed);
    join_all(&mut harness, count);

    // Mọi người ready → host start → `RoundStart` tới mọi client.
    assert!(
        harness.wait_for_all(
            |client| client.net.room().round().is_some(),
            ROOM_WAIT_STEPS
        ),
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
    assert!(
        harness.server_tick().is_some_and(|tick| tick > 0),
        "server phải chạy lượt thật (tick authority tiến)"
    );
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
        assert_eq!(
            result.round_id, round_id,
            "kết quả phải của round đang chơi"
        );
        assert_eq!(result.results.len(), count, "kết quả phải có đủ người chơi");
        assert_eq!(
            harness.client(index).net.room().result_acked(),
            Some(round_id),
            "client {index} phải ack kết quả"
        );
    }

    // Về phòng và vào được trận 2.
    assert!(
        harness.wait_for_all(
            |client| {
                client
                    .net
                    .room()
                    .round()
                    .is_some_and(|plan| plan.round_id > round_id)
            },
            ROOM_WAIT_STEPS
        ),
        "mọi client phải vào được trận 2"
    );
    harness
}

#[test]
fn hai_client_trong_luong_phong_day_du() {
    let harness = chay_luong_phong(
        2,
        ScheduledFaults::none(),
        ScheduledFaults::none(),
        20261020,
    );
    assert_eq!(harness.client_count(), 2);
    assert!(
        harness.client(0).net.room().is_host(),
        "client tạo phòng phải là host"
    );
    assert!(!harness.client(1).net.room().is_host());
}

#[test]
fn bon_client_trong_luong_phong_day_du() {
    let harness = chay_luong_phong(
        4,
        ScheduledFaults::none(),
        ScheduledFaults::none(),
        20261021,
    );
    assert_eq!(harness.client_count(), 4);
    for index in 0..4 {
        let client = harness.client(index);
        let room = client.net.room().room().expect("client phải ở trong phòng");
        assert_eq!(room.members.len(), 4, "phòng phải đủ 4 người");
        // Định danh người chơi trong phòng = `AuthOk.account_id`; slot trận lấy từ
        // `RoundStart.roster` theo định danh đó (ADR 0009 Addendum B §B.2).
        let player = client.player.0;
        let slot = client.slot().expect("client phải biết slot trận của mình");
        assert!(
            room.members
                .iter()
                .any(|member| member.slot == slot && member.player == player),
            "client {index} phải có mặt trong phòng với slot/player của mình"
        );
        let plan = client.net.room().round().expect("có kế hoạch round");
        assert!(
            plan.roster
                .iter()
                .any(|member| member.slot == slot && member.player == player),
            "roster của round phải chứa client {index}"
        );
        // `Welcome.player_id` là định danh **kết nối**, khác slot trận: client vẫn phải điều
        // khiển đúng nhân vật của mình (slot lấy từ roster).
        let connection = client
            .connection_id()
            .expect("client phải biết định danh kết nối");
        assert_ne!(
            connection, slot,
            "định danh kết nối phải khác slot trận (client {index})"
        );
        // Định danh kết nối phía server cũng khác định danh người chơi trong phòng.
        assert_ne!(
            client.connection, client.player,
            "định danh kết nối phải khác `account_id` (client {index})"
        );
    }
}

#[test]
fn luong_phong_chay_duoc_khi_mang_tre_mat_trung() {
    // Mạng có trễ, mất, trùng và đảo thứ tự: luồng phải vẫn chạy trọn vẹn nhờ client gửi lại
    // lệnh phòng (cùng request ID → server dedup).
    //
    let out_faults = ScheduledFaults {
        loss_permille: 50,
        duplicate_permille: 50,
        reorder_permille: 100,
        delay_ms: 20,
        jitter_ms: 30,
        reorder_window_ms: 10,
    };
    let in_faults = ScheduledFaults {
        loss_permille: 50,
        duplicate_permille: 50,
        reorder_permille: 100,
        delay_ms: 20,
        jitter_ms: 30,
        reorder_window_ms: 10,
    };
    // Lịch seed này mất frame cả hai chiều nhưng giữ các thông báo RoundStart/RoundResult
    // một lần; regression bên dưới ép mất RoundResult rồi kiểm recovery.
    let harness = chay_luong_phong(2, out_faults, in_faults, 20261023);
    // Lỗi mạng phải **thật sự xảy ra**: nếu không mất frame nào thì test không chứng minh
    // được gì.
    for index in 0..harness.client_count() {
        let inbound = harness.client(index).inbound_stats();
        assert!(
            inbound.lost > 0,
            "chiều server→client {index} phải mất frame: {inbound:?}"
        );
    }
    let stats = harness.net_stats();
    assert!(stats.lost > 0, "phải có frame bị mất: {stats:?}");
    assert!(stats.delayed > 0, "phải có frame bị trễ: {stats:?}");
    assert!(
        stats.reordered > 0,
        "phải có frame tới đảo thứ tự: {stats:?}"
    );
    assert!(
        harness.client(0).net.room().result().is_some(),
        "phải nhận kết quả dù mạng mất frame"
    );
    assert!(
        harness.client(1).net.room().result().is_some(),
        "mọi client phải nhận kết quả dù mạng mất frame"
    );
}

/// Mất frame kết quả đầu: NetClient xin lại bảng đã commit từ server thật.
#[test]
fn mat_round_result_xin_lai_dung_bang_authority() {
    let mut harness = RoomHarness::new(
        2,
        ScheduledFaults::none(),
        ScheduledFaults::none(),
        20261023,
    );
    join_all(&mut harness, 2);
    assert!(harness.wait_for_all(
        |client| client.net.room().round().is_some(),
        ROOM_WAIT_STEPS
    ));
    play(&mut harness, 2, 20);
    harness.client(0).drop_round_results();
    run_out_round(&mut harness);
    assert_eq!(
        harness.client(0).inbound_stats().lost,
        1,
        "ép mất đúng RoundResult"
    );
    assert!(
        harness.client(1).net.room().result().is_some(),
        "peer nhận kết quả server thật"
    );
    assert!(harness.wait_for(0, |client| client.net.room().result().is_some(), 200));
    assert_eq!(
        harness.client(0).net.room().result(),
        harness.client(1).net.room().result()
    );
    assert!(harness.client(0).net.room().result_acked().is_some());
    assert_eq!(
        harness.client(0).inbound_stats().lost,
        1,
        "server chưa gửi lại RoundResult"
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
    assert_eq!(
        room.members.len(),
        1,
        "lệnh lặp không được tạo phòng thứ hai"
    );
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
        harness.wait_for(
            0,
            |client| client.net.room().room().is_some(),
            ROOM_WAIT_STEPS
        ),
        "client 0 phải vào được phòng"
    );
    harness.enable_auto(1, 2);
    assert!(
        harness.wait_for(
            1,
            |client| client.net.room().room().is_some(),
            ROOM_WAIT_STEPS
        ),
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

// --- RoundStart cũ/trùng (review PR #136 P1) và nhịp gửi lại ack (review PR #136 P2) ---

#[test]
fn round_start_cu_toi_tre_khong_lam_lui_round_moi() {
    // `RoundStart` của round **cũ** tới trễ **sau** khi client đã á snapshot của round mới:
    // client phải bỏ nó trước mọi side effect — không lùi kế hoạch round, không ghi lùi
    // slot trận (roster của round cũ đặt slot khác), không ack loading cho round cũ.
    let mut client = SoloClient::new();
    let mut state = solo_state(7);
    state.tick = 5;
    // Round 2: client nhận `RoundStart` (slot 0 theo roster) rồi á snapshot round 2.
    client.push_round_start(2);
    client.push_snapshot(&solo_snapshot(&state, 2, 5));
    assert_eq!(client.net.player_slot(), Some(SOLO_SLOT));

    // `RoundStart` của round 1 tới trễ, roster đặt slot 7 cho client.
    client.push_round_start_raw(&SoloClient::round_start_of(1, 7));
    assert!(
        client
            .client_to_server
            .try_recv_frame()
            .expect("đọc frame")
            .is_none(),
        "`RoundStart` cũ không được ack loading"
    );
    assert_eq!(
        client.net.player_slot(),
        Some(SOLO_SLOT),
        "`RoundStart` cũ không được ghi lùi slot trận"
    );
    assert_eq!(
        client.net.round_plan().map(|plan| plan.round_id),
        Some(2),
        "`RoundStart` cũ không được làm lùi kế hoạch round"
    );

    // Snapshot của round mới vẫn được áp: client không bị chặn bởi `RoundStart` cũ.
    let mut next = solo_state(7);
    next.tick = 6;
    client.push_snapshot(&solo_snapshot(&next, 2, 6));
    assert_eq!(
        client.net.round_id(),
        Some(2),
        "client vẫn phải nhận snapshot của round mới"
    );

    // Input đi kèm **đúng round** và **đúng slot** của round mới.
    client
        .net
        .predict(PlayerInput {
            dir: Dir::Right,
            place_bomb: false,
        })
        .expect("dự đoán");
    client.net.resend().expect("gửi cửa sổ lệnh");
    let batch = client.take_batch().expect("phải có batch input");
    assert_eq!(batch.round_id, 2, "input phải mang round mới");
    assert_eq!(
        batch.player, SOLO_SLOT,
        "input phải mang slot trận của round mới"
    );
}

/// Nhịp gửi lại trong test: nhỏ hơn mặc định (500 ms) để test không phải chờ lâu, nhưng
/// **khác 0** — nhịp 0 khiến hai nhịp (lệnh phòng và ack) luôn trùng nhau nên lỗi "hai nhịp
/// dùng chung đồng hồ" bị che.
const SOLO_RETRY_INTERVAL: Duration = Duration::from_millis(20);

/// Trần **số vòng** chờ nhịp gửi lại của client (nhịp dùng đồng hồ thật trong client).
///
/// Điều kiện dừng là "client đã gửi lại `ResultAck`", không phải hằng số thời gian ⇒ máy
/// chậm vẫn xanh.
const RETRY_WAIT_ROUNDS: u32 = 200;

#[test]
fn result_ack_mat_frame_duoc_gui_lai_khi_lenh_phong_con_pending() {
    // Nhịp gửi lại của **lệnh phòng** và của **ack** phải là hai đồng hồ riêng: nếu chung,
    // mỗi lần gửi lại lệnh phòng đẩy mốc của ack về hiện tại ⇒ ack không bao giờ được gửi
    // lại ⇒ frame `ResultAck` mất thì client phải chờ timeout phía server.
    let mut client = SoloClient::with_retry_interval(SOLO_RETRY_INTERVAL);
    // Luồng tự động: client vào phòng (server giả trả `RoomState`) và bật ready ⇒ lệnh phòng
    // `SetReady` **đang chờ** xác nhận nên nhịp gửi lại lệnh phòng chạy liên tục.
    client
        .net
        .enable_auto_flow(RoomCharacter::C00, 2)
        .expect("bật luồng tự động");
    client.push_room_state();
    // Lượt đang chơi: `RoundStart` + snapshot tick > 0 ⇒ lượt đã chạy (bằng chứng server đã
    // nhận `LoadingAck`) nên nhịp gửi lại loading ack không chen vào.
    client.push_round_start_raw(&SoloClient::round_start_of(2, SOLO_SLOT));
    let mut state = solo_state(7);
    state.tick = 5;
    client.push_snapshot(&solo_snapshot(&state, 2, 5));

    // Kết quả round: client gửi `ResultAck` **đầu tiên** — frame này bị mất (đọc rồi bỏ, không
    // tới server).
    client.push_round_result(2);
    let kinds = client.drain_kinds();
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == MessageKind::ResultAck)
            .count(),
        1,
        "client phải gửi `ResultAck` khi nhận kết quả: {kinds:?}"
    );

    // Chờ hơn một nhịp rồi bơm: nhịp gửi lại lệnh phòng vẫn đang chạy (`SetReady` còn
    // pending) nhưng ack phải được gửi lại **riêng**, không nhờ timeout phía server.
    //
    // Nhịp gửi lại của client dùng **đồng hồ thật** (`Instant`) ⇒ vòng chờ ở đây dùng thời
    // gian thật, nhưng **điều kiện** quyết định khi nào dừng: tới khi client đã gửi lại
    // `ResultAck` (không phụ thuộc một hằng số thời gian nào).
    let mut kinds = Vec::new();
    for _ in 0..RETRY_WAIT_ROUNDS {
        thread::sleep(SOLO_RETRY_INTERVAL);
        client.net.pump(PUMP_TIMEOUT).expect("bơm message");
        kinds.extend(client.drain_kinds());
        if kinds.contains(&MessageKind::ResultAck) {
            break;
        }
    }
    assert!(
        kinds.contains(&MessageKind::RoomCommand),
        "lệnh phòng vẫn phải được gửi lại: {kinds:?}"
    );
    assert!(
        kinds.contains(&MessageKind::ResultAck),
        "`ResultAck` mất phải được gửi lại, không nhờ timeout phía server: {kinds:?}"
    );
}

#[test]
fn round_result_lap_va_cu_khong_kich_hoat_lai_ui() {
    let mut client = SoloClient::new();
    client.push_round_start(2);
    client.push_round_result(2);
    let result = client.net.room().result().cloned().unwrap();
    client.drain_kinds();
    client.push_round_result(2);
    assert_eq!(client.net.room().result(), Some(&result));
    assert!(!client.drain_kinds().contains(&MessageKind::ResultAck));
    client.push_round_start(3);
    client.drain_kinds();
    client.push_round_result(2);
    assert!(!client.drain_kinds().contains(&MessageKind::ResultAck));
}

/// Nhịp retry độc lập auto-ready, tăng backoff và dừng khi hết retention hoặc đổi round.
#[test]
fn round_result_retry_backoff_va_dung() {
    let mut client = SoloClient::new();
    client.push_round_start(2);
    let mut state = solo_state(7);
    state.phase = boom_core::Phase::Ended(boom_core::Outcome::Draw);
    client.push_snapshot(&solo_snapshot(&state, 2, 1));
    client.drain_kinds();
    for tick in 2..=28 {
        client.net.pump(Duration::ZERO).unwrap();
        let kinds = client.drain_kinds();
        assert_eq!(
            kinds.contains(&MessageKind::RoundResultRequest),
            [4, 12, 28].contains(&tick),
            "pump {tick}"
        );
    }
    client
        .net
        .handle(ClientMessage::RoomRejected(boom_proto::RoomRejected {
            request: 2,
            reason: boom_proto::RoomRejectReason::StaleRequest,
        }))
        .unwrap();
    for _ in 0..128 {
        client.net.pump(Duration::ZERO).unwrap();
    }
    assert!(
        !client
            .drain_kinds()
            .contains(&MessageKind::RoundResultRequest)
    );
    client.push_round_start(3);
    client.drain_kinds();
    for _ in 0..128 {
        client.net.pump(Duration::ZERO).unwrap();
    }
    assert!(
        !client
            .drain_kinds()
            .contains(&MessageKind::RoundResultRequest)
    );
}

#[test]
fn round_result_round_ended_event_cung_mo_recovery() {
    let mut client = SoloClient::new();
    client.push_round_start(2);
    client.drain_kinds();
    client
        .net
        .handle(ClientMessage::RoundEvents(boom_proto::RoundEvents {
            round_id: 2,
            events: vec![boom_proto::RoundEvent {
                id: boom_proto::EventId { tick: 100, seq: 0 },
                name: "round_ended".into(),
                fields: vec![],
            }],
        }))
        .unwrap();
    for _ in 0..4 {
        client.net.pump(Duration::ZERO).unwrap();
    }
    assert!(
        client
            .drain_kinds()
            .contains(&MessageKind::RoundResultRequest)
    );
    client.push_round_result(2);
    client.drain_kinds();
    for _ in 0..128 {
        client.net.pump(Duration::ZERO).unwrap();
    }
    assert!(
        !client
            .drain_kinds()
            .contains(&MessageKind::RoundResultRequest)
    );
}

#[test]
fn client_predict_stamps_target_tick_from_clock_and_resend_keeps_payload() {
    let mut client = SoloClient::new();
    let state = solo_state(7);
    client.push_round_start(1);
    client.push_snapshot(&solo_snapshot(&state, 1, 1));

    // Đồng hồ kiểm soát: snapshot đã nhận cách đây 100 ms, RTT đo 50 ms.
    // target_tick phải là tick authority ước lượng, không phải tick state dự đoán.
    let now = Instant::now();
    client.net.snapshot_received_at = Some(now - Duration::from_millis(100));
    client.net.input_rtt = Some(Duration::from_millis(50));
    let confirmed_tick = client.net.confirmed_state().expect("confirmed").tick;
    let expected = client.net.input_target_tick_at(now);
    assert!(
        expected > confirmed_tick,
        "clock estimate phải vượt tick confirmed: {expected} <= {confirmed_tick}"
    );

    client
        .net
        .predict(PlayerInput {
            dir: Dir::Right,
            place_bomb: true,
        })
        .expect("dự đoán");
    client.net.resend().expect("gửi batch");
    let first = client
        .take_batch()
        .expect("phải có batch input")
        .commands
        .into_iter()
        .next()
        .expect("batch có command");
    assert!(
        first.target_tick >= expected,
        "target_tick phải lấy từ clock: {} < {expected}",
        first.target_tick
    );

    // Resend phải giữ nguyên toàn bộ payload của lệnh đã gửi, kể cả target_tick.
    client.net.resend().expect("resend batch");
    let second = client
        .take_batch()
        .expect("phải có batch resend")
        .commands
        .into_iter()
        .next()
        .expect("batch resend có command");
    assert_eq!(second, first, "resend phải giữ payload bất biến");
}

#[test]
fn producer_batch_moves_and_places_bomb_with_controlled_fixture_clock() {
    // Mạng có trễ 100 ms nhưng thời gian do fixture điều khiển; target_tick được đặt
    // tường minh theo tick server + 8 (đủ cho trễ 6 tick ở 60 Hz). Đây không phải
    // `TimingMode::Virtual` giao ngay: `ScheduledTransport` chỉ giao khi `advance_to`.
    let out = ScheduledFaults {
        delay_ms: 100,
        ..ScheduledFaults::none()
    };
    let mut harness = ScheduledHarness::new(out, ScheduledFaults::none(), 20261024);
    assert_eq!(harness.out_faults().delay_ms, 100);
    assert_eq!(harness.in_faults(), ScheduledFaults::none());
    harness.start_round();
    let player = harness.client(0).player;
    let start = harness.start_position(player).expect("có vị trí đầu");

    for _step in 0..24 {
        let target = harness.server_tick().unwrap_or(0) + 4;
        harness.set_target_override(0, target);
        harness.set_target_override(1, target);
        harness.tick([Dir::Right, Dir::Right], [true, false]);
    }

    let tick = harness.server_tick().expect("server đã chạy");
    let end = harness
        .server_position(player, tick)
        .expect("có vị trí server");
    assert!(
        end.0 > start.0,
        "producer→batch→authority phải làm người chơi di chuyển: {start:?} -> {end:?}"
    );
    assert!(
        harness.server_bomb_count() > 0,
        "command đặt bom phải được authority áp dụng"
    );
    assert!(
        harness.retired_commands().is_empty(),
        "target_tick điều khiển đúng không được retire: {:?}",
        harness.retired_commands()
    );
}
