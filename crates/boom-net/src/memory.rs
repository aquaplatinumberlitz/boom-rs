//! Transport trong bộ nhớ: mô phỏng trễ/mất/trùng/đảo thứ tự **theo seed**, cho fixture mạng V1-027.
//!
//! Mô hình: hai endpoint chia sẻ một "mạng" ảo. Mỗi frame gửi đi được gán một *mốc giao* trên
//! **đồng hồ ảo** (milli giây ảo). Frame tới hạn mốc giao mới được giao cho ứng dụng nhận, theo
//! thứ tự `(mốc giao, số thứ tự gửi)` — tức là thứ tự mà mạng thật sẽ dùng với độ trễ khác nhau.
//!
//! - **Trễ**: `delay_ms` cố định + ngẫu nhiên trong `[0, jitter_ms]`.
//! - **Mất**: `loss_permille` — frame bị mạng nuốt, ứng dụng gửi vẫn thành công (giống mạng thật).
//! - **Trùng**: `duplicate_permille` — frame được gửi lần hai, tới hơn bản gốc 1 ms.
//! - **Đảo thứ tự**: `reorder_permille` + `reorder_window_ms` — frame bị trễ thêm một khoảng
//!   ngẫu nhiên trong `[1, reorder_window_ms]`, nên có thể tới sau các frame gửi sau nó.
//!
//! Mọi quyết định lỗi mạng lấy từ PRNG theo seed ⇒ **cùng seed và cùng lệnh gửi cho cùng thứ tự
//! giao** trên mọi OS (số học 64 bit nguyên, không float, không đồng hồ thật).
//!
//! [`Timing::Virtual`] (mặc định) không ngủ thật: trễ chỉ là số ms ảo. [`Timing::Real`] ngủ đúng
//! số ms ảo để đo RTT thật, nhưng thứ tự giao vẫn tất định.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::error::NetError;
use crate::frame::FrameLimits;
use crate::rng::Rng;
use crate::transport::Transport;

/// Một nghìn: mức tỉ lệ tính bằng số nguyên để không phụ thuộc float.
const PERMILLE: u32 = 1000;

/// Cách đo trễ của transport trong bộ nhớ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Timing {
    /// Đồng hồ ảo (mặc định): không ngủ, tất định hoàn toàn.
    #[default]
    Virtual,
    /// Trễ thật: ngủ đúng số ms ảo trước khi giao frame.
    Real,
}

/// Lỗi mạng mô phỏng.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryFaults {
    /// Tỉ lệ mất frame (phần nghìn, 0..=1000).
    pub loss_permille: u16,
    /// Tỉ lệ gửi trùng frame (phần nghìn, 0..=1000).
    pub duplicate_permille: u16,
    /// Tỉ lệ đảo thứ tự (phần nghìn, 0..=1000).
    pub reorder_permille: u16,
    /// Trễ cố định của mọi frame (ms ảo).
    pub delay_ms: u32,
    /// Trễ ngẫu nhiên thêm trong `[0, jitter_ms]` (ms ảo).
    pub jitter_ms: u32,
    /// Cửa sổ đảo thứ tự: trễ thêm ngẫu nhiên trong `[1, reorder_window_ms]` (ms ảo).
    pub reorder_window_ms: u32,
}

impl MemoryFaults {
    /// Không mô phỏng lỗi nào: chỉ còn độ trễ khai báo.
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

/// Cấu hình transport trong bộ nhớ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryConfig {
    /// Seed quyết định lỗi mạng; cùng seed + cùng lệnh gửi ⇒ cùng thứ tự giao.
    pub seed: u64,
    /// Lỗi mạng mô phỏng.
    pub faults: MemoryFaults,
    /// Cách đo trễ.
    pub timing: Timing,
}

impl MemoryConfig {
    /// Cấu hình với seed, không mô phỏng lỗi.
    pub const fn seeded(seed: u64) -> Self {
        Self {
            seed,
            faults: MemoryFaults::none(),
            timing: Timing::Virtual,
        }
    }
}

/// Hai đầu của mạng ảo.
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

/// Một frame đang trên đường truyền ảo.
struct Packet {
    to: Endpoint,
    seq: u64,
    deliver_at: u64,
    payload: Vec<u8>,
}

/// Trạng thái chung của mạng ảo.
#[derive(Default)]
struct Shared {
    /// Đồng hồ ảo (ms).
    clock_ms: u64,
    /// Số thứ tự cấp cho frame gửi đi (dùng làm khoá phụ khi trùng mốc giao).
    next_seq: u64,
    /// Các frame đang trên đường.
    packets: Vec<Packet>,
    /// Endpoint nào đã đóng.
    closed: [bool; 2],
    /// Số frame đã gửi đi nhưng bị mạng nuốt, tính theo endpoint **gửi** (index = endpoint gửi).
    dropped: [u64; 2],
    /// Số frame đã **đưa lên đường** (kể cả bản trùng lặp), tính theo endpoint **gửi**.
    ///
    /// Frame bị mạng nuốt thì **không** tính: nó không bao giờ tới nơi.
    sent: [u64; 2],
    /// Số frame đã **giao cho ứng dụng nhận** (đã lấy khỏi hàng đợi), tính theo endpoint **nhận**.
    delivered: [u64; 2],
}

/// Mạng ảo dùng chung cho hai endpoint.
struct Network {
    inner: Mutex<Shared>,
    signal: Condvar,
}

/// Handle theo dõi bộ đếm của mạng ảo — **không** giữ hàng đợi nhận.
///
/// Cho phép test quan sát số frame đã đưa lên đường / đã tới nơi **từ nơi khác**: ví dụ
/// test nắm transport phía client nhưng cần biết luồng đọc của server đã lấy frame client
/// gửi chưa. Handle chỉ đọc bộ đếm dùng chung nên nhân bản được và dùng ở nhiều luồng.
#[derive(Clone)]
pub struct MemoryCounters {
    network: Arc<Network>,
    endpoint: Endpoint,
}

impl MemoryCounters {
    /// Số frame endpoint này đã **đưa lên đường** — không bị mạng nuốt, kể cả bản trùng lặp.
    ///
    /// Mỗi frame đưa lên đường sẽ được đối phương lấy **đúng một lần**, nên đối chiếu với
    /// [`Self::peer_delivered_frames`] cho biết chính xác còn frame nào đang trên đường.
    pub fn sent_frames(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").sent[self.endpoint.index()]
    }

    /// Số frame **đối phương** đã đưa lên đường (chiều ngược lại).
    pub fn peer_sent_frames(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").sent[self.endpoint.peer().index()]
    }

    /// Số frame endpoint này đã **nhận** (đã lấy khỏi hàng đợi và trả cho ứng dụng).
    ///
    /// Khác [`Self::dropped_frames`]: đây là frame *tới nơi*, không phải frame bị nuốt.
    pub fn delivered_frames(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").delivered[self.endpoint.index()]
    }

    /// Số frame **đối phương đã nhận** — tức frame endpoint này gửi đi đã tới nơi.
    ///
    /// Dùng để biết chắc đối phương (ví dụ luồng đọc của server) đã lấy frame khỏi mạng ảo.
    /// Không phụ thuộc đồng hồ thật nên test không đua với lịch chạy thread.
    pub fn peer_delivered_frames(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").delivered[self.endpoint.peer().index()]
    }

    /// Số frame endpoint này gửi đi đã bị mạng nuốt (mất).
    pub fn dropped_frames(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").dropped[self.endpoint.index()]
    }

    /// Số frame **đối phương** gửi đi đã bị mạng nuốt (mất) — chiều ngược lại.
    pub fn peer_dropped_frames(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").dropped[self.endpoint.peer().index()]
    }
}

/// Một đầu của transport trong bộ nhớ.
pub struct MemoryTransport {
    network: Arc<Network>,
    endpoint: Endpoint,
    limits: FrameLimits,
    faults: MemoryFaults,
    timing: Timing,
    rng: Rng,
    /// Frame đã tới hạn, chờ ứng dụng lấy.
    incoming: VecDeque<Vec<u8>>,
    /// Endpoint này đã đóng.
    closed: bool,
    /// Đã thấy EOF từ đối phương.
    eof: bool,
}

impl MemoryTransport {
    /// Tạo một cặp endpoint nối với nhau qua mạng ảo.
    pub fn pair(config: MemoryConfig) -> (Self, Self) {
        Self::pair_with_limits(config, FrameLimits::default())
    }

    /// Tạo một cặp endpoint với trần kích thước frame riêng.
    pub fn pair_with_limits(config: MemoryConfig, limits: FrameLimits) -> (Self, Self) {
        let network = Arc::new(Network {
            inner: Mutex::new(Shared::default()),
            signal: Condvar::new(),
        });
        let a = Self::new(Arc::clone(&network), Endpoint::A, config, limits);
        let b = Self::new(network, Endpoint::B, config, limits);
        (a, b)
    }

    fn new(
        network: Arc<Network>,
        endpoint: Endpoint,
        config: MemoryConfig,
        limits: FrameLimits,
    ) -> Self {
        // Mỗi chiều có dòng PRNG riêng để lỗi mạng hai chiều không phụ thuộc nhau, nhưng vẫn
        // tái lập được từ seed.
        let seed = config
            .seed
            .wrapping_add(0x9e37_79b9_7f4a_7c15u64.wrapping_mul(endpoint.index() as u64 + 1));
        Self {
            network,
            endpoint,
            limits,
            faults: config.faults,
            timing: config.timing,
            rng: Rng::new(seed),
            incoming: VecDeque::new(),
            closed: false,
            eof: false,
        }
    }

    /// Đồng hồ ảo hiện tại (ms ảo).
    pub fn clock_ms(&self) -> u64 {
        self.network.inner.lock().expect("khoá mạng ảo").clock_ms
    }

    /// Số frame **endpoint này gửi đi** đã bị mạng nuốt (mất).
    ///
    /// Quyết định mất lấy từ PRNG theo seed nên **tức thì** và giống nhau trên mọi OS:
    /// test biết chính xác frame vừa gửi có tới nơi không, thay vì đoán theo hạn thời gian
    /// thật (hạn thời gian phụ thuộc tốc độ máy và độ phân giải timer của hệ điều hành).
    pub fn dropped_frames(&self) -> u64 {
        self.counters().dropped_frames()
    }

    /// Số frame **đối phương gửi đi** đã bị mạng nuốt (mất) — chiều ngược lại.
    ///
    /// Dùng để biết câu trả lời từ server có bị mất không (khác với [`Self::dropped_frames`]
    /// là câu hỏi của chiều gửi đi).
    pub fn peer_dropped_frames(&self) -> u64 {
        self.counters().peer_dropped_frames()
    }

    /// Số frame endpoint này đã **đưa lên đường** — không bị mạng nuốt, kể cả bản trùng lặp.
    ///
    /// Mỗi frame đưa lên đường sẽ được đối phương lấy **đúng một lần**, nên đối chiếu với
    /// [`Self::peer_delivered_frames`] cho biết chính xác còn frame nào đang trên đường.
    pub fn sent_frames(&self) -> u64 {
        self.counters().sent_frames()
    }

    /// Số frame **đối phương** đã đưa lên đường (chiều ngược lại).
    pub fn peer_sent_frames(&self) -> u64 {
        self.counters().peer_sent_frames()
    }

    /// Số frame endpoint này đã **nhận** (đã lấy khỏi hàng đợi và trả cho ứng dụng).
    ///
    /// Khác [`Self::dropped_frames`]: đây là frame *tới nơi*, không phải frame bị nuốt.
    pub fn delivered_frames(&self) -> u64 {
        self.counters().delivered_frames()
    }

    /// Số frame **đối phương đã nhận** — tức frame endpoint này gửi đi đã tới nơi.
    ///
    /// Dùng để biết chắc đối phương (ví dụ luồng đọc của server) đã lấy frame khỏi mạng ảo:
    /// đếm theo số frame đã đưa lên đường mà **không** bị nuốt. Không phụ thuộc đồng hồ thật.
    pub fn peer_delivered_frames(&self) -> u64 {
        self.counters().peer_delivered_frames()
    }

    /// Handle bộ đếm của mạng ảo — **không** giữ hàng đợi nhận.
    ///
    /// Cho phép test quan sát số frame đã đưa lên đường / đã tới nơi **từ nơi khác** (ví dụ:
    /// test nắm transport phía client nhưng cần biết luồng đọc của server đã lấy frame chưa).
    pub fn counters(&self) -> MemoryCounters {
        MemoryCounters {
            network: Arc::clone(&self.network),
            endpoint: self.endpoint,
        }
    }

    /// Đã thấy EOF từ đối phương chưa (đối phương đóng và đã giao hết frame).
    pub fn is_eof(&self) -> bool {
        self.eof
    }

    /// Nhận frame **không chờ đối phương gửi thêm**.
    ///
    /// - `Ok(Some(frame))`: có frame; đồng hồ ảo được nâng tới mốc giao kế tiếp nếu cần.
    /// - `Ok(None)`: chưa có frame nào tới (đối phương có thể chưa gửi, hoặc đã đóng — xem
    ///   [`MemoryTransport::is_eof`]).
    /// - `Err(NetError::Closed)`: endpoint này đã đóng.
    ///
    /// Khác [`Transport::recv_frame`]: hàm này không chờ đối phương gửi thêm frame. Trong
    /// [`Timing::Real`] nó vẫn ngủ tới mốc giao của frame kế tiếp, để đồng hồ ảo khớp thời
    /// gian thật.
    pub fn try_recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        self.recv_impl(false)
    }

    /// Quyết định có xảy ra lỗi mạng với xác suất `permille` phần nghìn.
    fn roll(&mut self, permille: u16) -> bool {
        if permille == 0 {
            return false;
        }
        self.rng.below(PERMILLE) < u32::from(permille)
    }

    fn recv_impl(&mut self, block: bool) -> Result<Option<Vec<u8>>, NetError> {
        if self.closed {
            return Err(NetError::Closed);
        }
        let mut net = self.network.inner.lock().expect("khoá mạng ảo");
        loop {
            if let Some(frame) = self.incoming.pop_front() {
                // Frame đã tới nơi: đếm để ứng dụng biết chắc peer đã lấy frame này.
                net.delivered[self.endpoint.index()] += 1;
                return Ok(Some(frame));
            }
            // Mốc giao sớm nhất của các frame đang bay tới endpoint này.
            let target = net
                .packets
                .iter()
                .filter(|p| p.to == self.endpoint)
                .map(|p| p.deliver_at)
                .min();
            let Some(target) = target else {
                if net.closed[self.endpoint.peer().index()] {
                    // Đối phương đóng và không còn frame nào tới: EOF.
                    self.eof = true;
                    drop(net);
                    return Ok(None);
                }
                if !block {
                    drop(net);
                    return Ok(None);
                }
                // Chờ đối phương gửi hoặc đóng (giữ khoá khi chờ để không lỡ tín hiệu).
                net = self.network.signal.wait(net).expect("khoá mạng ảo");
                continue;
            };
            let delta = target.saturating_sub(net.clock_ms);
            net.clock_ms = net.clock_ms.max(target);
            if self.timing == Timing::Real && delta > 0 {
                drop(net);
                std::thread::sleep(Duration::from_millis(delta));
                net = self.network.inner.lock().expect("khoá mạng ảo");
            }
            // Giao các frame đã tới hạn, theo thứ tự (mốc giao, số thứ tự gửi).
            let now = net.clock_ms;
            let endpoint = self.endpoint;
            let mut ready: Vec<(u64, u64, Vec<u8>)> = Vec::new();
            net.packets.retain(|p| {
                if p.to == endpoint && p.deliver_at <= now {
                    ready.push((p.deliver_at, p.seq, p.payload.clone()));
                    false
                } else {
                    true
                }
            });
            ready.sort_by_key(|(deliver_at, seq, _)| (*deliver_at, *seq));
            self.incoming
                .extend(ready.into_iter().map(|(_, _, payload)| payload));
        }
    }
}

impl Transport for MemoryTransport {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        if self.closed {
            return Err(NetError::Closed);
        }
        if frame.len() > self.limits.max_frame_bytes {
            return Err(NetError::FrameTooLarge {
                len: frame.len(),
                max: self.limits.max_frame_bytes,
            });
        }
        // Thứ tự rút số ngẫu nhiên là cố định để lỗi mạng tái lập được.
        let lost = self.roll(self.faults.loss_permille);
        let duplicate = self.roll(self.faults.duplicate_permille);
        let reorder = self.roll(self.faults.reorder_permille);
        let jitter = if self.faults.jitter_ms == 0 {
            0
        } else {
            u64::from(self.rng.below(self.faults.jitter_ms + 1))
        };
        let reorder_extra = if reorder {
            u64::from(self.rng.below(self.faults.reorder_window_ms.max(1)))
        } else {
            0
        };
        let delay = u64::from(self.faults.delay_ms) + jitter + reorder_extra;
        if lost {
            // Mạng nuốt frame: ứng dụng gửi vẫn thành công, giống mạng thật.
            let mut net = self.network.inner.lock().expect("khoá mạng ảo");
            net.dropped[self.endpoint.index()] += 1;
            return Ok(());
        }
        {
            let mut net = self.network.inner.lock().expect("khoá mạng ảo");
            let now = net.clock_ms;
            let seq = net.next_seq;
            net.next_seq += 1;
            net.packets.push(Packet {
                to: self.endpoint.peer(),
                seq,
                deliver_at: now + delay,
                payload: frame.to_vec(),
            });
            // Frame đưa lên đường: đối phương sẽ lấy đúng một lần (kể cả bản trùng lặp).
            net.sent[self.endpoint.index()] += 1;
            if duplicate {
                let dup_seq = net.next_seq;
                net.next_seq += 1;
                net.packets.push(Packet {
                    to: self.endpoint.peer(),
                    seq: dup_seq,
                    deliver_at: now + delay + 1,
                    payload: frame.to_vec(),
                });
                net.sent[self.endpoint.index()] += 1;
            }
        }
        self.network.signal.notify_all();
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        self.recv_impl(true)
    }

    fn close(&mut self) -> Result<(), NetError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        {
            let mut net = self.network.inner.lock().expect("khoá mạng ảo");
            net.closed[self.endpoint.index()] = true;
        }
        self.network.signal.notify_all();
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed || self.eof
    }
}
