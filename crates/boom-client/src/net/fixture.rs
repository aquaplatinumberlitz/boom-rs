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
//! theo seed trên mọi OS. Vì vậy phần auth cũng làm thủ công ở đây, nhưng **có thật**: server
//! giả dùng `boom_server::auth::AuthService` + SQLite tempfile (Argon2id + DB thật, ADR 0011
//! §5–§6, §10), nên client phải `Auth` sau `Welcome` và nhận `AuthOk` như với server thật.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{
    CharacterStats, CharacterStatsTable, Dir, PlayerInput, Rules, Stat, World, mode_by_id,
};
use boom_net::{NetError, Transport};
use boom_proto::dto::auth::{Auth, AuthOk};
use boom_proto::{
    Hello, InputBatch, InputRejectReason, InputRejected, MessageKind, ProtocolLimits,
    RoomCharacter, RoundEvent, RoundMember, RoundStart, Welcome, decode_frame, encode_frame,
    read_envelope,
};
use boom_room::{CharacterId, PlayerId, RequestId, RoomAction, RoomCommand};
use boom_server::auth::{AuthConfig, AuthService};
use boom_server::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use boom_server::{Authority, HandshakeConfig, InputError, MatchSetup, Server, ServerConfig};

use super::{ClientSession, Credentials, NetClient, hello_for_profile};

/// Rào reader: lần nhận kế tiếp chỉ bắt đầu sau khi frame trước đã được enqueue.
/// Dùng cho reader thread của cả client và server, không thay đổi ClientSession.
pub struct EnqueuedReader<T> {
    inner: T,
    progress: ReaderProgress,
    pending: bool,
}

#[derive(Clone, Default)]
pub struct ReaderProgress(Arc<std::sync::atomic::AtomicU64>);

impl ReaderProgress {
    pub fn wait_for(&self, frames: u64) {
        for _ in 0..MAX_WAIT_ROUNDS {
            if self.0.load(std::sync::atomic::Ordering::Acquire) >= frames {
                return;
            }
            thread::yield_now();
        }
        panic!("reader chưa enqueue đủ {frames} frame");
    }
}

impl<T: Transport> EnqueuedReader<T> {
    pub fn new(inner: T) -> (Self, ReaderProgress) {
        let progress = ReaderProgress::default();
        (
            Self {
                inner,
                progress: progress.clone(),
                pending: false,
            },
            progress,
        )
    }
}

impl<T: Transport> Transport for EnqueuedReader<T> {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        self.inner.send_frame(frame)
    }
    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        if self.pending {
            self.progress
                .0
                .fetch_add(1, std::sync::atomic::Ordering::Release);
        }
        let frame = self.inner.recv_frame()?;
        self.pending = frame.is_some();
        Ok(frame)
    }
    fn close(&mut self) -> Result<(), NetError> {
        self.inner.close()
    }
    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

/// Writer dùng chung để test gửi marker Ping mà không thêm API production.
pub struct SharedWriter<T>(Arc<Mutex<T>>);
impl<T> Clone for SharedWriter<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T> SharedWriter<T> {
    pub fn new(inner: T) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }
}
impl<T: Transport> Transport for SharedWriter<T> {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        self.0.lock().expect("writer").send_frame(frame)
    }
    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        self.0.lock().expect("writer").recv_frame()
    }
    fn close(&mut self) -> Result<(), NetError> {
        self.0.lock().expect("writer").close()
    }
    fn is_closed(&self) -> bool {
        self.0.lock().expect("writer").is_closed()
    }
}

/// Phản hồi Pong(u64::MAX) làm marker FIFO cho writer thread của server.
/// Marker chỉ phục vụ rào test, không đi qua fault PRNG hay tới client.
pub struct FlushWriter<T> {
    inner: T,
    progress: ReaderProgress,
}
impl<T: Transport> FlushWriter<T> {
    pub fn new(inner: T) -> (Self, ReaderProgress) {
        let progress = ReaderProgress::default();
        (
            Self {
                inner,
                progress: progress.clone(),
            },
            progress,
        )
    }
}
impl ReaderProgress {
    pub fn count(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::Acquire)
    }
}
impl<T: Transport> Transport for FlushWriter<T> {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        if read_envelope(frame).expect("envelope").0.kind == MessageKind::Pong
            && decode_frame::<boom_proto::Pong>(frame, &ProtocolLimits::v1())
                .expect("pong")
                .0
                == u64::MAX
        {
            self.progress
                .0
                .fetch_add(1, std::sync::atomic::Ordering::Release);
            Ok(())
        } else {
            self.inner.send_frame(frame)
        }
    }
    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        self.inner.recv_frame()
    }
    fn close(&mut self) -> Result<(), NetError> {
        self.inner.close()
    }
    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

/// Profile rules cố định cho test (khác `data/rules/v1-dev.toml` nhưng cùng schema).
const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

/// Mật khẩu tài khoản thật trong fixture (đủ 8 ký tự — ADR 0011 §4).
pub const ACCOUNT_PASSWORD: &str = "matkhau123";

/// Trần **số vòng** chờ trong fixture (bơm client / poll server).
///
/// Trần là số vòng, **không phải thời gian thật**: tốc độ máy và độ phân giải timer của hệ
/// điều hành không được quyết định kết quả test. Vòng chờ **không tiến đồng hồ ảo** nên độ
/// trễ thực tế của luồng đọc không làm lệch kịch bản.
const MAX_WAIT_ROUNDS: u32 = 200_000;

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
    drop_results: bool,
    /// Số frame **đã tới hàng đợi nhận** của endpoint (đã đến hạn giao, chờ luồng đọc lấy),
    /// tính theo endpoint **nhận**.
    ///
    /// Khác [`Self::sent`]: frame còn trên đường (chưa đến hạn giao) **không** tính ở đây.
    queued: [u64; 2],
    /// Số frame reader đã enqueue vào channel ứng dụng, tính theo endpoint nhận.
    /// recv_frame kế tiếp xác nhận enqueue; try_recv đồng bộ xác nhận ngay khi lấy frame.
    delivered: [u64; 2],
}

/// Mạng ảo có lịch giao theo thời gian server.
#[derive(Default)]
pub struct ScheduledNet {
    inner: Arc<Mutex<Shared>>,
    signal: Condvar,
}

/// Bộ đếm frame của **chiều nhận** của mạng ảo.
///
/// Cho phép test biết chắc khi nào frame đã **tới nơi** mà không đoán theo hạn thời gian
/// thật: mỗi frame đưa lên đường sẽ được đối phương lấy **đúng một lần**.
#[derive(Clone)]
pub struct ScheduledCounters {
    inner: Arc<Mutex<Shared>>,
    /// Index endpoint **nhận** trong cặp.
    receiver: usize,
}

impl ScheduledCounters {
    /// Số frame reader đã enqueue (hoặc fixture đã nhận đồng bộ qua try_recv).
    pub fn delivered_frames(&self) -> u64 {
        let inner = self.inner.lock().expect("khoá mạng ảo");
        inner.delivered[self.receiver]
    }

    /// Số frame chiều này **đã tới hàng đợi nhận** (đến hạn giao) nhưng luồng đọc chưa lấy.
    ///
    /// Frame còn trên đường (chưa đến hạn giao theo đồng hồ mạng) **không** tính ở đây.
    /// Nhờ vậy test biết chính xác khi nào còn frame **đang chờ luồng đọc** (khác với frame
    /// còn trên đường) mà không phải đoán theo thời gian thật.
    pub fn queued_frames(&self) -> u64 {
        let inner = self.inner.lock().expect("khoá mạng ảo");
        inner.queued[self.receiver]
    }
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
            // Frame đã tới hàng đợi nhận: luồng đọc chưa lấy ⇒ còn frame đang chờ.
            inner.queued[index] += 1;
        }
        drop(inner);
        self.signal.notify_all();
    }

    /// Thống kê lỗi mạng đã xảy ra.
    pub fn stats(&self) -> NetStats {
        self.inner.lock().expect("khoá mạng ảo").stats
    }

    /// Bộ đếm frame của chiều **nhận** ở endpoint `receiver` (index endpoint trong cặp).
    pub fn counters(&self, receiver: usize) -> ScheduledCounters {
        ScheduledCounters {
            inner: Arc::clone(&self.inner),
            receiver,
        }
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
    pending: bool,
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
            pending: false,
        };
        let b = Self {
            net,
            endpoint: Endpoint::B,
            rng: Rng::new(seed.wrapping_add(0x3C6E_F372_FE94_F82B)),
            closed: false,
            pending: false,
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
        let frame = inner.ready[self.endpoint.index()].pop_front();
        if frame.is_some() {
            // Frame đã tới nơi: đếm giống `recv_frame` để bộ đếm nhất quán.
            inner.delivered[self.endpoint.index()] += 1;
        }
        frame
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
        let (faults, drop_results) = {
            let mut inner = self.net.inner.lock().expect("khoá mạng ảo");
            let drop_result = inner.drop_results
                && read_envelope(frame).expect("envelope").0.kind == MessageKind::RoundResult;
            if drop_result {
                inner.drop_results = false;
            }
            (inner.faults, drop_result)
        };
        // Quyết định lỗi mạng trước khi khoá mạng (PRNG thuộc endpoint này); thứ tự gọi PRNG
        // cố định nên cùng seed cho cùng thứ tự gửi ⇒ cùng lỗi mạng trên mọi OS.
        let lost = self.roll(faults.loss_permille)
            || (drop_results
                && read_envelope(frame).expect("envelope").0.kind == MessageKind::RoundResult);
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
        if self.pending {
            inner.delivered[self.endpoint.index()] += 1;
            self.pending = false;
        }
        loop {
            if let Some(frame) = inner.ready[self.endpoint.index()].pop_front() {
                // Chỉ xác nhận ở recv_frame kế tiếp, sau tx.send của reader thread.
                self.pending = true;
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

/// Thời gian một lượt (ms) từ rules profile (`round.duration_ms` qua `round_ticks`).
fn round_duration_ms(rules: &Rules) -> u64 {
    let ticks = u64::from(rules.round_ticks);
    let hz = u64::from(rules.ticks_per_second).max(1);
    ticks.saturating_mul(1_000) / hz
}

/// Một client trong fixture.
pub struct ScheduledClient {
    /// Client mạng (đọc snapshot, gửi input).
    pub net: NetClient<ScheduledTransport>,
    /// Định danh người chơi trong phòng/round (= `AuthOk.account_id`).
    ///
    /// Server bind mọi trường `player` trên dây từ giá trị này
    /// (ADR 0009 Addendum B §B.2).
    pub player: PlayerId,
    /// Slot trận của người chơi (id trong state).
    ///
    /// Lấy từ `RoundStart.roster` theo `account_id` (xem [`ScheduledHarness::tick`]).
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

impl ScheduledClient {
    /// Bộ đếm frame chiều **server → client**.
    ///
    /// Trong `ScheduledTransport::pair` endpoint `A` là phía **gửi** đầu tiên, `B` là phía
    /// nhận; chiều server→client nên server gửi (`A`) và client nhận (`B`).
    pub fn in_counters(&self) -> ScheduledCounters {
        self.in_net.counters(1)
    }
}

/// Fixture: server authority + hai client, lịch giao theo tick của server.
pub struct ScheduledHarness {
    authority: Authority,
    clients: Vec<ScheduledClient>,
    /// Dịch vụ auth thật của server (đăng ký/đăng nhập Argon2id, ADR 0011 §5).
    auth: AuthService,
    /// Store SQLite thật trên file tạm (ADR 0011 §10).
    store: SqliteAccountStore,
    /// Thư mục tạm chứa DB: phải sống suốt đời fixture thì file DB còn tồn tại.
    _db_dir: tempfile::TempDir,
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
    /// Tên profile rules mà client khai ở handshake (đưa vào `RoundStart`).
    profile_id: String,
    /// Hash rules mà client khai ở handshake (đưa vào `RoundStart`).
    rules_hash: u64,
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
    /// Lỗi mạng đã cấu hình cho chiều client→server / server→client.
    ///
    /// Fixture **biết trước** cấu hình mạng của chính nó; dùng để điều khiển target_tick
    /// một cách tường minh trong test (Codex review #226: "phải điều khiển mốc/delay tường
    /// minh"), không phải nguồn của client sản phẩm.
    out_faults: ScheduledFaults,
    in_faults: ScheduledFaults,
    /// Gốc đồng hồ test dùng cho input clock của mọi client.
    clock_origin: Instant,
    /// Sequence bị authority retire, kèm round và người chơi.
    ///
    /// Lấy từ `InternalServerEvent::InputRetired` do `Authority::advance_to` trả về, không
    /// suy từ event trên dây: dùng để chứng minh producer không tạo lệnh quá hạn.
    retired_commands: Vec<(u64, PlayerId, u32)>,
}

impl ScheduledHarness {
    /// Dựng fixture: hai client handshake vào server (authority) qua mạng có lịch giao.
    ///
    /// `out_faults` áp cho chiều client→server (khung input), `in_faults` cho chiều
    /// server→client (snapshot/event).
    ///
    /// Mỗi client **đăng nhập thật** (Argon2id + SQLite tempfile) trước khi vào trận: server
    /// chặn mọi kind trước `AuthOk` (ADR 0009 Addendum A §A.1) nên fixture phải trải đúng
    /// bước `Welcome` → `Auth` → `AuthOk`.
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
        // Auth thật: SQLite trên file tạm + `AuthService` của server (ADR 0011 §5–§6, §10).
        let db_dir = tempfile::tempdir().expect("tempdir");
        let mut store = SqliteAccountStore::open(
            &db_dir.path().join("auth.sqlite3"),
            SqliteOptions::default(),
        )
        .expect("mở store");
        let mut auth = AuthService::new(
            AuthConfig::default(),
            "fixture-instance",
            Box::new(OsRandomSource),
        )
        .expect("dịch vụ auth");
        let mut clients = Vec::new();
        let hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
        for index in 0..2 {
            let slot = u8::try_from(index).expect("slot");
            // Hai chiều tách riêng: client→server và server→client.
            let (client_writer, mut server_reader) =
                ScheduledTransport::pair(seed.wrapping_add(index as u64));
            let (mut server_writer, client_reader) =
                ScheduledTransport::pair(seed.wrapping_add(index as u64));
            let out_net = server_reader.net();
            let in_net = server_writer.net();
            // Handshake: client gửi `Hello`, fixture trả `Welcome` (như server thật), rồi
            // client `Auth` và fixture trả `AuthOk` (đăng nhập thật).
            let client_rules = rules.clone();
            let client_hello = hello.clone();
            let credentials =
                Credentials::register(format!("fixture-{index}"), ACCOUNT_PASSWORD.to_owned());
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
                server_version: "boom-server-fixture".to_owned(),
                rules_profile_id: profile.profile_id.clone(),
                rules_hash: rules.profile_hash,
                content_sha256: [0; 32],
                content_epoch: 1,
                snapshot_interval_ms: SNAPSHOT_INTERVAL_MS as u32,
                protocol_limits_version: ProtocolLimits::v1().revision,
            };
            server_writer
                .send_frame(&encode_frame(&welcome, &ProtocolLimits::v1()).expect("encode welcome"))
                .expect("gửi welcome");
            in_net.advance_to(0);
            // Chờ `Auth` của client rồi đăng ký tài khoản thật (Argon2id + SQLite).
            let frame = loop {
                out_net.advance_to(0);
                if let Some(frame) = server_reader.try_recv() {
                    break frame;
                }
                thread::sleep(Duration::from_millis(1));
            };
            let auth_in: Auth = decode_frame(&frame, &ProtocolLimits::v1()).expect("decode auth");
            // Đăng ký tài khoản thật (Argon2id + SQLite) rồi trả `AuthOk`: `account_id` cấp
            // bởi store là **định danh người chơi trong phòng/round** mà server ghi vào mọi
            // trường `player` trên dây (ADR 0009 Addendum B §B.2).
            let outcome = auth
                .register(
                    &mut store,
                    &auth_in.username,
                    &auth_in.password,
                    "127.0.0.1",
                    &format!("fixture-{index}"),
                    0,
                )
                .expect("đăng ký tài khoản");
            let success = outcome.success();
            let auth_ok = AuthOk {
                account_id: u64::try_from(success.account_id.0).expect("account id"),
                display_name: success.display_name.clone(),
                token: success.token.expose().clone(),
                expires_at_ms: u64::try_from(success.expires_at_ms).unwrap_or(u64::MAX),
            };
            // Định danh người chơi trong phòng/round = `account_id` (server bind từ phiên).
            let player = PlayerId(auth_ok.account_id);
            server_writer
                .send_frame(&encode_frame(&auth_ok, &ProtocolLimits::v1()).expect("encode auth ok"))
                .expect("gửi auth ok");
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
            auth,
            store,
            _db_dir: db_dir,
            hashes: BTreeMap::new(),
            server_positions: BTreeMap::new(),
            predicted_positions: BTreeMap::new(),
            start_positions: BTreeMap::new(),
            origin_ms: 0,
            profile_id: profile.profile_id.clone(),
            rules_hash: rules.profile_hash,
            ticks: 0,
            hz,
            snapshot_seq: 0,
            last_snapshot_ms: 0,
            input_batches: 0,
            duplicate_commands: 0,
            mismatches: 0,
            corrections: 0,
            clock_origin: Instant::now(),
            rejects: 0,
            last_reject: None,
            out_faults,
            in_faults,
            retired_commands: Vec::new(),
        }
    }

    /// Dựng phòng, ready, start và qua loading barrier: lượt bắt đầu ở tick 0.
    pub fn start_round(&mut self) {
        // Định danh người chơi trong phòng/round = `account_id` (server bind từ phiên),
        // nên lấy đúng từ client đã đăng nhập.
        let host = self.clients[0].player;
        let guest = self.clients[1].player;
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
                    player: guest,
                    request: RequestId(2),
                    action: RoomAction::Join {
                        room,
                        character: CharacterId::C01,
                    },
                },
                now,
            )
            .expect("join");
        for (index, player) in [host, guest].into_iter().enumerate() {
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
        // Gửi `RoundStart` cho mọi client **trước** khi qua loading barrier (server thật gửi
        // `RoundStart` rồi mới chờ `LoadingAck`): client lấy slot trận của mình từ roster
        // theo `account_id` (ADR 0009 Addendum B §B.2).
        self.send_round_start();
        for player in [host, guest] {
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

    /// Gửi `RoundStart` (giống server thật) cho mọi client trong fixture.
    ///
    /// Roster lấy từ phòng của authority: `player` là **định danh người chơi** (`account_id`),
    /// `slot` là slot trận do server bind.
    fn send_round_start(&mut self) {
        let view = self.authority.room_view().expect("có phòng");
        let round = view.round.as_ref().expect("đã start");
        let roster: Vec<RoundMember> = round
            .members
            .iter()
            .map(|member| RoundMember {
                player: member.player.0,
                slot: member.slot.0,
                character: match member.character {
                    CharacterId::C00 => RoomCharacter::C00,
                    CharacterId::C01 => RoomCharacter::C01,
                },
                // Tên hiển thị: fixture chạy `Authority` trực tiếp nên không có account store;
                // điền tên giả để roster có đủ trường (server thật điền tên từ account store —
                // ADR 0009 Addendum B §B.6).
                name: format!("fixture-{}", member.player.0),
            })
            .collect();
        let start = RoundStart {
            round_id: round.id.0,
            seed: self.authority.round_seed().expect("có seed"),
            rules_profile_id: self.profile_id.clone(),
            rules_hash: self.rules_hash,
            content_sha256: [0; 32],
            map: "skeleton-test".to_owned(),
            mode: "battle_ffa".to_owned(),
            roster,
            start_tick: self.authority.tick().unwrap_or(0),
        };
        let limits = ProtocolLimits::v1();
        let frame = encode_frame(&start, &limits).expect("encode round start");
        for client in &mut self.clients {
            client
                .in_server
                .send_frame(&frame)
                .expect("gửi round start");
        }
    }

    /// Một bước: giao frame đến hạn, client nhận snapshot rồi dự đoán, server nhận input và
    /// chạy tick, cuối cùng server gửi snapshot/event.
    pub fn tick(&mut self, dirs: [Dir; 2], bombs: [bool; 2]) {
        let now_ms = self.next_ms();
        self.set_client_clocks(now_ms);
        // 1. Giao các frame đã đến hạn (snapshot/event của tick trước, input tới trễ).
        self.advance_network(now_ms);
        // 2. Client nhận message: chờ tới khi **mọi** frame server gửi đã được client xử lý
        //    (điều kiện theo bộ đếm frame; đồng hồ ảo đứng yên ⇒ độ trễ thực tế của luồng
        //    đọc không làm lệch kịch bản).
        let before: Vec<(usize, u32)> = self
            .clients
            .iter()
            .map(|client| {
                (
                    client.net.pending_len(),
                    client.net.confirmed_state().map_or(0, |state| state.tick),
                )
            })
            .collect();
        self.pump_clients();
        for (index, client) in self.clients.iter_mut().enumerate() {
            let (pending_before, confirmed_before) = before[index];
            // Slot trận của client: lấy từ `RoundStart.roster` theo `account_id` (client tự
            // suy ra từ roster, không phải fixture gán sẵn — ADR 0009 Addendum B §B.2).
            if let Some(slot) = client.net.player_slot() {
                client.slot = slot;
            }
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
        let retired = self.authority.advance_to(now_ms).expect("server chạy tick");
        for event in retired {
            if let boom_server::InternalServerEvent::InputRetired {
                round, player, seq, ..
            } = event
            {
                self.retired_commands.push((round.0, player, seq));
            }
        }
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
            self.set_client_clocks(now_ms);
            self.advance_network(now_ms);
            // Chờ tới khi mọi frame server gửi đã được client xử lý (đồng hồ ảo đứng yên).
            self.pump_clients();
            for client in &mut self.clients {
                // Slot trận của client lấy từ roster (xem [`ScheduledHarness::tick`]).
                if let Some(slot) = client.net.player_slot() {
                    client.slot = slot;
                }
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

    /// Đăng nhập lại tài khoản của client `index` bằng mật khẩu đã dùng.
    ///
    /// Fixture đăng ký tài khoản **thật** nên tài khoản đã có phiên sống ⇒ đăng nhập lại phải bị
    /// từ chối `AccountHasActiveSession` (ADR 0011 §6). Kết quả này chứng minh store SQLite thật
    /// đang lưu phiên (không phải store giả trong RAM).
    pub fn login_again(&mut self, index: usize) -> Result<String, boom_server::auth::AuthError> {
        let username = format!("fixture-{index}");
        let outcome = self.auth.login(
            &mut self.store,
            &username,
            ACCOUNT_PASSWORD,
            "127.0.0.1",
            "fixture-login-again",
            0,
        )?;
        Ok(outcome.success().display_name.clone())
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

    /// Cấu hình trễ đã dùng cho chiều client→server (test kiểm tra fixture tường minh).
    pub fn out_faults(&self) -> ScheduledFaults {
        self.out_faults
    }

    /// Cấu hình trễ đã dùng cho chiều server→client.
    pub fn in_faults(&self) -> ScheduledFaults {
        self.in_faults
    }

    /// Tick server hiện tại.
    pub fn server_tick(&self) -> Option<u32> {
        self.authority.tick()
    }

    /// Số bom trong snapshot hiện tại của authority (0 nếu chưa có snapshot).
    pub fn server_bomb_count(&self) -> usize {
        self.authority
            .snapshot(0, 1)
            .and_then(Result::ok)
            .map_or(0, |snapshot| snapshot.bombs.len())
    }

    /// Các sequence `InputRetired` đã thấy từ authority, kèm round và player.
    pub fn retired_commands(&self) -> &[(u64, PlayerId, u32)] {
        &self.retired_commands
    }

    /// Đặt target tick tường minh cho client (chỉ fixture test dùng).
    pub fn set_target_override(&mut self, index: usize, target: u32) {
        self.clients[index]
            .net
            .set_input_target_override(Some(target));
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
    /// Đưa toàn bộ input clock của client về cùng mốc ảo của fixture.
    fn set_client_clocks(&mut self, now_ms: u64) {
        let now = self.clock_origin + Duration::from_millis(now_ms);
        for client in &mut self.clients {
            client.net.set_input_clock(now);
        }
    }

    fn next_ms(&mut self) -> u64 {
        let now = self.origin_ms + (self.ticks * 1_000) / u64::from(self.hz);
        self.ticks += 1;
        now
    }

    /// Bơm hàng đợi đọc của mọi client tới khi **mọi frame server gửi** đã được xử lý.
    ///
    /// Điều kiện dựa trên **bộ đếm frame** của mạng ảo, không đo thời gian thật. Vòng chờ
    /// **không tiến đồng hồ ảo** nên độ trễ thực tế của luồng đọc client không làm lệch
    /// kịch bản.
    ///
    /// Chờ reader enqueue đủ frame đến hạn rồi drain một lần với Duration::ZERO.
    fn pump_clients(&mut self) {
        for client in &mut self.clients {
            let counters = client.in_counters();
            let target = counters.queued_frames();
            let mut ready = false;
            for _ in 0..MAX_WAIT_ROUNDS {
                if counters.delivered_frames() >= target {
                    ready = true;
                    break;
                }
                thread::yield_now();
            }
            assert!(ready, "client chưa enqueue đủ {target} frame");
            client.net.pump(Duration::ZERO).expect("bơm message");
            client.events.extend(client.net.take_events());
        }
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
                // Client gửi nhiều kind trên cùng kênh (kể cả `LoadingAck` sau
                // `RoundStart`): chỉ `InputBatch` mới là input trận, kind khác bỏ qua.
                let (envelope, _) = read_envelope(&frame).expect("đọc envelope");
                if envelope.kind != MessageKind::InputBatch {
                    continue;
                }
                let batch: InputBatch = decode_frame(&frame, &limits).expect("decode input batch");
                self.input_batches += 1;
                for command in &batch.commands {
                    if !client.sent_seqs.insert(command.seq) {
                        self.duplicate_commands += 1;
                    }
                }
                match self.authority.submit_input(client.player, &batch, now_ms) {
                    Ok(ack) => {
                        let round = u64::from(self.authority.current_round_id().unwrap_or(0));
                        for seq in ack.retired {
                            self.retired_commands.push((round, client.player, seq));
                        }
                    }
                    Err(error) => {
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

// --- Fixture phòng end-to-end: server thật + nhiều client qua mạng có lịch giao. ---

/// Tên map của fixture (server gửi trong `RoundStart.map`).
pub const ROOM_MAP_ID: &str = "skeleton-test";

/// Bước đồng hồ ảo (ms) của fixture phòng: 16 ms = một tick ở 60 Hz.
const ROOM_STEP_MS: u64 = 16;

/// IP nguồn giả của kết nối trong fixture phòng (bucket rate limit theo nguồn, ADR 0011 §5).
const ROOM_TEST_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Gốc định danh **kết nối** trong fixture phòng.
///
/// Fixture cố tình tách định danh kết nối khỏi định danh người chơi (`account_id`) để test
/// chứng minh client không lấy slot trận từ `Welcome.player_id`
/// (ADR 0009 Addendum B §B.2).
const CONNECTION_ID_BASE: u64 = 1_000;

/// Một client trong fixture phòng.
pub struct RoomClient {
    /// Client mạng (đọc snapshot, gửi input, điều khiển phòng).
    pub net: NetClient<ScheduledTransport>,
    /// Người chơi trên server = `AuthOk.account_id`.
    ///
    /// Đây là **định danh người chơi trong phòng/round**: mọi trường `player` trên dây
    /// (`RoomState`, `RoundStart.roster`, `RoundResult`) mang giá trị này
    /// (ADR 0009 Addendum B §B.2).
    pub player: PlayerId,
    /// Định danh **kết nối** trong phiên (khoá session phía server).
    ///
    /// **Không** phải slot trận: slot lấy từ `RoundStart.roster` theo `player`.
    pub connection: PlayerId,
    /// Mạng chiều client→server (điều khiển đồng hồ).
    out_net: Arc<ScheduledNet>,
    /// Mạng chiều server→client (điều khiển đồng hồ).
    in_net: Arc<ScheduledNet>,
    /// Event đã nhận (đã dedup theo `(round_id, EventId)`).
    events: Vec<RoundEvent>,
    writer_progress: ReaderProgress,
}

impl RoomClient {
    /// Thống kê riêng chiều server→client, tránh assert tổng che mất coverage chiều này.
    pub fn inbound_stats(&self) -> NetStats {
        self.in_net.stats()
    }

    /// Fault xác định cho regression: làm mất RoundResult kế tiếp của client này.
    pub fn drop_round_results(&self) {
        self.in_net.inner.lock().expect("khoá mạng ảo").drop_results = true;
    }

    /// Event đã nhận của client.
    pub fn events(&self) -> &[RoundEvent] {
        &self.events
    }

    /// Slot trận của client: mục `RoundStart.roster` có `player == account_id`.
    ///
    /// `None` = client chưa nhận `RoundStart` (chưa vào lượt).
    pub fn slot(&self) -> Option<u8> {
        self.net.player_slot()
    }

    /// Định danh kết nối trong phiên (`Welcome.player_id`).
    ///
    /// **Không** phải slot trận: slot lấy từ roster theo `account_id`
    /// (ADR 0009 Addendum B §B.2).
    pub fn connection_id(&self) -> Option<u8> {
        self.net.connection_id()
    }

    /// Bộ đếm frame chiều **server → client**.
    ///
    /// Trong `ScheduledTransport::pair` endpoint `A` là phía **gửi** đầu tiên, `B` là phía
    /// nhận; chiều server→client nên server gửi (`A`) và client nhận (`B`).
    pub fn in_counters(&self) -> ScheduledCounters {
        self.in_net.counters(1)
    }

    /// Bộ đếm frame chiều **client → server**.
    pub fn out_counters(&self) -> ScheduledCounters {
        self.out_net.counters(1)
    }
}

/// Fixture phòng end-to-end: server thật (`boom_server::Server`) + N client qua mạng có lịch
/// giao theo đồng hồ ảo.
///
/// Khác [`ScheduledHarness`] (chạy lõi `Authority` trực tiếp), fixture này chạy **vòng IO thật**
/// của server nên message sảnh/phòng/chat/loading/result đi đúng đường wire
/// (ADR 0009 Addendum B). Mỗi client **đăng nhập thật** (Argon2id + SQLite tempfile) nên phải
/// đi đúng bước `Welcome` → `Auth` → `AuthOk` trước khi dùng lệnh phòng.
pub struct RoomHarness {
    /// Server thật (vòng IO + authority + auth).
    server: Server,
    /// Client trong fixture.
    clients: Vec<RoomClient>,
    /// Đồng hồ ảo (ms): vừa là thời gian server vừa là mốc giao của mạng.
    now_ms: u64,
    /// Thời gian một lượt (ms), lấy từ rules profile (`round.duration_ms`).
    round_duration_ms: u64,
    /// Thư mục tạm chứa DB auth: phải sống suốt đời fixture thì file DB còn tồn tại.
    _db_dir: tempfile::TempDir,
}

impl RoomHarness {
    /// Dựng server + `count` client đã đăng nhập thật.
    ///
    /// `out_faults` áp cho chiều client→server, `in_faults` cho chiều server→client. Lỗi mạng
    /// **bật sau handshake**: mất `Hello`/`Welcome` là hỏng phiên chứ không phải lỗi mạng.
    pub fn new(
        count: usize,
        out_faults: ScheduledFaults,
        in_faults: ScheduledFaults,
        seed: u64,
    ) -> Self {
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
                map: ROOM_MAP_ID.to_owned(),
                replay: None,
            },
        );
        let handshake = HandshakeConfig {
            rules_profile_id: profile.profile_id.clone(),
            rules_hash: rules.profile_hash,
            content_sha256: [0; 32],
            content_epoch: 1,
            snapshot_interval_ms: SNAPSHOT_INTERVAL_MS as u32,
        };
        // Auth thật: SQLite trên file tạm + `AuthService` của server (ADR 0011 §5–§6, §10).
        let db_dir = tempfile::tempdir().expect("tempdir");
        let store = SqliteAccountStore::open(
            &db_dir.path().join("auth.sqlite3"),
            SqliteOptions::default(),
        )
        .expect("mở store");
        let auth = AuthService::new(
            AuthConfig::default(),
            "room-fixture-instance",
            Box::new(OsRandomSource),
        )
        .expect("dịch vụ auth");
        // Ping/liveness tắt (`u64::MAX`): fixture nhảy thời gian ảo theo bước lớn, không muốn
        // server đóng phiên chỉ vì client chưa kịp trả `Pong`.
        let mut server = Server::with_auth_and_timers(
            authority,
            handshake,
            auth,
            Box::new(store),
            0,
            u64::MAX,
            u64::MAX,
        )
        .expect("dựng server");
        // Hạn đăng nhập tắt: đồng hồ ảo của fixture chạy nhanh hơn thời gian thật (client
        // handshake ở luồng riêng), nên hạn theo đồng hồ ảo sẽ đóng phiên sớm khi máy chậm.
        server.set_auth_deadline_ms(u64::MAX);
        let hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
        let mut nets: Vec<(Arc<ScheduledNet>, Arc<ScheduledNet>)> = Vec::new();
        let mut handles = Vec::new();
        let mut identities = Vec::new();
        let mut writers = Vec::new();
        for index in 0..count {
            // Định danh **kết nối** trong phiên: server dùng làm khoá session, **không** phải
            // slot trận (ADR 0009 Addendum B §B.2).
            let connection = PlayerId(u64::try_from(index).expect("id") + CONNECTION_ID_BASE);
            // `Welcome.player_id` là định danh kết nối: fixture cố tình chọn giá trị **khác**
            // slot trận (server bind slot từ roster theo `account_id`) để chứng minh client
            // không lấy slot từ `Welcome`.
            let connection_id = u8::try_from(index).expect("id") + 1;
            // Hai chiều tách riêng: client→server và server→client (giống cách server chia).
            let (client_writer, server_reader) =
                ScheduledTransport::pair(seed.wrapping_add(index as u64));
            let (server_writer, client_reader) =
                ScheduledTransport::pair(seed.wrapping_add(index as u64));
            let out_net = client_writer.net();
            let in_net = client_reader.net();
            let (server_writer, progress) = FlushWriter::new(server_writer);
            writers.push(progress);
            nets.push((Arc::clone(&out_net), Arc::clone(&in_net)));
            server
                .accept(
                    connection,
                    connection_id,
                    ROOM_TEST_IP,
                    server_reader,
                    server_writer,
                )
                .expect("accept");
            let client_rules = rules.clone();
            let client_hello = hello.clone();
            let credentials =
                Credentials::register(format!("room-{index}"), ACCOUNT_PASSWORD.to_owned());
            handles.push(thread::spawn(move || {
                let mut net = NetClient::with_room_retry_interval(
                    ClientSession::new(client_reader, client_writer),
                    client_rules,
                    mode_by_id("battle_ffa").expect("mode"),
                    // Gửi lại lệnh phòng **mỗi** vòng bơm: mạng ảo chạy theo đồng hồ ảo nên nhịp
                    // gửi lại không được phụ thuộc thời gian thật (nếu không, test chậm máy sẽ đỏ).
                    Duration::ZERO,
                );
                net.handshake(&client_hello).expect("handshake");
                net.authenticate(&credentials).expect("auth");
                net
            }));
            identities.push(connection);
        }
        // Handshake giữ clock tại 0; số lượt poll phụ thuộc scheduler không đổi origin.
        let now_ms = 0u64;
        while handles.iter().any(|handle| !handle.is_finished()) {
            for (out_net, in_net) in &nets {
                out_net.advance_to(now_ms);
                in_net.advance_to(now_ms);
            }
            let _ = server
                .poll(now_ms, Duration::from_millis(1))
                .expect("server poll lúc handshake");
        }
        let mut clients = Vec::new();
        for (index, handle) in handles.into_iter().enumerate() {
            let net = handle.join().expect("join luồng client");
            let connection = identities[index];
            let (out_net, in_net) = nets[index].clone();
            // Định danh người chơi trong phòng/round = `AuthOk.account_id`: server bind mọi
            // trường `player` trên dây từ giá trị này (ADR 0009 Addendum B §B.2).
            let account_id = net.account().expect("client đã đăng nhập").account_id;
            clients.push(RoomClient {
                net,
                player: PlayerId(account_id),
                connection,
                out_net,
                in_net,
                events: Vec::new(),
                writer_progress: writers[index].clone(),
            });
        }
        // Lỗi mạng chỉ bật **sau** handshake: mất `Hello`/`Welcome` là hỏng phiên chứ không
        // phải lỗi mạng. Cùng bộ lỗi áp cho mọi client và cả hai chiều.
        for (out_net, in_net) in &nets {
            out_net.set_faults(out_faults);
            in_net.set_faults(in_faults);
        }
        Self {
            server,
            clients,
            now_ms,
            round_duration_ms: round_duration_ms(&rules),
            _db_dir: db_dir,
        }
    }

    /// Số client trong fixture.
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Client thứ `index`.
    pub fn client(&self, index: usize) -> &RoomClient {
        &self.clients[index]
    }

    /// Client thứ `index` (mutable).
    pub fn client_mut(&mut self, index: usize) -> &mut RoomClient {
        &mut self.clients[index]
    }

    /// Đồng hồ ảo hiện tại (ms).
    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }

    /// Tick hiện tại của server (authority).
    pub fn server_tick(&self) -> Option<u32> {
        self.server.authority().tick()
    }

    /// Thời gian một lượt (ms), lấy từ rules profile.
    pub fn round_duration_ms(&self) -> u64 {
        self.round_duration_ms
    }

    /// Bật luồng tự động (`--auto-ready`) cho client thứ `index`: client tự xin danh sách phòng,
    /// vào phòng, ready, start (nếu là host) và ack loading/kết quả.
    ///
    /// `min_players` là số người tối thiểu trong phòng để host start (tham số kịch bản QA).
    pub fn enable_auto(&mut self, index: usize, min_players: usize) {
        self.clients[index]
            .net
            .enable_auto_flow(boom_proto::RoomCharacter::C00, min_players)
            .expect("bật luồng tự động");
    }

    /// Bơm mạng `steps` bước: giao frame đến hạn, server xử lý, client nhận message.
    ///
    /// Mỗi bước **đóng** trước khi đồng hồ ảo tiến: server đã xử lý hết frame client gửi
    /// và client đã xử lý hết frame server gửi. Nhờ vậy độ trễ thực tế của luồng đọc
    /// (luồng đọc client chạy ở thread riêng) **không** làm lệch kịch bản: cùng seed cho
    /// cùng kết quả trên mọi OS và mọi tốc độ máy.
    pub fn pump(&mut self, steps: u32) {
        for _ in 0..steps {
            self.now_ms += ROOM_STEP_MS;
            self.advance_network();
            self.poll_server();
            self.pump_clients();
        }
    }

    /// Nhảy đồng hồ tới `now_ms`: giao frame đến hạn và server chạy tới mốc mới.
    ///
    /// Dùng để chạy hết thời gian lượt mà không phải bơm từng tick: mọi tick đến hạn được
    /// server chạy trong một lần `poll` (đồng hồ ảo, không phụ thuộc thời gian thực).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = self.now_ms.max(now_ms);
        self.advance_network();
        self.poll_server();
    }

    /// Bơm tới khi điều kiện của client `index` thành hiện thực; trả về kết quả điều kiện.
    pub fn wait_for(
        &mut self,
        index: usize,
        condition: impl Fn(&RoomClient) -> bool,
        steps: u32,
    ) -> bool {
        for _ in 0..steps {
            if condition(&self.clients[index]) {
                return true;
            }
            self.pump(1);
        }
        condition(&self.clients[index])
    }

    /// Bơm tới khi **mọi** client thỏa điều kiện; trả về kết quả điều kiện.
    pub fn wait_for_all(&mut self, condition: impl Fn(&RoomClient) -> bool, steps: u32) -> bool {
        for _ in 0..steps {
            if self.clients.iter().all(&condition) {
                return true;
            }
            self.pump(1);
        }
        self.clients.iter().all(&condition)
    }

    /// Client đã vào phòng và có `RoomState` chưa.
    pub fn in_room(&self, index: usize) -> bool {
        self.clients[index].net.room().room().is_some()
    }

    /// Thống kê lỗi mạng đã xảy ra trên **cả hai chiều** của mọi client.
    ///
    /// Test dùng để chứng minh lỗi mạng **thật sự xảy ra** (fixture không rỗng): mất, trễ,
    /// trùng và đảo thứ tự đều phải được quan sát thấy.
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

    /// Client đã sẵn sàng (ready) trong phòng chưa.
    pub fn is_ready(&self, index: usize) -> bool {
        self.clients[index]
            .net
            .room()
            .my_member()
            .is_some_and(|member| member.ready)
    }

    /// Giao frame đến hạn trên mọi chiều.
    fn advance_network(&self) {
        for client in &self.clients {
            client.out_net.advance_to(self.now_ms);
            client.in_net.advance_to(self.now_ms);
        }
    }

    /// Chờ server **xử lý hết** frame client đã gửi, rồi trả về.
    ///
    /// Điều kiện dựa trên **bộ đếm frame** của mạng ảo, không đo thời gian thật. Vòng chờ
    /// **không tiến đồng hồ ảo** nên độ trễ thực tế của luồng đọc không làm lệch kịch bản.
    ///
    /// Reader xác nhận enqueue; marker Ping/Pong ở poll kế tiếp rào writer FIFO.
    fn poll_server(&mut self) {
        for client in &self.clients {
            let counters = client.out_counters();
            let target = counters.queued_frames();
            let mut ready = false;
            for _ in 0..MAX_WAIT_ROUNDS {
                if counters.delivered_frames() >= target {
                    ready = true;
                    break;
                }
                thread::yield_now();
            }
            assert!(ready, "server chưa enqueue đủ {target} frame");
        }
        self.server
            .poll(self.now_ms, Duration::ZERO)
            .expect("server poll");
        // Poll trước đã xếp toàn bộ output. Marker ở poll kế tiếp đứng SAU output đó.
        for client in &self.clients {
            let mut inner = client.out_net.inner.lock().expect("khoá mạng ảo");
            inner.ready[1].push_back(
                encode_frame(&boom_proto::Ping(u64::MAX), &ProtocolLimits::v1()).expect("marker"),
            );
            inner.queued[1] += 1;
            drop(inner);
            client.out_net.signal.notify_all();
        }
        for client in &self.clients {
            let counters = client.out_counters();
            let target = counters.queued_frames();
            let mut ready = false;
            for _ in 0..MAX_WAIT_ROUNDS {
                if counters.delivered_frames() >= target {
                    ready = true;
                    break;
                }
                thread::yield_now();
            }
            assert!(ready, "server chưa enqueue marker");
        }
        let targets: Vec<_> = self
            .clients
            .iter()
            .map(|c| c.writer_progress.count() + 1)
            .collect();
        self.server
            .poll(self.now_ms, Duration::ZERO)
            .expect("poll marker");
        for (client, target) in self.clients.iter().zip(targets) {
            client.writer_progress.wait_for(target);
        }
    }

    /// Bơm hàng đợi đọc của mọi client tới khi **mọi frame server gửi** đã được xử lý.
    ///
    /// Điều kiện dựa trên **bộ đếm frame** của mạng ảo, không đo thời gian thật. Vòng chờ
    /// **không tiến đồng hồ ảo** nên độ trễ thực tế của luồng đọc client không làm lệch
    /// kịch bản.
    ///
    /// Chờ reader enqueue đủ frame đến hạn rồi drain một lần với Duration::ZERO.
    fn pump_clients(&mut self) {
        for client in &mut self.clients {
            let counters = client.in_counters();
            let target = counters.queued_frames();
            let mut ready = false;
            for _ in 0..MAX_WAIT_ROUNDS {
                if counters.delivered_frames() >= target {
                    ready = true;
                    break;
                }
                thread::yield_now();
            }
            assert!(ready, "client chưa enqueue đủ {target} frame");
            client.net.pump(Duration::ZERO).expect("bơm message");
            client.events.extend(client.net.take_events());
        }
    }
}
