//! Chọn clip và frame cho trạng thái trình diễn của V1 (module thuần, test được không GPU).
//!
//! Module này **không** biết gì về macroquad: nó chỉ trả lời "state core này dùng clip
//! nào, frame nào". Việc đặt ảnh (toạ độ, z-order) nằm ở [`crate::scene`], phần vẽ
//! nằm ở [`crate::render`].
//!
//! Nguồn quyết định:
//!
//! - [RULES_V1 §8](../../../../docs/specification/gameplay/RULES_V1.md) — claim
//!   state → animation (R-077…R-079), quy ước trình diễn của V1 (R-084…R-088) và
//!   sổ trình diễn (R-089).
//! - [BUNDLES §7](../../../../docs/specification/assets/BUNDLES.md) — quy ước đặt ảnh
//!   của client (design).
//!
//! **Không** có hằng số luật chơi trong module: thời lượng frame đọc từ rules profile
//! (`presentation.frame_duration_ms`) và tần số tick từ `sim.ticks_per_second`; frame
//! tiến theo **đồng hồ trình diễn**, không theo đồng hồ wall clock (ADR 0004). Đồng hồ
//! trình diễn bám tick mô phỏng khi lượt còn chạy và **tiến nốt** sau khi lượt kết thúc
//! (R-089).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

use boom_core::{Cell, Dir, EventId, EventKind, Life, SimulationEvent};

/// Nhân vật V1 (SCOPE_V1 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Character {
    /// Nhân vật C00.
    C00,
    /// Nhân vật C01.
    C01,
}

impl Character {
    /// Khoá nhân vật: tiền tố ID clip trong bundle (`c00`, `c01`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::C00 => "c00",
            Self::C01 => "c01",
        }
    }
}

/// Nhân vật của người chơi theo `id`.
///
/// State core V1 **không** mang trường nhân vật (`PlayerSetup`/`Player` không có
/// trường này), nên V1 ghép `id` người chơi với nhân vật theo thứ tự `C00`, `C01`
/// (R-086). Khi có contract nhân vật trong state, hàm này được thay bằng cách đọc
/// trực tiếp từ state.
#[must_use]
pub fn character_of(player_id: u8) -> Character {
    match player_id {
        0 => Character::C00,
        _ => Character::C01,
    }
}

/// Clip ID của bom đang chờ nổ (đếm fuse).
pub const BOMB_CLIP: &str = "bomb.fuse";

/// Clip ID của lửa trên ô.
pub const FLAME_CLIP: &str = "flame.main";

/// Clip ID của bong bóng phủ lên người bị nhốt (#305: trước đây không được vẽ).
pub const TRAPPED_BUBBLE_CLIP: &str = "trapped.bubble";

/// Clip ID của chữ HELP! trên bong bóng người bị nhốt (`Bomb/HELP!`, F-02 #327).
pub const TRAPPED_HELP_CLIP: &str = "trapped.help";

/// Frame của chữ HELP!: lặp đều theo nhịp frame chung.
///
/// Video VN cho thấy chữ đổi hình liên tục suốt lúc bị nhốt; nhịp per-frame chưa đo nên dùng
/// `presentation.frame_duration_ms` (`design`, R-118; ADR 0014 bảng nợ F-02).
#[must_use]
pub fn help_frame(clock: &PresentationClock, count: usize) -> usize {
    clock.looping_frame(count)
}

/// Số frame "lớn dần" đầu clip bong bóng: phát một lần từ lúc bị nhốt (frame 0–2).
pub const BUBBLE_GROW_FRAMES: usize = 3;

/// Frame bóng tròn giữ suốt thời gian bị nhốt (`Bubble00` frame 6, #320).
pub const BUBBLE_HOLD_FRAME: usize = 6;

/// Frame đầu của chuỗi vỡ bóng (`Bubble00` frame 3–5), phát khi người bị nhốt chết.
pub const BUBBLE_POP_FIRST: usize = 3;

/// Số frame của chuỗi vỡ bóng.
pub const BUBBLE_POP_FRAMES: usize = 3;

/// Độ đục của bóng khi đang giữ: trong mờ, vẫn thấy nhân vật bên trong (video VN, #320).
pub const BUBBLE_HOLD_ALPHA: u8 = 160;

/// Bóng chuyển sang đục hẳn trong khoảng này (ms) trước khi vỡ (video VN ~0,6 s, #320).
pub const BUBBLE_OPAQUE_MS: u64 = 600;

/// Hình bong bóng tại một thời điểm: frame trong clip và độ đục (0–255).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BubbleLook {
    /// Chỉ số frame trong clip `trapped.bubble`.
    pub frame: usize,
    /// Độ đục khi vẽ.
    pub alpha: u8,
}

/// Bong bóng của người bị nhốt từ tick `since`, nhốt tối đa `trap_ticks` tick.
///
/// Theo client gốc VN (#320): lớn dần frame 0–2 **một lần**, giữ bóng tròn (frame 6) trong mờ,
/// `BUBBLE_OPAQUE_MS` cuối thì đục hẳn; **không** lặp. Clip ngắn hơn thì giữ frame cuối để
/// bóng không biến mất khi người chơi vẫn bị nhốt (#305).
#[must_use]
pub fn bubble_look(
    clock: &PresentationClock,
    since: u32,
    trap_ticks: u32,
    count: usize,
) -> Option<BubbleLook> {
    if count == 0 {
        return None;
    }
    let grow = count.min(BUBBLE_GROW_FRAMES);
    if let Some(frame) = clock.once_frame(since, grow) {
        return Some(BubbleLook {
            frame,
            alpha: BUBBLE_HOLD_ALPHA,
        });
    }
    let frame = BUBBLE_HOLD_FRAME.min(count - 1);
    let elapsed = clock.frame.elapsed_ms_since(since, clock.tick);
    let total = clock.frame.elapsed_ms_since(0, trap_ticks);
    let alpha = if elapsed.saturating_add(BUBBLE_OPAQUE_MS) >= total {
        u8::MAX
    } else {
        BUBBLE_HOLD_ALPHA
    };
    Some(BubbleLook { frame, alpha })
}

/// Frame vỡ bóng khi người bị nhốt chết: `death_frame` là frame hiện tại của chuỗi chết.
///
/// Ba frame đầu của chuỗi chết phủ thêm frame vỡ 3–5 của bóng (video VN, #320).
#[must_use]
pub fn bubble_pop_frame(death_frame: usize, count: usize) -> Option<usize> {
    (death_frame < BUBBLE_POP_FRAMES && count >= BUBBLE_POP_FIRST + BUBBLE_POP_FRAMES)
        .then_some(BUBBLE_POP_FIRST + death_frame)
}

/// Clip ID của item trên nền theo chỉ số `Rules::items` (V1: 0 Bomb, 1 Fire, 2 Roller).
#[must_use]
pub fn item_clip(kind: u16) -> Option<&'static str> {
    match kind {
        0 => Some("item.bomb"),
        1 => Some("item.fire"),
        2 => Some("item.roller"),
        _ => None,
    }
}

/// Clip ID của hiệu ứng nổ bom.
///
/// Pack KR 00967 chỉ có **một** clip nổ (`FxDesktop/Bomb/BombExplosion001`, R-084):
/// không tách tâm/nhánh/đuôi, nên V1 dùng clip này cho cả vụ nổ.
pub const EXPLOSION_CLIP: &str = "explosion.main";

/// Dải frame của clip lửa dùng cho **một ô** lửa.
///
/// Clip `flame.main` là **một** blob 100 frame gộp thân/đuôi/nhánh theo hướng; cách
/// chia chưa có consumer nào xác nhận (R-078, Q-003). Trên pack KR 00967, frame
/// 28..=59 là dải ảnh đơn 40×40 (hình lửa đứng giữa ô); V1 dùng dải này cho ô lửa
/// (R-087, [BUNDLES §7](../../../../docs/specification/assets/BUNDLES.md)).
///
/// Giới hạn đã biết: vì không biết hướng lửa trong state core (`Flame` không mang
/// hướng), mọi ô lửa vẽ cùng một frame của dải này.
pub const FLAME_FRAMES: RangeInclusive<usize> = 28..=59;

/// Nhịp phát frame: thời lượng một frame và tần số tick mô phỏng.
///
/// Cả hai giá trị lấy từ rules profile nên không có hằng số luật trong code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameClock {
    /// Thời lượng một frame (ms) — `presentation.frame_duration_ms` trong profile.
    pub frame_ms: u32,
    /// Số tick mỗi giây — `sim.ticks_per_second` trong profile.
    pub ticks_per_second: u32,
}

impl FrameClock {
    /// Số mili giây đã trôi qua theo tick mô phỏng.
    ///
    /// Tính từ tick, không từ đồng hồ wall clock: cùng tick ⇒ cùng frame (ADR 0004).
    #[must_use]
    pub fn elapsed_ms(&self, tick: u32) -> u64 {
        u64::from(tick) * 1000 / u64::from(self.ticks_per_second.max(1))
    }

    /// Chỉ số frame lặp của clip tại `tick` (khoảng `0..frame_count`).
    ///
    /// Frame tiến mỗi `frame_ms` mili giây mô phỏng và lặp về đầu khi hết clip.
    #[must_use]
    pub fn looping_frame(&self, tick: u32, frame_count: usize) -> usize {
        if frame_count == 0 {
            return 0;
        }
        let frame_ms = u64::from(self.frame_ms.max(1));
        let steps = self.elapsed_ms(tick) / frame_ms;
        (steps % frame_count as u64) as usize
    }

    /// Số mili giây mô phỏng đã trôi qua từ `start` tới `tick`.
    ///
    /// Tính từ **tick mô phỏng**, không từ đồng hồ wall clock: cùng khoảng tick cho
    /// cùng kết quả (ADR 0004).
    #[must_use]
    pub fn elapsed_ms_since(&self, start: u32, tick: u32) -> u64 {
        u64::from(tick.saturating_sub(start)) * 1000 / u64::from(self.ticks_per_second.max(1))
    }

    /// Chỉ số frame của clip **phát một lần** (không lặp) tại `tick`, tính từ `start`.
    ///
    /// Clip phát một lần hết sau `frame_count` frame: trả `None` từ đó trở đi (và
    /// `None` luôn nếu clip rỗng). Dùng cho hiệu ứng nổ: sau khoảng clip, ô đó về
    /// lại `flame.main` (R-088).
    #[must_use]
    pub fn once_frame(&self, start: u32, tick: u32, frame_count: usize) -> Option<usize> {
        if frame_count == 0 {
            return None;
        }
        let frame_ms = u64::from(self.frame_ms.max(1));
        let elapsed = self.elapsed_ms_since(start, tick);
        let total = u64::try_from(frame_count)
            .unwrap_or(u64::MAX)
            .saturating_mul(frame_ms);
        if elapsed >= total {
            return None;
        }
        Some((elapsed / frame_ms) as usize)
    }
}

/// Đồng hồ trình diễn: nhịp phát frame + **tick trình diễn** hiện tại.
///
/// Tick trình diễn **bám** tick mô phỏng khi lượt còn chạy và **tiến nốt** (cùng nhịp
/// `sim.ticks_per_second`) sau khi lượt **kết thúc** và mô phỏng dừng. Nhờ vậy công thức
/// chọn frame (R-085) vẫn đúng và chuỗi chết của lượt cuối vẫn phát **trọn clip**
/// thay vì kẹt ở frame đầu (R-089).
///
/// Vẫn **không** theo đồng hồ wall clock: cùng tick trình diễn ⇒ cùng hình (ADR 0004).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationClock {
    /// Nhịp phát frame (đọc từ rules profile).
    pub frame: FrameClock,
    /// Tick trình diễn hiện tại.
    pub tick: u32,
}

impl PresentationClock {
    /// Đồng hồ mới: nhịp frame `frame`, tick trình diễn `tick`.
    #[must_use]
    pub fn new(frame: FrameClock, tick: u32) -> Self {
        Self { frame, tick }
    }

    /// Chỉ số frame **lặp** của clip tại tick hiện tại.
    #[must_use]
    pub fn looping_frame(&self, frame_count: usize) -> usize {
        self.frame.looping_frame(self.tick, frame_count)
    }

    /// Chỉ số frame **phát một lần** của clip tính từ `start`.
    ///
    /// `None` khi hết khoảng clip (số frame × `presentation.frame_duration_ms`).
    #[must_use]
    pub fn once_frame(&self, start: u32, frame_count: usize) -> Option<usize> {
        self.frame.once_frame(start, self.tick, frame_count)
    }
}

/// Loại hiệu ứng trình diễn mà sổ theo dõi (R-089).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationKind {
    /// Vụ nổ tại ô bom.
    Explosion,
    /// Nhân vật chết.
    Death,
    /// Nhân vật bị nhốt.
    Trapped,
    /// Nhân vật bắt đầu lượt.
    Start,
}

/// Đối tượng một hiệu ứng áp lên.
///
/// Dùng để **một** hiệu ứng cho mỗi đối tượng: vụ nổ mới ở cùng ô thay vụ nổ cũ, chết
/// thay nhốt, và mục client tự sinh bị event xác nhận của cùng đối tượng thay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Subject {
    /// Nhân vật.
    Player(u8),
    /// Ô bản đồ.
    Cell(Cell),
}

/// Nguồn của một mục sổ trình diễn.
///
/// ADR 0005: sổ phân biệt **event dự đoán** (client tự sinh) và **event đã xác nhận**
/// (server quyết kết quả).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationSource {
    /// Event server đã xác nhận — có `(round_id, EventId)`.
    Confirmed,
    /// Client tự sinh: hiệu ứng chưa có event server (mở đầu lượt) hoặc dự đoán.
    Local,
}

/// Khoá một mục sổ đã xác nhận: `(round_id, EventId)`.
///
/// `EventId` ổn định theo `(tick, seq)` nên cùng input cho cùng ID: consumer dedup được
/// khi replay/reconcile (ARCHITECTURE §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresentationKey {
    /// ID lượt.
    pub round_id: u32,
    /// ID event trong lượt.
    pub event_id: EventId,
}

/// Một mục của sổ trình diễn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presentation {
    /// Khoá `(round_id, EventId)`; `None` với mục client tự sinh.
    pub key: Option<PresentationKey>,
    /// Loại hiệu ứng.
    pub kind: PresentationKind,
    /// Đối tượng áp dụng.
    pub subject: Subject,
    /// Nguồn của mục.
    pub source: PresentationSource,
    /// Tick mô phỏng bắt đầu phát (tick của event).
    pub start_tick: u32,
}

/// Một hiệu ứng đang phát tại một tick, kèm chỉ số frame trong clip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivePresentation {
    /// Loại hiệu ứng.
    pub kind: PresentationKind,
    /// Đối tượng áp dụng.
    pub subject: Subject,
    /// Nguồn của mục.
    pub source: PresentationSource,
    /// Chỉ số frame trong clip (tính theo tick mô phỏng).
    pub frame: usize,
}

/// Số đơn vị thời lượng ODT mỗi giây: bộ đếm của consumer tăng 1 mỗi lần chạy ở nhịp 30 Hz
/// (R-222, R-251).
pub const ODT_UNITS_PER_SECOND: u64 = 30;

/// Thời lượng "giữ vô hạn" trong bảng ODT (`0xFFFFFFFF`, R-223).
pub const ODT_HOLD_FOREVER: u32 = u32::MAX;

/// Một bước chuỗi phát: (chỉ số frame trong clip, thời lượng theo đơn vị ODT).
pub type Step = (u32, u32);

/// Số đơn vị ODT đã trôi từ tick `start` tới tick trình diễn hiện tại.
#[must_use]
pub fn odt_units_since(clock: &PresentationClock, start: u32) -> u64 {
    // Tính thẳng từ số tick (không qua ms) để không hụt do làm tròn.
    u64::from(clock.tick.saturating_sub(start)) * ODT_UNITS_PER_SECOND
        / u64::from(clock.frame.ticks_per_second.max(1))
}

/// Frame của chuỗi `steps` sau `units` đơn vị ODT.
///
/// Chuỗi **lặp** quay về đầu khi hết; chuỗi **một lần** trả `None` khi hết. Bước có thời lượng
/// [`ODT_HOLD_FOREVER`] giữ frame đó mãi (R-223).
#[must_use]
pub fn sequence_frame(steps: &[Step], units: u64, looping: bool) -> Option<usize> {
    let total: u64 = steps.iter().map(|&(_, len)| u64::from(len)).sum();
    if total == 0 {
        return None;
    }
    let mut at = if looping { units % total } else { units };
    for &(frame, len) in steps {
        if len == ODT_HOLD_FOREVER || at < u64::from(len) {
            return usize::try_from(frame).ok();
        }
        at -= u64::from(len);
    }
    None
}

/// Tổng thời lượng (đơn vị ODT) của chuỗi; `None` nếu có bước giữ vô hạn.
#[must_use]
pub fn sequence_units(steps: &[Step]) -> Option<u64> {
    steps.iter().try_fold(0u64, |sum, &(_, len)| {
        (len != ODT_HOLD_FOREVER).then(|| sum + u64::from(len))
    })
}

/// Số frame của các clip trình diễn (đọc từ bundle).
///
/// Sổ trình diễn không biết bundle: caller đưa vào bảng số frame theo clip ID, nên đổi
/// bundle không phải sửa sổ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCounts {
    counts: Vec<(String, usize)>,
    /// Chuỗi phát **chính** (chuỗi ODT đầu tiên) của clip, nếu bundle có (R-251).
    sequences: Vec<(String, Vec<Step>)>,
}

impl FrameCounts {
    /// Ghi số frame của một clip (ghi đè nếu clip đã có).
    pub fn set(&mut self, clip: &str, frames: usize) {
        match self.counts.iter_mut().find(|(id, _)| id == clip) {
            Some(slot) => slot.1 = frames,
            None => self.counts.push((clip.to_owned(), frames)),
        }
    }

    /// Ghi chuỗi phát chính của một clip (ghi đè nếu đã có).
    pub fn set_sequence(&mut self, clip: &str, steps: Vec<Step>) {
        match self.sequences.iter_mut().find(|(id, _)| id == clip) {
            Some(slot) => slot.1 = steps,
            None => self.sequences.push((clip.to_owned(), steps)),
        }
    }

    /// Chuỗi phát chính của clip; `None` nếu clip không có chuỗi ODT.
    #[must_use]
    pub fn sequence(&self, clip: &str) -> Option<&[Step]> {
        self.sequences
            .iter()
            .find(|(id, _)| id == clip)
            .map(|(_, steps)| steps.as_slice())
    }

    /// Số frame của clip; `None` nếu bundle không có clip đó.
    #[must_use]
    pub fn get(&self, clip: &str) -> Option<usize> {
        self.counts
            .iter()
            .find(|(id, _)| id == clip)
            .map(|(_, frames)| *frames)
    }
}

/// Loại hiệu ứng và đối tượng của một event server; `None` nếu event không có hiệu ứng
/// trình diễn.
#[must_use]
pub fn presentation_of(event: &SimulationEvent) -> Option<(PresentationKind, Subject)> {
    match &event.kind {
        EventKind::BombExploded { cell, .. } => {
            Some((PresentationKind::Explosion, Subject::Cell(*cell)))
        }
        EventKind::PlayerDied { player, .. } => {
            Some((PresentationKind::Death, Subject::Player(*player)))
        }
        EventKind::PlayerTrapped { player, .. } => {
            Some((PresentationKind::Trapped, Subject::Player(*player)))
        }
        _ => None,
    }
}

/// Clip ID của một hiệu ứng trình diễn theo loại và đối tượng.
///
/// Nhân vật V1 ghép với `C00`/`C01` theo `id` vì state core chưa mang trường nhân vật
/// (R-086).
#[must_use]
pub fn presentation_clip(kind: PresentationKind, subject: Subject) -> Option<String> {
    match (kind, subject) {
        (PresentationKind::Explosion, _) => Some(EXPLOSION_CLIP.to_owned()),
        (PresentationKind::Death, Subject::Player(id)) => {
            Some(format!("{}.dead.static", character_of(id).key()))
        }
        (PresentationKind::Trapped, Subject::Player(id)) => {
            Some(format!("{}.trapped.static", character_of(id).key()))
        }
        (PresentationKind::Start, Subject::Player(id)) => {
            Some(format!("{}.start.static", character_of(id).key()))
        }
        (_, Subject::Cell(_)) => None,
    }
}

/// Clip ID mà sổ trình diễn cần: nổ + chết/nhốt/mở đầu lượt của hai nhân vật V1.
#[must_use]
pub fn presentation_clip_ids() -> Vec<String> {
    let mut ids = vec![EXPLOSION_CLIP.to_owned()];
    for character in [Character::C00, Character::C01] {
        let key = character.key();
        ids.push(format!("{key}.dead.static"));
        ids.push(format!("{key}.trapped.static"));
        ids.push(format!("{key}.start.static"));
    }
    ids
}

/// Sổ trình diễn của client: các hiệu ứng đang phát, gắn với event đã xác nhận.
///
/// Sổ phân biệt **event dự đoán** (client tự sinh) và **event đã xác nhận** (ADR 0005):
///
/// - mục đã xác nhận gắn `(round_id, EventId)` ⇒ event lặp (reconcile/snapshot gửi lại
///   event cũ) **không** tạo mục mới;
/// - mục client tự sinh bị **thay** khi event xác nhận của cùng đối tượng tới.
///
/// Sổ ghi theo **tick mô phỏng** nên cùng tick cho cùng hình (ADR 0004), không theo đồng
/// hồ wall clock; thời lượng hiệu ứng lấy từ **số frame của clip** trong bundle và
/// nhịp frame trong rules profile — không có hằng số luật trong module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationJournal {
    /// Mục đang phát.
    items: Vec<Presentation>,
    /// Khoá event đã xác nhận — chống event lặp tạo mục mới.
    seen: BTreeSet<PresentationKey>,
    /// Lượt hiện tại của sổ.
    round_id: u32,
    /// Tick bắt đầu đứng yên của từng nhân vật (vắng = đang đi).
    idle_since: BTreeMap<u8, u32>,
}

impl PresentationJournal {
    /// Sổ mới cho lượt `round_id`.
    #[must_use]
    pub fn new(round_id: u32) -> Self {
        Self {
            items: Vec::new(),
            seen: BTreeSet::new(),
            round_id,
            idle_since: BTreeMap::new(),
        }
    }

    /// Ghi nhân vật `player` đang đi hay đứng yên tại `tick`: đứng yên thì nhớ tick bắt đầu.
    pub fn note_motion(&mut self, player: u8, moving: bool, tick: u32) {
        if moving {
            self.idle_since.remove(&player);
        } else {
            self.idle_since.entry(player).or_insert(tick);
        }
    }

    /// Tick bắt đầu đứng yên của nhân vật; `None` nếu đang đi hoặc chưa ghi.
    #[must_use]
    pub fn idle_since(&self, player: u8) -> Option<u32> {
        self.idle_since.get(&player).copied()
    }

    /// Chuyển sang lượt khác: dọn mục đang phát và quên event của lượt cũ.
    ///
    /// Event của lượt cũ không thể lặp lại trong lượt mới, nên khoá cũng bỏ.
    pub fn set_round(&mut self, round_id: u32) {
        if self.round_id == round_id {
            return;
        }
        self.round_id = round_id;
        self.items.clear();
        self.seen.clear();
        self.idle_since.clear();
    }

    /// Lượt hiện tại của sổ: khoá dedup event là `(round_id, EventId)`.
    ///
    /// Đây là **nguồn chuẩn duy nhất** của lượt trình diễn: client đọc lượt này để biết
    /// event nào thuộc lượt đang chơi.
    #[must_use]
    pub fn round_id(&self) -> u32 {
        self.round_id
    }

    /// Ghi hiệu ứng **mở đầu lượt** cho các nhân vật trong `slots`.
    ///
    /// Mở đầu lượt là **client tự sinh**: core V1 chưa có event mở đầu lượt, nên mục này
    /// **không** có khoá event (ADR 0005, R-089). Khi event server xác nhận cùng đối
    /// tượng tới thì mục dự đoán bị thay.
    ///
    /// `slots` là **slot trận** của người chơi: offline là id người chơi trong state core,
    /// mạng là slot trong roster của `RoundStart` (ADR 0009 Addendum B §B.2).
    pub fn record_start(&mut self, slots: &[u8], tick: u32) {
        for slot in slots {
            self.record_local(PresentationKind::Start, Subject::Player(*slot), tick);
        }
    }

    /// Ghi mục từ event server đã xác nhận.
    ///
    /// Trả `false` nếu event không có hiệu ứng trình diễn, hoặc event **lặp** (đã thấy
    /// `(round_id, EventId)`) — reconcile/snapshot gửi lại event cũ không được phát lại.
    pub fn record_event(&mut self, event: &SimulationEvent) -> bool {
        let Some((kind, subject)) = presentation_of(event) else {
            return false;
        };
        let key = PresentationKey {
            round_id: self.round_id,
            event_id: event.id,
        };
        if !self.seen.insert(key) {
            return false;
        }
        self.push(Presentation {
            key: Some(key),
            kind,
            subject,
            source: PresentationSource::Confirmed,
            start_tick: event.id.tick,
        });
        true
    }

    /// Ghi mục **client tự sinh** (hiệu ứng chưa có event server, hoặc dự đoán).
    ///
    /// Mục cũ cùng đối tượng bị thay. Nếu đối tượng đã có mục **đã xác nhận** thì bỏ
    /// qua: event server là nguồn chân lý (ADR 0005).
    pub fn record_local(&mut self, kind: PresentationKind, subject: Subject, tick: u32) -> bool {
        let confirmed = self
            .items
            .iter()
            .any(|item| item.subject == subject && item.source == PresentationSource::Confirmed);
        if confirmed {
            return false;
        }
        self.push(Presentation {
            key: None,
            kind,
            subject,
            source: PresentationSource::Local,
            start_tick: tick,
        });
        true
    }

    /// Thêm mục; mục cũ của cùng đối tượng bị thay (một đối tượng chỉ có một hiệu ứng).
    fn push(&mut self, item: Presentation) {
        self.items.retain(|old| old.subject != item.subject);
        self.items.push(item);
    }

    /// Hiệu ứng đang phát cho `subject` tại `tick`, kèm chỉ số frame.
    ///
    /// `None` nếu đối tượng không có hiệu ứng, hoặc hiệu ứng đã hết khoảng clip.
    #[must_use]
    pub fn active_for(
        &self,
        subject: Subject,
        clock: &PresentationClock,
        frames: &FrameCounts,
    ) -> Option<ActivePresentation> {
        let item = self.item_for(subject)?;
        let frame = frame_of(item, clock, frames)?;
        Some(ActivePresentation {
            kind: item.kind,
            subject: item.subject,
            source: item.source,
            frame,
        })
    }

    /// Mục của sổ cho `subject`: `None` nếu đối tượng không có hiệu ứng.
    ///
    /// Mục còn trong sổ dù đã hết khoảng clip (chưa dọn) vẫn trả về `Some` — dùng để
    /// đọc khoá event của mục đã xác nhận.
    #[must_use]
    pub fn item_for(&self, subject: Subject) -> Option<&Presentation> {
        self.items.iter().find(|item| item.subject == subject)
    }

    /// Các hiệu ứng đang phát tại tick trình diễn hiện tại.
    #[must_use]
    pub fn active(
        &self,
        clock: &PresentationClock,
        frames: &FrameCounts,
    ) -> Vec<ActivePresentation> {
        self.items
            .iter()
            .filter_map(|item| {
                let frame = frame_of(item, clock, frames)?;
                Some(ActivePresentation {
                    kind: item.kind,
                    subject: item.subject,
                    source: item.source,
                    frame,
                })
            })
            .collect()
    }

    /// Bỏ mục đã hết khoảng clip (theo số frame của clip và nhịp frame).
    ///
    /// Mục hết khoảng clip thì không còn hiệu ứng để phát. **Khoá event vẫn giữ** để event
    /// lặp sau đó không tạo mục mới; khoá chỉ bỏ khi đổi lượt.
    ///
    /// Bundle không có clip cần ⇒ mục bị bỏ: không có clip thì không có gì để phát.
    pub fn prune(&mut self, clock: &PresentationClock, frames: &FrameCounts) {
        self.items
            .retain(|item| frame_of(item, clock, frames).is_some());
    }
}

/// Chỉ số frame của một mục tại tick trình diễn; `None` nếu hết khoảng clip hoặc thiếu clip.
fn frame_of(item: &Presentation, clock: &PresentationClock, frames: &FrameCounts) -> Option<usize> {
    let clip = presentation_clip(item.kind, item.subject)?;
    // Có chuỗi ODT thì phát đúng chuỗi một lần (R-251); không có thì mỗi frame một nhịp chung.
    if let Some(steps) = frames.sequence(&clip) {
        return sequence_frame(steps, odt_units_since(clock, item.start_tick), false);
    }
    let count = frames.get(&clip)?;
    clock.once_frame(item.start_tick, count)
}

/// Clip ID của nhân vật theo state core.
///
/// - chết → clip `dead` (R-077, `confirmed`); chuỗi chết phát **một lần** theo sổ
///   trình diễn rồi giữ khung cuối có ảnh (R-089, xem [`crate::scene`]);
/// - bị nhốt → clip `trapped` (R-078, **candidate** — chưa có consumer gốc xác nhận);
/// - đang đi → clip `walk` theo hướng (R-077, `confirmed`);
/// - đứng yên → clip `walk` của hướng cuối, giữ frame cuối (R-198, #320).
///
/// `moving` là việc nhân vật **đã đổi ô** so với tick trước: state core không có cờ
/// "đang đi" (hướng giữ qua lệnh `Dir::None` không xoá hướng), nên client tự so vị
/// trí giữa hai tick.
#[must_use]
pub fn character_clip(character: Character, life: Life, facing: Dir) -> String {
    let key = character.key();
    match life {
        Life::Dead => format!("{key}.dead.static"),
        Life::Trapped { .. } => format!("{key}.trapped.static"),
        // Đứng yên cũng dùng clip đi theo hướng cuối, giữ frame cuối (R-198, #320):
        // client gốc không về `Blink` khi vừa dừng.
        Life::Alive => format!("{key}.walk.{}.static", direction_key(facing)),
    }
}

/// Chỉ số frame của clip **lặp** của nhân vật tại `tick`.
///
/// Nhân vật **còn sống** dùng clip lặp (đi/đứng); nhân vật **bị nhốt** cũng lặp clip
/// `trapped` sau khi chuỗi nhốt phát xong. Nhân vật **chết** thì **không** lặp: chuỗi
/// chết phát **một lần** theo sổ trình diễn rồi giữ khung cuối có ảnh (R-089) — biết
/// khung nào có pixel cần đọc bundle nên phần đó nằm ở [`crate::scene`].
#[must_use]
pub fn character_loop_frame(clock: &PresentationClock, frame_count: usize) -> usize {
    clock.looping_frame(frame_count)
}

/// Nhân vật **đang đi** khi vị trí đổi so với tick trước.
///
/// State core không có cờ "đang đi": hướng được giữ qua lệnh `Dir::None` (movement
/// bỏ qua người chơi không giữ phím nên `facing` không về `None`). Vì vậy client so
/// vị trí giữa hai tick; tick đầu tiên không có vị trí trước nên coi như đứng yên.
#[must_use]
pub fn is_moving(current: (i32, i32), previous: Option<(i32, i32)>) -> bool {
    match previous {
        Some(before) => before != current,
        None => false,
    }
}

/// Chỉ số frame của bom đang chờ: lặp trong clip (tia lửa xoay, R-086).
#[must_use]
pub fn bomb_frame(clock: &PresentationClock, frame_count: usize) -> usize {
    clock.looping_frame(frame_count)
}

/// Chỉ số frame của lửa trên ô: lặp trong dải frame lõi của clip lửa (R-087).
#[must_use]
pub fn flame_frame(clock: &PresentationClock, frame_count: usize) -> usize {
    let start = *FLAME_FRAMES.start();
    let count = (FLAME_FRAMES.end() - FLAME_FRAMES.start() + 1).min(frame_count);
    if count == 0 {
        return 0;
    }
    start + clock.looping_frame(count)
}

/// Tên hướng trong nhãn state ODT (`Walk Left`, `Stop Down`…). `Dir::None` dùng `Down`.
#[must_use]
pub fn direction_name(dir: Dir) -> &'static str {
    match dir {
        Dir::None | Dir::Down => "Down",
        Dir::Up => "Up",
        Dir::Left => "Left",
        Dir::Right => "Right",
    }
}

/// Khoá hướng dạng `snake_case` — trùng tên state trong bundle.
///
/// `Dir::None` không có clip riêng (hướng chỉ giữ khi đi); V1 dùng `down` cho trường
/// hợp đó.
fn direction_key(dir: Dir) -> &'static str {
    match dir {
        Dir::None | Dir::Down => "down",
        Dir::Up => "up",
        Dir::Left => "left",
        Dir::Right => "right",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_core::EntityId;

    /// Nhịp 33 Hz, frame 100 ms — như profile dev V1.
    const CLOCK: FrameClock = FrameClock {
        frame_ms: 100,
        ticks_per_second: 33,
    };

    /// Đồng hồ trình diễn tại `tick`.
    fn clock_at(tick: u32) -> PresentationClock {
        PresentationClock::new(CLOCK, tick)
    }

    #[test]
    fn bubble_grows_once_holds_translucent_then_turns_opaque_without_looping() {
        let frame_ticks = |n: u32| -> u32 {
            // Tick đầu tiên mà đồng hồ đã qua `n` frame kể từ tick 0.
            (0..10_000)
                .find(|&tick| clock_at(tick).once_frame(0, n as usize).is_none())
                .expect("đồng hồ tiến")
        };
        let trap = 378;
        let look = |tick| bubble_look(&clock_at(tick), 0, trap, 9).expect("có bóng");
        // 9 frame: 0,1,2 lớn dần một lần.
        assert_eq!(look(0).frame, 0);
        let after_grow = frame_ticks(3);
        // Sau đó giữ bóng tròn (frame 6) trong mờ, không lặp qua frame vỡ 3–5 (#320).
        // CLOCK test 33 tick/s: 0,6 s ≈ 20 tick.
        for tick in after_grow..trap - 25 {
            assert_eq!(
                look(tick),
                BubbleLook {
                    frame: BUBBLE_HOLD_FRAME,
                    alpha: BUBBLE_HOLD_ALPHA
                },
                "tick {tick}"
            );
        }
        // 0,6 s cuối: đục hẳn.
        assert_eq!(look(trap - 10).alpha, u8::MAX);
        assert_eq!(look(trap).frame, BUBBLE_HOLD_FRAME);
    }

    #[test]
    fn odt_sequences_play_once_loop_and_hold() {
        let walk: Vec<Step> = vec![(0, 3), (1, 3), (2, 3)];
        assert_eq!(sequence_frame(&walk, 0, false), Some(0));
        assert_eq!(sequence_frame(&walk, 3, false), Some(1));
        assert_eq!(sequence_frame(&walk, 8, false), Some(2));
        // Một lần: hết chuỗi thì không còn frame; lặp: quay về đầu.
        assert_eq!(sequence_frame(&walk, 9, false), None);
        assert_eq!(sequence_frame(&walk, 9, true), Some(0));
        assert_eq!(sequence_units(&walk), Some(9));
        // `0xFFFFFFFF` giữ frame mãi (Bubble Lock, R-223).
        let lock: Vec<Step> = vec![(0, 4), (1, 4), (2, 4), (6, ODT_HOLD_FOREVER)];
        assert_eq!(sequence_frame(&lock, 12, false), Some(6));
        assert_eq!(sequence_frame(&lock, 1_000_000, false), Some(6));
        assert_eq!(sequence_units(&lock), None);
        assert_eq!(sequence_frame(&[], 0, true), None);
        // Đơn vị ODT = 1/30 s (R-222): 1 giây trình diễn = 30 đơn vị.
        let clock = PresentationClock::new(
            FrameClock {
                frame_ms: 100,
                ticks_per_second: 30,
            },
            30,
        );
        assert_eq!(odt_units_since(&clock, 0), 30);
    }

    #[test]
    fn journal_plays_the_odt_sequence_once_for_death() {
        let mut frames = FrameCounts::default();
        frames.set("c00.dead.static", 14);
        frames.set_sequence("c00.dead.static", vec![(0, 2), (1, 2), (13, 2)]);
        let mut journal = PresentationJournal::new(1);
        journal.record_local(PresentationKind::Death, Subject::Player(0), 0);
        let clock = |tick| {
            PresentationClock::new(
                FrameClock {
                    frame_ms: 100,
                    ticks_per_second: 30,
                },
                tick,
            )
        };
        let frame_at = |tick| {
            journal
                .active_for(Subject::Player(0), &clock(tick), &frames)
                .map(|active| active.frame)
        };
        // 30 tick/s ⇒ 1 tick = 1 đơn vị ODT.
        assert_eq!(frame_at(0), Some(0));
        assert_eq!(frame_at(2), Some(1));
        assert_eq!(frame_at(5), Some(13));
        assert_eq!(frame_at(6), None);
    }

    #[test]
    fn journal_remembers_when_each_player_stopped() {
        let mut journal = PresentationJournal::new(1);
        journal.note_motion(0, false, 10);
        journal.note_motion(0, false, 20);
        assert_eq!(journal.idle_since(0), Some(10));
        journal.note_motion(0, true, 21);
        assert_eq!(journal.idle_since(0), None);
        journal.note_motion(0, false, 30);
        journal.set_round(2);
        assert_eq!(journal.idle_since(0), None);
    }

    #[test]
    fn short_or_empty_bubble_never_disappears_while_trapped() {
        assert_eq!(bubble_look(&clock_at(0), 0, 378, 0), None);
        for count in 1..=3 {
            for tick in [0_u32, 50, 300, 378] {
                let look = bubble_look(&clock_at(tick), 0, 378, count).expect("vẫn có bong bóng");
                assert!(look.frame < count, "count {count} tick {tick}: {look:?}");
            }
            // Sau khi lớn dần xong thì giữ frame cuối.
            assert_eq!(
                bubble_look(&clock_at(300), 0, 378, count).map(|look| look.frame),
                Some(count - 1)
            );
        }
    }

    #[test]
    fn bubble_pops_over_the_first_death_frames() {
        assert_eq!(bubble_pop_frame(0, 9), Some(3));
        assert_eq!(bubble_pop_frame(2, 9), Some(5));
        assert_eq!(bubble_pop_frame(3, 9), None);
        assert_eq!(bubble_pop_frame(0, 5), None);
    }

    #[test]
    fn only_the_three_v1_items_have_clips() {
        assert_eq!(item_clip(0), Some("item.bomb"));
        assert_eq!(item_clip(1), Some("item.fire"));
        assert_eq!(item_clip(2), Some("item.roller"));
        assert_eq!(item_clip(3), None);
    }

    #[test]
    fn character_clip_follows_the_core_state() {
        let c00 = Character::C00;
        // Đi theo bốn hướng.
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Up),
            "c00.walk.up.static"
        );
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Down),
            "c00.walk.down.static"
        );
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Left),
            "c00.walk.left.static"
        );
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Right),
            "c00.walk.right.static"
        );
        // Đứng yên.
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Down),
            "c00.walk.down.static"
        );
        // Bị nhốt và chết.
        assert_eq!(
            character_clip(c00, Life::Trapped { since: 7 }, Dir::Down),
            "c00.trapped.static"
        );
        assert_eq!(
            character_clip(c00, Life::Dead, Dir::Down),
            "c00.dead.static"
        );
        // Nhân vật C01 dùng tiền tố clip khác.
        assert_eq!(
            character_clip(Character::C01, Life::Alive, Dir::Left),
            "c01.walk.left.static"
        );
    }

    #[test]
    fn character_of_maps_the_two_v1_characters() {
        assert_eq!(character_of(0), Character::C00);
        assert_eq!(character_of(1), Character::C01);
    }

    #[test]
    fn frame_advances_with_the_core_tick_only() {
        // 33 Hz, frame 100 ms ⇒ frame đổi mỗi ~3,3 tick.
        assert_eq!(CLOCK.elapsed_ms(0), 0);
        assert_eq!(CLOCK.elapsed_ms(33), 1000);
        assert_eq!(CLOCK.looping_frame(0, 5), 0);
        assert_eq!(CLOCK.looping_frame(3, 5), 0);
        assert_eq!(CLOCK.looping_frame(4, 5), 1);
        // Lặp về đầu khi hết clip: 5 frame × 100 ms = 500 ms ≈ 16,5 tick.
        assert_eq!(CLOCK.looping_frame(16, 5), 4);
        assert_eq!(CLOCK.looping_frame(17, 5), 0);
        // Cùng tick ⇒ cùng frame (không phụ thuộc đồng hồ wall clock).
        assert_eq!(CLOCK.looping_frame(17, 5), CLOCK.looping_frame(17, 5));
    }

    #[test]
    fn dead_character_uses_the_journal_not_the_loop() {
        // Nhân vật còn sống thì frame lặp theo tick.
        assert_eq!(character_loop_frame(&clock_at(4), 5), 1);
        assert_eq!(character_loop_frame(&clock_at(4), 3), 1);
    }

    #[test]
    fn flame_frame_stays_inside_the_core_range() {
        for tick in [0_u32, 1, 7, 33, 100, 999] {
            let frame = flame_frame(&clock_at(tick), 100);
            assert!(
                (28..=59).contains(&frame),
                "frame lửa {frame} nằm ngoài dải lõi"
            );
        }
        // Lặp trong dải: sau 32 frame lặp lại từ đầu dải.
        assert_eq!(flame_frame(&clock_at(0), 100), 28);
        let period_ticks = 32 * 100 * 33 / 1000 + 1;
        assert_eq!(
            flame_frame(&clock_at(period_ticks), 100),
            flame_frame(&clock_at(0), 100)
        );
    }

    #[test]
    fn bomb_frame_loops_over_the_fuse_clip() {
        assert_eq!(bomb_frame(&clock_at(0), 4), 0);
        assert_eq!(bomb_frame(&clock_at(4), 4), 1);
        assert_eq!(bomb_frame(&clock_at(16), 4), 0);
    }

    /// Nhân vật đang đi khi vị trí đổi so với tick trước.
    #[test]
    fn moving_follows_the_position_change_between_ticks() {
        assert!(!is_moving((100, 100), None));
        assert!(!is_moving((100, 100), Some((100, 100))));
        assert!(is_moving((101, 100), Some((100, 100))));
    }

    /// Clip rỗng không được gây lỗi chia phép.
    #[test]
    fn empty_clip_has_no_frame() {
        assert_eq!(CLOCK.looping_frame(10, 0), 0);
        assert_eq!(character_loop_frame(&clock_at(10), 0), 0);
        assert_eq!(flame_frame(&clock_at(10), 0), 0);
        assert_eq!(CLOCK.once_frame(0, 10, 0), None);
    }

    /// Hiệu ứng nổ phát **một lần**: frame tiến theo tick rồi hết (không lặp).
    #[test]
    fn one_shot_clip_ends_after_its_own_length() {
        // Clip 10 frame × 100 ms = 1000 ms; ở 33 Hz một frame mỗi ~3,3 tick.
        assert_eq!(CLOCK.once_frame(0, 0, 10), Some(0));
        assert_eq!(CLOCK.once_frame(0, 3, 10), Some(0));
        assert_eq!(CLOCK.once_frame(0, 4, 10), Some(1));
        assert_eq!(CLOCK.once_frame(0, 32, 10), Some(9));
        // Hết khoảng clip: không còn frame nào để vẽ (ô đó về lại lửa).
        assert_eq!(CLOCK.once_frame(0, 33, 10), None);
        assert_eq!(CLOCK.once_frame(0, 999, 10), None);
        // Clip rỗng: không có gì để phát.
        assert_eq!(CLOCK.once_frame(0, 0, 0), None);
    }

    /// Event server đã xác nhận.
    fn event(tick: u32, seq: u16, kind: EventKind) -> SimulationEvent {
        SimulationEvent {
            id: EventId { tick, seq },
            kind,
        }
    }

    /// Vụ nổ của bom ở `cell`.
    fn explosion_event(tick: u32, seq: u16, cell: Cell) -> SimulationEvent {
        event(
            tick,
            seq,
            EventKind::BombExploded {
                bomb: EntityId(1),
                owner: 0,
                cell,
                chained: false,
            },
        )
    }

    /// Nhân vật `player` chết.
    fn died_event(tick: u32, seq: u16, player: u8) -> SimulationEvent {
        event(tick, seq, EventKind::PlayerDied { player, by: None })
    }

    /// Bảng số frame của clip: nổ 10 frame, chết 14, nhốt 3, mở đầu lượt 10.
    fn counts() -> FrameCounts {
        let mut frames = FrameCounts::default();
        frames.set(EXPLOSION_CLIP, 10);
        for key in ["c00", "c01"] {
            frames.set(&format!("{key}.dead.static"), 14);
            frames.set(&format!("{key}.trapped.static"), 3);
            frames.set(&format!("{key}.start.static"), 10);
        }
        frames
    }

    /// Sổ trình diễn: ghi vụ nổ theo ô, hết khoảng clip thì bỏ.
    #[test]
    fn journal_keeps_one_explosion_per_cell_until_the_clip_ends() {
        let mut journal = PresentationJournal::new(3);
        let frames = counts();
        assert!(journal.active(&clock_at(0), &frames).is_empty());
        let cell = Cell::new(2, 3);
        assert!(journal.record_event(&explosion_event(10, 0, cell)));
        // Vụ nổ mới ở cùng ô thay vụ cũ.
        assert!(journal.record_event(&explosion_event(20, 0, cell)));
        let active = journal.active(&clock_at(21), &frames);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].subject, Subject::Cell(cell));
        assert_eq!(active[0].frame, 0);
        // Ô khác: không có hiệu ứng.
        assert!(
            !journal
                .active(&clock_at(21), &frames)
                .iter()
                .any(|item| item.subject == Subject::Cell(Cell::new(0, 0)))
        );
        // Hết khoảng clip (10 frame × 100 ms) ⇒ không còn hiệu ứng ở ô đó.
        assert!(journal.active(&clock_at(54), &frames).is_empty());
        // Dọn sổ: vụ nổ đã hết bị bỏ.
        journal.prune(&clock_at(54), &frames);
        assert!(journal.active(&clock_at(54), &frames).is_empty());
        assert!(journal.record_event(&explosion_event(60, 0, cell)));
        journal.prune(&clock_at(61), &frames);
        assert_eq!(journal.active(&clock_at(61), &frames).len(), 1);
        // Bundle không có clip nổ ⇒ không có gì để phát ⇒ sổ bị dọn hết.
        journal.prune(&clock_at(62), &FrameCounts::default());
        assert!(journal.active(&clock_at(62), &frames).is_empty());
    }

    /// Event lặp (reconcile/snapshot gửi lại event cũ) không tạo mục mới.
    #[test]
    fn repeated_event_does_not_create_a_new_item() {
        let mut journal = PresentationJournal::new(3);
        let died = died_event(10, 0, 1);
        assert!(journal.record_event(&died));
        assert!(!journal.record_event(&died), "event lặp không được ghi lại");
        // Cùng event ở lượt khác là event khác ⇒ được ghi.
        journal.set_round(4);
        assert!(journal.record_event(&died));
        // Mục đã xác nhận giữ khoá đúng `(round_id, EventId)`.
        let item = journal.items.first().expect("có mục");
        assert_eq!(
            item.key,
            Some(PresentationKey {
                round_id: 4,
                event_id: EventId { tick: 10, seq: 0 },
            })
        );
        assert_eq!(item.source, PresentationSource::Confirmed);
    }

    /// Mục client tự sinh bị **thay** khi event xác nhận của cùng đối tượng tới.
    #[test]
    fn confirmed_event_replaces_the_local_prediction() {
        let mut journal = PresentationJournal::new(3);
        let frames = counts();
        let subject = Subject::Player(0);
        // Client dự đoán nhân vật chết (chưa có event server).
        assert!(journal.record_local(PresentationKind::Death, subject, 10));
        let active = journal
            .active_for(subject, &clock_at(11), &frames)
            .expect("mục dự đoán đang phát");
        assert_eq!(active.source, PresentationSource::Local);
        assert_eq!(active.frame, 0);
        // Event xác nhận tới ⇒ thay mục dự đoán, và bắt đầu lại từ tick của event.
        assert!(journal.record_event(&died_event(12, 1, 0)));
        let active = journal
            .active_for(subject, &clock_at(12), &frames)
            .expect("mục xác nhận đang phát");
        assert_eq!(active.source, PresentationSource::Confirmed);
        assert_eq!(active.frame, 0);
        // Mục xác nhận rồi thì client không tự ghi đè bằng dự đoán nữa.
        assert!(!journal.record_local(PresentationKind::Death, subject, 13));
    }

    /// Chuỗi chết phát **đủ frame** rồi kết thúc đúng (một lần, không lặp).
    #[test]
    fn death_clip_plays_every_frame_once() {
        let mut journal = PresentationJournal::new(3);
        let frames = counts();
        let subject = Subject::Player(0);
        assert!(journal.record_event(&died_event(10, 0, 0)));
        // Clip chết 14 frame × 100 ms: mỗi frame ~3,3 tick.
        let mut seen = Vec::new();
        for tick in 10..10 + 14 * 4 {
            if let Some(active) = journal.active_for(subject, &clock_at(tick), &frames) {
                seen.push((tick, active.frame));
            }
        }
        let played: Vec<usize> = seen.iter().map(|(_, frame)| *frame).collect();
        // Mỗi frame hiện ra khi tick đủ xa (100 ms/frame ở 33 Hz ≈ 3,3 tick), nên
        // chuỗi phải **tăng dần** và đi hết **đủ** 14 frame, không lặp về đầu.
        assert!(
            played.windows(2).all(|pair| pair[0] <= pair[1]),
            "chuỗi chết không lặp về đầu: {played:?}"
        );
        let mut unique = played.clone();
        unique.dedup();
        assert_eq!(
            unique,
            (0..14).collect::<Vec<usize>>(),
            "chuỗi chết phát đủ frame, mỗi frame đúng một lần"
        );
        // Hết khoảng clip thì không còn hiệu ứng nào để phát.
        assert!(
            journal
                .active_for(subject, &clock_at(10 + 14 * 4), &frames)
                .is_none()
        );
        // Dọn sổ: mục hết khoảng clip bị bỏ, nhưng khoá event **giữ** ⇒ event lặp vẫn
        // không tạo mục mới.
        journal.prune(&clock_at(10 + 14 * 4), &frames);
        assert!(
            journal
                .active_for(subject, &clock_at(10 + 14 * 4), &frames)
                .is_none()
        );
        assert!(!journal.record_event(&died_event(10, 0, 0)));
    }

    /// Mục cho từng loại hiệu ứng và đối tượng.
    #[test]
    fn journal_covers_death_trapped_start_and_explosion() {
        let mut journal = PresentationJournal::new(3);
        let frames = counts();
        assert!(journal.record_event(&event(1, 0, EventKind::PlayerTrapped { player: 0, by: 1 })));
        assert!(journal.record_event(&explosion_event(2, 1, Cell::new(1, 1))));
        assert!(journal.record_local(PresentationKind::Start, Subject::Player(1), 3));
        let kinds: Vec<PresentationKind> = journal
            .active(&clock_at(3), &frames)
            .iter()
            .map(|item| item.kind)
            .collect();
        assert!(kinds.contains(&PresentationKind::Trapped));
        assert!(kinds.contains(&PresentationKind::Explosion));
        assert!(kinds.contains(&PresentationKind::Start));
        // Clip ID theo loại hiệu ứng.
        assert_eq!(
            presentation_clip(PresentationKind::Death, Subject::Player(1)),
            Some("c01.dead.static".to_owned())
        );
        assert_eq!(
            presentation_clip(PresentationKind::Start, Subject::Player(0)),
            Some("c00.start.static".to_owned())
        );
        assert_eq!(
            presentation_clip(PresentationKind::Explosion, Subject::Cell(Cell::new(0, 0))),
            Some(EXPLOSION_CLIP.to_owned())
        );
    }
}
