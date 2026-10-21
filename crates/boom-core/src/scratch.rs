//! Dữ liệu tạm của **một tick**, truyền giữa các system — không thuộc `SimulationState`.
//!
//! **Vì sao không nằm trong state.** Kênh blast (ô bị vụ nổ phủ) chỉ sống trong tick: `explosion`
//! ghi, `blocks` đọc rồi xóa, hết tick là hết. Nếu nó là field của `SimulationState` thì state
//! mang dữ liệu không đóng góp vào luật ở mốc tick, nhưng lại phải đi qua state hash và DTO
//! snapshot — hoặc phải ghi ngoại lệ "rỗng ở mọi mốc tick" mà không có ai canh. Rời khỏi state
//! thì vấn đề biến mất: state không mang kênh này nên **không có gì để bỏ** khi đóng gói snapshot,
//! và state khôi phục luôn cho cùng kết quả tick.
//!
//! **Tính tất định.** Ở đường gameplay, [`crate::sim::SimulationState::step`] tự tạo scratch
//! **rỗng** cho mỗi tick, nên nội dung kênh blast luôn suy ra từ state trong chính tick đó:
//! cùng state + cùng input ⇒ cùng kết quả tick, không phụ thuộc thứ gì bị bỏ khi đóng gói.
//!
//! Đường fixture ([`crate::sim::SimulationState::step_with_scratch`]) cho phép bên gọi đưa sẵn
//! kênh blast để test riêng system `blocks`; ở đường đó scratch là **input thêm** của tick và
//! được kiểm riêng bằng [`TickScratch::check`] — không lẫn vào bất biến của state.

use crate::num::Cell;
use crate::sim::World;

/// Scratch của một tick: dữ liệu truyền giữa các system, không lưu vào state.
///
/// Không hash, không snapshot, không phải contract của `SimulationState` (ADR 0004 §5,
/// ARCHITECTURE §5.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TickScratch {
    /// Ô bị vụ nổ phủ trong tick đang chạy: `explosion` ghi, `blocks` đọc rồi xóa.
    ///
    /// Sắp theo ô (hàng rồi cột), mỗi ô một lần.
    blasts: Vec<Cell>,
}

impl TickScratch {
    /// Scratch rỗng: đường gameplay dùng cái này cho mỗi tick.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ô bị vụ nổ phủ đang chờ `blocks` xử lý.
    pub fn blasts(&self) -> &[Cell] {
        &self.blasts
    }

    /// Gộp ô blast mới vào hàng chờ: giữ thứ tự ô, mỗi ô một lần.
    ///
    /// `explosion` gọi để đưa ô vừa bị vụ nổ phủ vào kênh; ở đường gameplay kênh rỗng trước
    /// tick nên gộp cũng như ghi đè.
    pub fn merge_blasts(&mut self, cells: impl IntoIterator<Item = Cell>) {
        self.blasts.extend(cells);
        self.blasts.sort_unstable();
        self.blasts.dedup();
    }

    /// `blocks` lấy hết ô blast rồi xóa khỏi hàng chờ.
    pub fn take_blasts(&mut self) -> Vec<Cell> {
        std::mem::take(&mut self.blasts)
    }

    /// Bất biến của kênh blast: ô trong lưới, mỗi ô một lần, sắp theo ô.
    ///
    /// `check_invariants` của state **không** kiểm kênh này: kênh nằm ngoài state nên state
    /// không mang nó (xem doc của module). Hàm này là nơi kiểm duy nhất cho kênh blast.
    pub fn check(&self, world: &World) -> Result<(), String> {
        if !self.blasts.windows(2).all(|w| w[0] < w[1]) {
            return Err("kênh blast phải sắp theo ô, mỗi ô một lần".into());
        }
        let terrain = world.terrain();
        for c in &self.blasts {
            if c.x < 0
                || c.y < 0
                || c.x >= i32::from(terrain.width)
                || c.y >= i32::from(terrain.height)
            {
                return Err(format!("blast ở ô ngoài lưới {c:?}"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;

    use crate::mode::mode_by_id;
    use crate::rules::Rules;
    use crate::sim::World;

    use super::*;

    const PROFILE: &str = include_str!("../../../fixtures/rules/skeleton-test.toml");

    fn world() -> World {
        let rules = Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap();
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(&["0..", "..."]).unwrap(), rules, ffa).unwrap()
    }

    #[test]
    fn merge_sorts_cells_and_drops_duplicates() {
        let mut s = TickScratch::new();
        s.merge_blasts([Cell::new(2, 0), Cell::new(1, 0)]);
        s.merge_blasts([Cell::new(2, 0)]);
        assert_eq!(
            s.blasts(),
            [Cell::new(1, 0), Cell::new(2, 0)],
            "kênh blast sắp theo ô, mỗi ô một lần"
        );
        s.check(&world()).expect("ô trong lưới thì hợp lệ");
    }

    #[test]
    fn take_blasts_empties_the_channel() {
        let mut s = TickScratch::new();
        s.merge_blasts([Cell::new(1, 0)]);
        assert_eq!(s.take_blasts(), vec![Cell::new(1, 0)]);
        assert!(s.blasts().is_empty(), "đọc xong thì kênh rỗng");
    }

    #[test]
    fn cell_outside_the_grid_is_rejected() {
        let mut s = TickScratch::new();
        s.merge_blasts([Cell::new(9, 9)]);
        let err = s.check(&world()).expect_err("ô ngoài lưới phải bị từ chối");
        assert!(err.contains("ngoài lưới"), "thực tế: {err}");
    }
}
