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

use std::collections::BTreeSet;
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

/// Số frame của các clip trình diễn (đọc từ bundle).
///
/// Sổ trình diễn không biết bundle: caller đưa vào bảng số frame theo clip ID, nên đổi
/// bundle không phải sửa sổ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCounts {
    counts: Vec<(String, usize)>,
}

impl FrameCounts {
    /// Ghi số frame của một clip (ghi đè nếu clip đã có).
    pub fn set(&mut self, clip: &str, frames: usize) {
        match self.counts.iter_mut().find(|(id, _)| id == clip) {
            Some(slot) => slot.1 = frames,
            None => self.counts.push((clip.to_owned(), frames)),
        }
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
}

impl PresentationJournal {
    /// Sổ mới cho lượt `round_id`.
    #[must_use]
    pub fn new(round_id: u32) -> Self {
        Self {
            items: Vec::new(),
            seen: BTreeSet::new(),
            round_id,
        }
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
    let count = frames.get(&clip)?;
    clock.once_frame(item.start_tick, count)
}

/// Clip ID của nhân vật theo state core.
///
/// - chết → clip `dead` (R-077, `confirmed`); chuỗi chết phát **một lần** theo sổ
///   trình diễn rồi giữ khung cuối có ảnh (R-089, xem [`crate::scene`]);
/// - bị nhốt → clip `trapped` (R-078, **candidate** — chưa có consumer gốc xác nhận);
/// - đang đi → clip `walk` theo hướng (R-077, `confirmed`);
/// - đứng yên → clip `idle` (R-078, **candidate**).
///
/// `moving` là việc nhân vật **đã đổi ô** so với tick trước: state core không có cờ
/// "đang đi" (hướng giữ qua lệnh `Dir::None` không xoá hướng), nên client tự so vị
/// trí giữa hai tick.
#[must_use]
pub fn character_clip(character: Character, life: Life, facing: Dir, moving: bool) -> String {
    let key = character.key();
    match life {
        Life::Dead => format!("{key}.dead.static"),
        Life::Trapped { .. } => format!("{key}.trapped.static"),
        Life::Alive if moving => format!("{key}.walk.{}.static", direction_key(facing)),
        Life::Alive => format!("{key}.idle.blink.static"),
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
    fn character_clip_follows_the_core_state() {
        let c00 = Character::C00;
        // Đi theo bốn hướng.
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Up, true),
            "c00.walk.up.static"
        );
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Down, true),
            "c00.walk.down.static"
        );
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Left, true),
            "c00.walk.left.static"
        );
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Right, true),
            "c00.walk.right.static"
        );
        // Đứng yên.
        assert_eq!(
            character_clip(c00, Life::Alive, Dir::Down, false),
            "c00.idle.blink.static"
        );
        // Bị nhốt và chết.
        assert_eq!(
            character_clip(c00, Life::Trapped { since: 7 }, Dir::Down, false),
            "c00.trapped.static"
        );
        assert_eq!(
            character_clip(c00, Life::Dead, Dir::Down, false),
            "c00.dead.static"
        );
        // Nhân vật C01 dùng tiền tố clip khác.
        assert_eq!(
            character_clip(Character::C01, Life::Alive, Dir::Left, true),
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
