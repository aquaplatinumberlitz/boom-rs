//! Fixture mạng cho test end-to-end: **lịch giao theo tick của server**.
//!
//! `MemoryTransport` của `boom-net` có đồng hồ ảo **riêng cho mỗi cặp transport** và giao frame
//! ngay khi người nhận hỏi: độ trễ chỉ đổi *thứ tự* giao, không làm input tới trễ so với tick
//! server, nên "delay/jitter bao nhiêu tick server" không được chứng minh. Fixture này gắn
//! đồng hồ mạng với thời gian server: mỗi frame có **mốc giao** (`deliver_at`) tính từ thời
//! điểm gửi, và chỉ được giao khi đồng hồ mạng đi tới mốc đó. Nhờ vậy test chứng minh được:
//! input tới trễ **nhiều tick server**, mất, trùng và đảo thứ tự — và đo được vì sao.
//!
//! Fixture chạy `boom_server::Authority` trực tiếp (không qua vòng IO `Server`) để mọi bước
//! đều do test điều khiển: không có luồng nào tự quyết khi nào gửi, nên thứ tự giao **tất định**
//! theo seed trên mọi OS.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStats, Dir, PlayerInput, Rules, Stat, World, mode_by_id};
use boom_net::{NetError, Transport};
use boom_proto::{
    Hello, InputBatch, InputRejectReason, InputRejected, ProtocolLimits, RoundEvent, Welcome,
    decode_frame, encode_frame,
};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand};
use boom_server::{Authority, CharacterStatsTable, InputError, MatchSetup, ServerConfig};

use super::{ClientSession, NetClient, hello_for_profile};

/// Profile rules cố định cho test (khác `data/rules/v1-dev.toml` nhưng cùng schema).
const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

/// Thời gian chờ message khi bơm hàng đợi đọc.
pub const PUMP_TIMEOUT: Duration = Duration::from_millis(20);

/// Ngưỡng "trễ nhiều tick" dùng trong test: input phải tới trễ hơn ngần này so với lúc gửi.
///
/// 50 ms ở 60 Hz là 3 tick: đủ để chứng minh input tới trễ **nhiều tick server** (không phải
/// trễ trong cùng một lần gọi) mà vẫn nằm trong ngân sách `INPUT_LATE` (100 ms) của giao thức.
pub const LATE_THRESHOLD_MS: u64 = 50;

/// Chu kỳ gửi snapshot (ms), giống server thật (`HandshakeConfig::snapshot_interval_ms`).
const SNAPSHOT_INTERVAL_MS: u64 = 50;

/// PRNG tất định (SplitMix64): cùng seed ⇒ cùng lỗi mạng trên mọi OS.
///
/// `boom-net` có PRNG riêng nhưng không xuất ra ngoài crate, nên fixture có bản của riêng mình.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Số nguyên trong `[0, bound)`; `bound` = 0 thì trả 0.
    fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        u32::try_from(self.next_u64() % u64::from(bound)).expect("phần dư nhỏ hơn bound")
    }
}

/// Lỗi mạng mô phỏng (tương tự `boom_net::MemoryFaults` nhưng lịch giao theo tick server).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScheduledFaults {
    /// Tỉ lệ mất frame (phần nghìn, 0..=1000).
    pub loss_permille: u16,
    /// Tỉ lệ gửi trùng frame (phần nghìn, 0..=1000).
    pub duplicate_permille: u16,
    /// Tỉ lệ đảo thứ tự (phần nghìn, 0..=1000).
    pub reorder_permille: u16,
    /// Trễ cố định của mọi frame (ms).
    pub delay_ms: u32,
    /// Trễ ngẫu nhiên thêm trong `[0, jitter_ms]` (ms).
    pub jitter_ms: u32,
    /// Cửa sổ đảo thứ tự: trễ thêm ngẫu nhiên trong `[1, reorder_window_ms]` (ms).
    pub reorder_window_ms: u32,
}

impl ScheduledFaults {
    /// Không mô phỏng lỗi nào.
    pub const fn none() -> Self {
        Self {
            loss_permille: 0,
            duplicate_permille: 0,
            reorder_permille: 0,
            delay_ms: 0,
            jitter_ms: 0,
            reorder_window_ms: 0,
        }
    }
}

/// Thống kê lỗi mạng **đã xảy ra**: test dùng để chứng minh fixture không rỗng.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetStats {
    /// Số frame đã gửi (không tính frame bị mất).
    pub sent: u32,
    /// Số frame bị mạng nuốt.
    pub lost: u32,
    /// Số frame được gửi trùng.
    pub duplicated: u32,
    /// Số frame có mốc giao sau mốc gửi (bị trễ).
    pub delayed: u32,
    /// Trễ lớn nhất quan sát được (ms) — dùng để chứng minh input tới trễ **nhiều tick
    /// server**, không chỉ trễ "trong cùng một lần gọi".
    pub max_late_ms: u64,
    /// Số frame được giao **đảo thứ tự** so với lúc gửi.
    pub reordered: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endpoint {
    A,
    B,
}

impl Endpoint {
    fn peer(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }
}

/// Một frame đang trên đường.
#[derive(Clone, Debug)]
struct Packet {
    /// Đích của frame.
    to: Endpoint,
    /// Số thứ tự gửi (khoá phụ khi trùng mốc giao; cũng để đo đảo thứ tự).
    seq: u64,
    /// Mốc giao (ms mạng) — frame chỉ được giao khi đồng hồ mạng tới mốc này.
    deliver_at: u64,
    /// Thời điểm gửi (ms mạng) — để đo trễ.
    sent_at: u64,
    payload: Vec<u8>,
}

#[derive(Default)]
struct Shared {
    /// Đồng hồ mạng (ms) — bằng thời gian server.
    now_ms: u64,
    /// Frame đang trên đường.
    packets: Vec<Packet>,
    /// Frame đã giao, chờ ứng dụng lấy (mỗi endpoint một hàng đợi).
    ready: [VecDeque<Vec<u8>>; 2],
    /// Số thứ tự cấp cho frame gửi đi.
    next_seq: u64,
    /// Endpoint nào đã đóng.
    closed: [bool; 2],
    /// Số thứ tự gửi lớn nhất đã giao tới mỗi endpoint (để đo đảo thứ tự).
    delivered_max: [u64; 2],
    /// Thống kê lỗi mạng đã xảy ra.
    stats: NetStats,
    /// Lỗi mạng áp cho chiều này; **bật sau handshake** (mất `Hello` thì phiên hỏng, không
    /// phải lỗi mạng).
    faults: ScheduledFaults,
}

/// Mạng ảo có lịch giao theo thời gian server.
#[derive(Default)]
pub struct ScheduledNet {
    inner: Mutex<Shared>,
    signal: Condvar,
}

impl ScheduledNet {
    /// Đưa đồng hồ mạng tới `now_ms` và giao các frame đã đến hạn.
    ///
    /// Frame được giao theo thứ tự `(mốc giao, số thứ tự gửi)` — đúng thứ tự mà mạng thật dùng
    /// với độ trễ khác nhau.
    pub fn advance_to(&self, now_ms: u64) {
        let mut inner = self.inner.lock().expect("khoá mạng ảo");
        inner.now_ms = inner.now_ms.max(now_ms);
        let now = inner.now_ms;
        let mut ready: Vec<Packet> = inner
            .packets
            .iter()
            .filter(|packet| packet.deliver_at <= now)
            .cloned()
            .collect();
        inner.packets.retain(|packet| packet.deliver_at > now);
        ready.sort_by_key(|packet| (packet.deliver_at, packet.seq));
        for packet in ready {
            let index = packet.to.index();
            let late = packet.deliver_at.saturating_sub(packet.sent_at);
            if late > inner.stats.max_late_ms {
                inner.stats.max_late_ms = late;
            }
            // Frame gửi sau mà giao trước ⇒ đảo thứ tự.
            if packet.seq < inner.delivered_max[index] {
                inner.stats.reordered += 1;
            }
            inner.delivered_max[index] = inner.delivered_max[index].max(packet.seq);
            inner.ready[index].push_back(packet.payload);
        }
        drop(inner);
        self.signal.notify_all();
    }

    /// Thống kê lỗi mạng đã xảy ra.
    pub fn stats(&self) -> NetStats {
        self.inner.lock().expect("khoá mạng ảo").stats
    }

    /// Bật lỗi mạng cho chiều này (gọi **sau** handshake).
    pub fn set_faults(&self, faults: ScheduledFaults) {
        self.inner.lock().expect("khoá mạng ảo").faults = faults;
    }
}

/// Một đầu của mạng ảo có lịch giao.
pub struct ScheduledTransport {
    net: Arc<ScheduledNet>,
    endpoint: Endpoint,
    rng: Rng,
    closed: bool,
}

impl ScheduledTransport {
    /// Tạo một cặp endpoint nối với nhau; cả hai dùng chung một mạng ảo.
    ///
    /// Lỗi mạng **chưa** bật: fixture bật sau handshake (xem [`ScheduledNet::set_faults`]).
    pub fn pair(seed: u64) -> (Self, Self) {
        let net = Arc::new(ScheduledNet::default());
        // Mỗi chiều có dòng PRNG riêng để lỗi mạng hai chiều không phụ thuộc nhau.
        let a = Self {
            net: Arc::clone(&net),
            endpoint: Endpoint::A,
            rng: Rng::new(seed.wrapping_add(0x9E37_79B9_7F4A_7C15)),
            closed: false,
        };
        let b = Self {
            net,
            endpoint: Endpoint::B,
            rng: Rng::new(seed.wrapping_add(0x3C6E_F372_FE94_F82B)),
            closed: false,
        };
        (a, b)
    }

    /// Handle của mạng ảo để test điều khiển đồng hồ.
    pub fn net(&self) -> Arc<ScheduledNet> {
        Arc::clone(&self.net)
    }

    /// Lấy frame đã giao mà **không chờ** (phía server của fixture).
    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        let mut inner = self.net.inner.lock().expect("khoá mạng ảo");
        inner.ready[self.endpoint.index()].pop_front()
    }

    fn roll(&mut self, permille: u16) -> bool {
        if permille == 0 {
            return false;
        }
        self.rng.below(1000) < u32::from(permille)
    }
}

impl Transport for ScheduledTransport {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        if self.closed {
            return Err(NetError::Closed);
        }
        let faults = self.net.inner.lock().expect("khoá mạng ảo").faults;
        // Quyết định lỗi mạng trước khi khoá mạng (PRNG thuộc endpoint này); thứ tự gọi PRNG
        // cố định nên cùng seed cho cùng thứ tự gửi ⇒ cùng lỗi mạng trên mọi OS.
        let lost = self.roll(faults.loss_permille);
        let jitter = if faults.jitter_ms == 0 {
            0
        } else {
            self.rng.below(faults.jitter_ms + 1)
        };
        let reordered = self.roll(faults.reorder_permille);
        let reorder_extra = if reordered {
            self.rng.below(faults.reorder_window_ms.max(1))
        } else {
            0
        };
        let duplicate = self.roll(faults.duplicate_permille);
        let mut inner = self.net.inner.lock().expect("khoá mạng ảo");
        let sent_at = inner.now_ms;
        let seq = inner.next_seq;
        inner.next_seq += 1;
        if lost {
            // Mạng nuốt frame: ứng dụng gửi vẫn thành công, giống mạng thật.
            inner.stats.lost += 1;
            return Ok(());
        }
        let delay = u64::from(faults.delay_ms) + u64::from(jitter) + u64::from(reorder_extra);
        inner.stats.sent += 1;
        if delay > 0 {
            inner.stats.delayed += 1;
        }
        let to = self.endpoint.peer();
        inner.packets.push(Packet {
            to,
            seq,
            deliver_at: sent_at + delay,
            sent_at,
            payload: frame.to_vec(),
        });
        if duplicate {
            let dup_seq = inner.next_seq;
            inner.next_seq += 1;
            inner.packets.push(Packet {
                to,
                seq: dup_seq,
                deliver_at: sent_at + delay + 1,
                sent_at,
                payload: frame.to_vec(),
            });
            inner.stats.duplicated += 1;
        }
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        if self.closed {
            return Err(NetError::Closed);
        }
        let mut inner = self.net.inner.lock().expect("khoá mạng ảo");
        loop {
            if let Some(frame) = inner.ready[self.endpoint.index()].pop_front() {
                return Ok(Some(frame));
            }
            if inner.closed[self.endpoint.peer().index()]
                && !inner
                    .packets
                    .iter()
                    .any(|packet| packet.to == self.endpoint)
            {
                // Đối phương đóng và không còn frame nào tới: EOF.
                return Ok(None);
            }
            inner = self.net.signal.wait(inner).expect("khoá mạng ảo");
        }
    }

    fn close(&mut self) -> Result<(), NetError> {
        self.closed = true;
        {
            let mut inner = self.net.inner.lock().expect("khoá mạng ảo");
            inner.closed[self.endpoint.index()] = true;
        }
        self.net.signal.notify_all();
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

/// Chỉ số của người chơi trong state của server.
pub fn stats() -> CharacterStats {
    CharacterStats {
        speed: Stat::new(1, 5),
        bombs: Stat::new(1, 5),
        range: Stat::new(1, 5),
    }
}

/// Địa hình mở: mọi spawn đều có chỗ đi, để kịch bản luôn có chuyển động thật.
pub fn terrain() -> Terrain {
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

/// Một client trong fixture.
pub struct ScheduledClient {
    /// Client mạng (đọc snapshot, gửi input).
    pub net: NetClient<ScheduledTransport>,
    /// Người chơi trên server.
    pub player: PlayerId,
    /// Slot của người chơi (id trong state).
    pub slot: u8,
    /// Endpoint phía server của chiều client→server (nhận input).
    out_server: ScheduledTransport,
    /// Endpoint phía server của chiều server→client (gửi snapshot/event).
    in_server: ScheduledTransport,
    /// Mạng chiều client→server (điều khiển đồng hồ).
    out_net: Arc<ScheduledNet>,
    /// Mạng chiều server→client (điều khiển đồng hồ).
    in_net: Arc<ScheduledNet>,
    /// Event đã nhận (đã dedup theo `(round_id, EventId)`).
    events: Vec<RoundEvent>,
    /// Sequence đã gửi lên server (để đếm lệnh trùng).
    sent_seqs: BTreeSet<u32>,
}

/// Fixture: server authority + hai client, lịch giao theo tick của server.
pub struct ScheduledHarness {
    authority: Authority,
    clients: Vec<ScheduledClient>,
    /// State hash của server theo tick.
    hashes: BTreeMap<u32, u64>,
    /// Vị trí người chơi ở server theo (người chơi, tick).
    server_positions: BTreeMap<(PlayerId, u32), (i32, i32)>,
    /// Vị trí client dự đoán theo (người chơi, tick).
    predicted_positions: BTreeMap<(PlayerId, u32), (i32, i32)>,
    /// Vị trí lúc bắt đầu lượt (tâm ô spawn), để chứng minh client đã di chuyển.
    start_positions: BTreeMap<PlayerId, (i32, i32)>,
    /// Mốc thời gian lúc lượt bắt đầu.
    origin_ms: u64,
    /// Số tick server đã chạy.
    ticks: u64,
    /// Tần số tick lấy từ rules profile.
    hz: u32,
    /// `snapshot_seq` gửi cho client.
    snapshot_seq: u32,
    /// Mốc thời gian gửi snapshot gần nhất.
    last_snapshot_ms: u64,
    /// Số batch input đã nhận (kể cả batch trùng).
    input_batches: u32,
    /// Số lệnh input **trùng sequence** đã nhận (đã từng gửi trước đó).
    duplicate_commands: u32,
    /// Số lần vị trí dự đoán lệch vị trí server ở cùng tick.
    mismatches: u32,
    /// Số lần client phải tính lại dự đoán (snapshot tới khi còn lệnh chưa ack).
    corrections: u32,
    /// Số batch bị server từ chối.
    rejects: u32,
    /// Lý do từ chối gần nhất.
    last_reject: Option<String>,
}

impl ScheduledHarness {
    /// Dựng fixture: hai client handshake vào server (authority) qua mạng có lịch giao.
    ///
    /// `out_faults` áp cho chiều client→server (khung input), `in_faults` cho chiều
    /// server→client (snapshot/event).
    pub fn new(out_faults: ScheduledFaults, in_faults: ScheduledFaults, seed: u64) -> Self {
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
        let mut clients = Vec::new();
        let hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
        for (index, player) in [PlayerId(1), PlayerId(2)].into_iter().enumerate() {
            let slot = u8::try_from(index).expect("slot");
            // Hai chiều tách riêng: client→server và server→client.
            let (client_writer, mut server_reader) =
                ScheduledTransport::pair(seed.wrapping_add(index as u64));
            let (mut server_writer, client_reader) =
                ScheduledTransport::pair(seed.wrapping_add(index as u64));
            let out_net = server_reader.net();
            let in_net = server_writer.net();
            // Handshake: client gửi `Hello`, fixture trả `Welcome` (như server thật).
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
            // Chờ `Hello` rồi trả `Welcome`.
            let frame = loop {
                out_net.advance_to(0);
                if let Some(frame) = server_reader.try_recv() {
                    break frame;
                }
                thread::sleep(Duration::from_millis(1));
            };
            let hello_in: Hello =
                decode_frame(&frame, &ProtocolLimits::v1()).expect("decode hello");
            assert_eq!(
                hello_in.rules_hash, rules.profile_hash,
                "client phải khai đúng hash rules"
            );
            let welcome = Welcome {
                session_id: format!("fixture-{index}"),
                player_id: slot,
                wire_schema: boom_proto::limits::WIRE_SCHEMA,
                state_schema: 1,
                rules_profile_id: profile.profile_id.clone(),
                rules_hash: rules.profile_hash,
                content_epoch: 1,
                snapshot_interval_ms: SNAPSHOT_INTERVAL_MS as u32,
                protocol_limits_version: ProtocolLimits::v1().revision,
            };
            server_writer
                .send_frame(&encode_frame(&welcome, &ProtocolLimits::v1()).expect("encode welcome"))
                .expect("gửi welcome");
            in_net.advance_to(0);
            let net = handle.join().expect("join luồng client");
            // Lỗi mạng chỉ bật **sau** handshake: mất `Hello`/`Welcome` là hỏng phiên chứ không
            // phải lỗi mạng.
            out_net.set_faults(out_faults);
            in_net.set_faults(in_faults);
            clients.push(ScheduledClient {
                net,
                player,
                slot,
                out_server: server_reader,
                in_server: server_writer,
                out_net,
                in_net,
                events: Vec::new(),
                sent_seqs: BTreeSet::new(),
            });
        }
        Self {
            authority,
            clients,
            hashes: BTreeMap::new(),
            server_positions: BTreeMap::new(),
            predicted_positions: BTreeMap::new(),
            start_positions: BTreeMap::new(),
            origin_ms: 0,
            ticks: 0,
            hz,
            snapshot_seq: 0,
            last_snapshot_ms: 0,
            input_batches: 0,
            duplicate_commands: 0,
            mismatches: 0,
            corrections: 0,
            rejects: 0,
            last_reject: None,
        }
    }

    /// Dựng phòng, ready, start và qua loading barrier: lượt bắt đầu ở tick 0.
    pub fn start_round(&mut self) {
        let host = PlayerId(1);
        let now = 0;
        self.authority
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
        let room = self.authority.room_view().expect("có phòng").id;
        self.authority
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
            self.authority
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
        self.authority
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
            self.authority
                .ack_loading(player, now)
                .expect("ack loading");
        }
        self.origin_ms = now;
        for client in &self.clients {
            if let Some(position) = self.authority.player_position(client.player) {
                self.start_positions.insert(client.player, position);
            }
        }
    }

    /// Một bước: giao frame đến hạn, client nhận snapshot rồi dự đoán, server nhận input và
    /// chạy tick, cuối cùng server gửi snapshot/event.
    pub fn tick(&mut self, dirs: [Dir; 2], bombs: [bool; 2]) {
        let now_ms = self.next_ms();
        // 1. Giao các frame đã đến hạn (snapshot/event của tick trước, input tới trễ).
        self.advance_network(now_ms);
        // 2. Client nhận message, dự đoán một bước và gửi cửa sổ lệnh chưa ack.
        for (index, client) in self.clients.iter_mut().enumerate() {
            let pending_before = client.net.pending_len();
            let confirmed_before = client.net.confirmed_state().map_or(0, |state| state.tick);
            client.net.pump(PUMP_TIMEOUT).expect("bơm message");
            client.events.extend(client.net.take_events());
            let confirmed_after = client.net.confirmed_state().map_or(0, |state| state.tick);
            if confirmed_after != confirmed_before && pending_before > 0 {
                // Snapshot tới khi còn lệnh chưa ack: client phải tính lại dự đoán (correction).
                self.corrections += 1;
            }
            client
                .net
                .predict(PlayerInput {
                    dir: dirs[index],
                    place_bomb: bombs[index],
                })
                .expect("dự đoán");
            client.net.resend().expect("gửi cửa sổ lệnh");
        }
        // 3. Input gửi ở bước 2 có mốc giao = now + trễ: phần đến hạn được giao, phần còn lại
        //    chờ các tick sau.
        self.advance_network(now_ms);
        // 4. Server nhận input đến hạn rồi chạy tick.
        self.receive_input(now_ms);
        self.authority.advance_to(now_ms).expect("server chạy tick");
        self.record_server_state();
        // 5. Server gửi snapshot và event cho client.
        self.send_snapshot_and_events(now_ms);
        // 6. Ghi lại vị trí client dự đoán và so với server ở cùng tick.
        self.record_predicted_positions();
    }

    /// Drain tới **mốc đồng bộ**: client hết lệnh chưa ack và state dự đoán bằng state
    /// confirmed. Trả về số vòng đã drain.
    ///
    /// Trong lúc drain client **không** sinh lệnh mới: server chỉ còn việc ack lệnh đã gửi và
    /// gửi snapshot, để client hội tụ về state của server.
    pub fn drain(&mut self) -> u32 {
        for round in 0..200 {
            let now_ms = self.next_ms();
            self.advance_network(now_ms);
            for client in &mut self.clients {
                client.net.pump(PUMP_TIMEOUT).expect("bơm message");
                client.events.extend(client.net.take_events());
                // Client không sinh lệnh mới nhưng vẫn gửi lại cửa sổ chưa ack (ADR 0009 §3.2).
                client.net.resend().expect("gửi lại cửa sổ");
            }
            self.advance_network(now_ms);
            self.receive_input(now_ms);
            self.authority.advance_to(now_ms).expect("server chạy tick");
            self.record_server_state();
            self.send_snapshot_and_events(now_ms);
            self.record_predicted_positions();
            if self.clients.iter().all(|client| {
                client.net.pending_len() == 0
                    && client
                        .net
                        .state()
                        .zip(client.net.confirmed_state())
                        .is_some_and(|(predicted, confirmed)| predicted.tick == confirmed.tick)
            }) {
                return round + 1;
            }
        }
        panic!("client không hội tụ sau 200 vòng drain");
    }

    /// Thống kê lỗi mạng đã xảy ra trên **cả hai chiều**.
    pub fn net_stats(&self) -> NetStats {
        let mut total = NetStats::default();
        for client in &self.clients {
            for stats in [client.out_net.stats(), client.in_net.stats()] {
                total.sent += stats.sent;
                total.lost += stats.lost;
                total.duplicated += stats.duplicated;
                total.delayed += stats.delayed;
                total.max_late_ms = total.max_late_ms.max(stats.max_late_ms);
                total.reordered += stats.reordered;
            }
        }
        total
    }

    /// Số batch input server đã nhận (kể cả batch trùng).
    pub fn input_batches(&self) -> u32 {
        self.input_batches
    }

    /// Số lệnh input **trùng sequence** server đã nhận.
    pub fn duplicate_commands(&self) -> u32 {
        self.duplicate_commands
    }

    /// Số lần vị trí dự đoán lệch vị trí server ở cùng tick.
    pub fn mismatches(&self) -> u32 {
        self.mismatches
    }

    /// Số lần client phải tính lại dự đoán (snapshot tới khi còn lệnh chưa ack).
    pub fn corrections(&self) -> u32 {
        self.corrections
    }

    /// Vị trí server của người chơi ở tick đã cho.
    pub fn server_position(&self, player: PlayerId, tick: u32) -> Option<(i32, i32)> {
        self.server_positions.get(&(player, tick)).copied()
    }

    /// State hash của server tại tick đã cho.
    pub fn server_hash(&self, tick: u32) -> Option<u64> {
        self.hashes.get(&tick).copied()
    }

    /// Vị trí lúc bắt đầu lượt (để chứng minh người chơi đã di chuyển).
    pub fn start_position(&self, player: PlayerId) -> Option<(i32, i32)> {
        self.start_positions.get(&player).copied()
    }

    /// Client trong fixture.
    pub fn client(&self, index: usize) -> &ScheduledClient {
        &self.clients[index]
    }

    /// Event đã nhận của client `index`.
    pub fn events(&self, index: usize) -> &[RoundEvent] {
        &self.clients[index].events
    }

    /// Mốc thời gian của tick kế tiếp (server tick `ticks`).
    fn next_ms(&mut self) -> u64 {
        let now = self.origin_ms + (self.ticks * 1_000) / u64::from(self.hz);
        self.ticks += 1;
        now
    }

    fn advance_network(&self, now_ms: u64) {
        for client in &self.clients {
            client.out_net.advance_to(now_ms);
            client.in_net.advance_to(now_ms);
        }
    }

    fn receive_input(&mut self, now_ms: u64) {
        let limits = ProtocolLimits::v1();
        for client in &mut self.clients {
            while let Some(frame) = client.out_server.try_recv() {
                let batch: InputBatch = decode_frame(&frame, &limits).expect("decode input batch");
                self.input_batches += 1;
                for command in &batch.commands {
                    if !client.sent_seqs.insert(command.seq) {
                        self.duplicate_commands += 1;
                    }
                }
                if let Err(error) = self.authority.submit_input(client.player, &batch, now_ms) {
                    // Server từ chối batch: gửi `InputRejected` như server thật.
                    self.rejects += 1;
                    self.last_reject = Some(error.to_string());
                    let applied = self.authority.applied_seq(client.player).unwrap_or(0);
                    let server_tick = self.authority.tick().unwrap_or(0);
                    let rejected = reject_for(&batch, &error, applied, server_tick);
                    let _ = client.in_server.send_frame(
                        &encode_frame(&rejected, &limits).expect("encode input rejected"),
                    );
                }
            }
        }
    }

    fn record_server_state(&mut self) {
        let Some(tick) = self.authority.tick() else {
            return;
        };
        let hash = self.authority.state_hash().expect("state hash");
        self.hashes.insert(tick, hash);
        for client in &self.clients {
            if let Some(position) = self.authority.player_position(client.player) {
                self.server_positions
                    .insert((client.player, tick), position);
            }
        }
    }

    fn send_snapshot_and_events(&mut self, now_ms: u64) {
        let limits = ProtocolLimits::v1();
        let Some(round_id) = self.authority.current_round_id() else {
            return;
        };
        let events = self.authority.take_events();
        let send_events = |clients: &mut Vec<ScheduledClient>| {
            if events.is_empty() {
                return;
            }
            let message = boom_sync::to_round_events(round_id, &events);
            let frame = encode_frame(&message, &limits).expect("encode round events");
            for client in clients.iter_mut() {
                let _ = client.in_server.send_frame(&frame);
            }
        };
        if now_ms < self.last_snapshot_ms + SNAPSHOT_INTERVAL_MS {
            // Chưa tới chu kỳ gửi snapshot; event vẫn gửi ngay (như server thật).
            send_events(&mut self.clients);
            return;
        }
        self.last_snapshot_ms = now_ms;
        self.snapshot_seq += 1;
        for client in &mut self.clients {
            if let Some(Ok(snapshot)) = self.authority.snapshot(self.snapshot_seq, 1) {
                let frame = encode_frame(&snapshot, &limits).expect("encode snapshot");
                let _ = client.in_server.send_frame(&frame);
            }
        }
        send_events(&mut self.clients);
    }

    fn record_predicted_positions(&mut self) {
        for client in &self.clients {
            let Some(state) = client.net.state() else {
                continue;
            };
            let tick = state.tick;
            if let Some(player) = state.player(client.slot) {
                let predicted = (player.x.0, player.y.0);
                self.predicted_positions
                    .insert((client.player, tick), predicted);
                // So với vị trí server ở cùng tick: chứng minh dự đoán có bị lệch (và được
                // sửa bởi correction) khi mạng trễ/mất.
                if let Some(server) = self.server_positions.get(&(client.player, tick))
                    && *server != predicted
                {
                    self.mismatches += 1;
                }
            }
        }
    }
}

/// Dựng `InputRejected` từ lỗi scheduler (giống server thật).
fn reject_for(
    batch: &InputBatch,
    error: &InputError,
    applied_seq: u32,
    server_tick: u32,
) -> InputRejected {
    let reason = match error {
        InputError::OutOfWindow { .. } => InputRejectReason::OutOfWindow,
        InputError::GapTooLarge { .. } => InputRejectReason::GapTooLarge,
        InputError::RateLimited { .. } => InputRejectReason::RateLimited,
        _ => InputRejectReason::BadSequence,
    };
    InputRejected {
        round_id: batch.round_id,
        reason,
        applied_seq,
        first_seq: batch.commands.first().map_or(0, |command| command.seq),
        command_count: u8::try_from(batch.commands.len()).unwrap_or(u8::MAX),
        server_tick,
    }
}
