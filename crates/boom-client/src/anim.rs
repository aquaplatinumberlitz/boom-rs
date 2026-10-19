//! Chọn clip và frame cho trạng thái trình diễn của V1 (module thuần, test được không GPU).
//!
//! Module này **không** biết gì về macroquad: nó chỉ trả lời "state core này dùng clip
//! nào, frame nào". Việc đặt ảnh (toạ độ, z-order) nằm ở [`crate::scene`], phần vẽ
//! nằm ở [`crate::render`].
//!
//! Nguồn quyết định:
//!
//! - [RULES_V1 §8](../../../../docs/specification/gameplay/RULES_V1.md) — claim
//!   state → animation (R-077…R-079) và quy ước trình diễn của V1 (R-084…R-088).
//! - [BUNDLES §7](../../../../docs/specification/assets/BUNDLES.md) — quy ước đặt ảnh
//!   của client (design).
//!
//! **Không** có hằng số luật chơi trong module: thời lượng frame đọc từ rules profile
//! (`presentation.frame_duration_ms`) và tần số tick từ `sim.ticks_per_second`; frame
//! tiến theo **tick mô phỏng**, không theo đồng hồ wall clock (ADR 0004).

use std::ops::RangeInclusive;

use boom_core::{Cell, Dir, Life};

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

/// Một vụ nổ đang phát trong trình diễn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Explosion {
    /// Ô của bom nổ.
    pub cell: Cell,
    /// Tick mô phỏng mà core báo vụ nổ (event `bomb_exploded`).
    pub tick: u32,
}

/// Sổ trình diễn của client: các vụ nổ đang phát.
///
/// V1 mới chỉ giữ **hiệu ứng nổ** trong sổ (chuỗi chết chưa phát — R-086). Sổ ghi
/// theo **tick mô phỏng** nên cùng tick cho cùng hình (ADR 0004), không theo đồng
/// hồ wall clock; thời lượng hiệu ứng lấy từ **số frame của clip** trong bundle và
/// nhịp frame trong rules profile — không có hằng số luật trong module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExplosionJournal {
    items: Vec<Explosion>,
}

impl ExplosionJournal {
    /// Ghi vụ nổ tại `cell`, bắt đầu ở `tick`.
    ///
    /// Vụ nổ mới ở cùng ô **thay** vụ cũ (một ô chỉ có một hiệu ứng nổ đang chạy).
    pub fn record(&mut self, cell: Cell, tick: u32) {
        self.items.retain(|item| item.cell != cell);
        self.items.push(Explosion { cell, tick });
    }

    /// Bỏ vụ nổ đã hết hiệu ứng (theo số frame của clip và nhịp frame).
    ///
    /// `frame_count` = 0 (bundle không có clip nổ) ⇒ bỏ hết: không có clip thì không
    /// có hiệu ứng để phát.
    pub fn prune(&mut self, tick: u32, clock: &FrameClock, frame_count: usize) {
        self.items
            .retain(|item| clock.once_frame(item.tick, tick, frame_count).is_some());
    }

    /// Các hiệu ứng nổ đang chạy tại `tick`: (ô, chỉ số frame).
    #[must_use]
    pub fn active(&self, tick: u32, clock: &FrameClock, frame_count: usize) -> Vec<(Cell, usize)> {
        self.items
            .iter()
            .filter_map(|item| {
                clock
                    .once_frame(item.tick, tick, frame_count)
                    .map(|frame| (item.cell, frame))
            })
            .collect()
    }
}

/// Clip ID của nhân vật theo state core.
///
/// - chết → clip `dead` (R-077, `confirmed`); nhân vật chết không vẽ (xem
///   [`character_frame`]);
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

/// Chỉ số frame của nhân vật tại `tick`.
///
/// Clip chết kết thúc bằng **ảnh rỗng 0×0** (dữ liệu thật của client gốc). V1 chưa có
/// presentation journal nên không phát lại chuỗi chết: dùng khung cuối ⇒ nhân vật chết
/// **không được vẽ** (BUNDLES §7, R-086). Chuỗi chết sẽ phát khi có journal (P7).
#[must_use]
pub fn character_frame(life: Life, tick: u32, clock: &FrameClock, frame_count: usize) -> usize {
    match life {
        Life::Dead => frame_count.saturating_sub(1),
        Life::Alive | Life::Trapped { .. } => clock.looping_frame(tick, frame_count),
    }
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
pub fn bomb_frame(tick: u32, clock: &FrameClock, frame_count: usize) -> usize {
    clock.looping_frame(tick, frame_count)
}

/// Chỉ số frame của lửa trên ô: lặp trong dải frame lõi của clip lửa (R-087).
#[must_use]
pub fn flame_frame(tick: u32, clock: &FrameClock, frame_count: usize) -> usize {
    let start = *FLAME_FRAMES.start();
    let count = (FLAME_FRAMES.end() - FLAME_FRAMES.start() + 1).min(frame_count);
    if count == 0 {
        return 0;
    }
    start + clock.looping_frame(tick, count)
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

    /// Nhịp 33 Hz, frame 100 ms — như profile dev V1.
    const CLOCK: FrameClock = FrameClock {
        frame_ms: 100,
        ticks_per_second: 33,
    };

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
    fn dead_character_uses_the_last_frame() {
        // Khung cuối của clip chết là ảnh 0×0 ⇒ nhân vật chết không vẽ.
        assert_eq!(character_frame(Life::Dead, 0, &CLOCK, 14), 13);
        assert_eq!(character_frame(Life::Dead, 999, &CLOCK, 14), 13);
        // Người còn sống thì frame lặp theo tick.
        assert_eq!(character_frame(Life::Alive, 4, &CLOCK, 5), 1);
        assert_eq!(character_frame(Life::Trapped { since: 1 }, 4, &CLOCK, 3), 1);
    }

    #[test]
    fn flame_frame_stays_inside_the_core_range() {
        for tick in [0_u32, 1, 7, 33, 100, 999] {
            let frame = flame_frame(tick, &CLOCK, 100);
            assert!(
                (28..=59).contains(&frame),
                "frame lửa {frame} nằm ngoài dải lõi"
            );
        }
        // Lặp trong dải: sau 32 frame lặp lại từ đầu dải.
        assert_eq!(flame_frame(0, &CLOCK, 100), 28);
        let period_ticks = 32 * 100 * 33 / 1000 + 1;
        assert_eq!(
            flame_frame(period_ticks, &CLOCK, 100),
            flame_frame(0, &CLOCK, 100)
        );
    }

    #[test]
    fn bomb_frame_loops_over_the_fuse_clip() {
        assert_eq!(bomb_frame(0, &CLOCK, 4), 0);
        assert_eq!(bomb_frame(4, &CLOCK, 4), 1);
        assert_eq!(bomb_frame(16, &CLOCK, 4), 0);
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
        assert_eq!(character_frame(Life::Alive, 10, &CLOCK, 0), 0);
        assert_eq!(flame_frame(10, &CLOCK, 0), 0);
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

    /// Sổ trình diễn: ghi vụ nổ theo ô, hết khoảng clip thì bỏ.
    #[test]
    fn journal_keeps_one_explosion_per_cell_until_the_clip_ends() {
        let mut journal = ExplosionJournal::default();
        assert!(journal.active(0, &CLOCK, 10).is_empty());
        let cell = Cell::new(2, 3);
        journal.record(cell, 10);
        // Vụ nổ mới ở cùng ô thay vụ cũ.
        journal.record(cell, 20);
        assert_eq!(journal.active(21, &CLOCK, 10), vec![(cell, 0)]);
        // Ô khác: không có hiệu ứng.
        assert!(
            !journal
                .active(21, &CLOCK, 10)
                .iter()
                .any(|(cell, _)| *cell == Cell::new(0, 0))
        );
        // Hết khoảng clip (10 frame × 100 ms) ⇒ không còn hiệu ứng ở ô đó.
        assert!(journal.active(54, &CLOCK, 10).is_empty());
        // Dọn sổ: vụ nổ đã hết bị bỏ, vụ nổ còn hiệu ưu giữ.
        journal.prune(54, &CLOCK, 10);
        assert!(journal.active(54, &CLOCK, 10).is_empty());
        journal.record(cell, 60);
        journal.prune(61, &CLOCK, 10);
        assert_eq!(journal.active(61, &CLOCK, 10), vec![(cell, 0)]);
        // Bundle không có clip nổ ⇒ không có gì để phát ⇒ sổ bị dọn hết.
        journal.prune(62, &CLOCK, 0);
        assert!(journal.active(62, &CLOCK, 0).is_empty());
    }
}
