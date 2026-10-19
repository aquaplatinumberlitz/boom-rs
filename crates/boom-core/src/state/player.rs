use boom_content::hash::Fnv1a64;

use crate::num::{Mpx, Tick};
use crate::sim::Dir;

/// Một chỉ số theo cấp có trần (BomberAttr: min là giá trị đầu, max là trần; RULES_V1 R-040…R-042).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stat {
    pub value: u8,
    pub max: u8,
}

impl Stat {
    pub const fn new(value: u8, max: u8) -> Self {
        Self { value, max }
    }

    /// Cộng `n` cấp, không vượt trần, không dưới 0. Trả về lượng thực tăng.
    pub fn add(&mut self, n: i32) -> i32 {
        let before = i32::from(self.value);
        let after = (before + n).clamp(0, i32::from(self.max));
        self.value = u8::try_from(after).expect("đã kẹp trong miền u8");
        after - before
    }
}

/// Chỉ số khởi đầu của một nhân vật (C00/C01 lấy từ BomberAttr qua content).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterStats {
    pub speed: Stat,
    pub bombs: Stat,
    pub range: Stat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Life {
    Alive,
    /// Bị nhốt trong bong bóng từ tick `since`; vẫn tính là chưa chết (SCOPE_V1 §4).
    Trapped {
        since: Tick,
    },
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub id: u8,
    /// Tâm nhân vật.
    pub x: Mpx,
    pub y: Mpx,
    pub facing: Dir,
    pub speed: Stat,
    /// Số bom được đặt cùng lúc.
    pub bombs: Stat,
    /// Tầm nổ của bom đặt tiếp theo (bom đã đặt giữ tầm lúc đặt, R-032).
    pub range: Stat,
    pub life: Life,
}

impl Player {
    pub fn is_dead(&self) -> bool {
        self.life == Life::Dead
    }

    pub(crate) fn hash_into(&self, h: &mut Fnv1a64) {
        h.u8(self.id)
            .i32(self.x.0)
            .i32(self.y.0)
            .u8(self.facing.code());
        for s in [self.speed, self.bombs, self.range] {
            h.u8(s.value).u8(s.max);
        }
        match self.life {
            Life::Alive => h.u8(0),
            Life::Trapped { since } => h.u8(1).u32(since),
            Life::Dead => h.u8(2),
        };
    }

    pub(crate) fn check(&self) -> Result<(), String> {
        for (name, s) in [
            ("speed", self.speed),
            ("bombs", self.bombs),
            ("range", self.range),
        ] {
            if s.value > s.max {
                return Err(format!(
                    "người chơi {}: {name} {} vượt trần {}",
                    self.id, s.value, s.max
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_clamps_to_max() {
        let mut s = Stat::new(6, 7);
        assert_eq!(s.add(1), 1);
        assert_eq!(s.add(1), 0);
        assert_eq!(s.value, 7);
        assert_eq!(s.add(-10), -7);
    }
}
