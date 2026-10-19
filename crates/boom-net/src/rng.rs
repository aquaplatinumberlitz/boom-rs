//! PRNG tất định cho transport trong bộ nhớ.
//!
//! Cùng họ SplitMix64 với `boom-core` nhưng là bản riêng: `boom-net` không phụ thuộc `boom-core`
//! (ARCHITECTURE §1), nên không dùng chung `Rng` của core. Số học 64 bit nguyên ⇒ cùng seed cho
//! cùng kết quả trên mọi OS.

/// SplitMix64: nhỏ, không phụ thuộc nền tảng, trạng thái nằm gọn trong một `u64`.
pub(crate) struct Rng {
    state: u64,
}

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Số nguyên đều trong `[0, n)`, không lệch: loại bỏ phần dư thay vì lấy modulo thẳng.
    pub(crate) fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "below(0)");
        let n = u64::from(n);
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let v = self.next_u64();
            if v < zone {
                return u32::try_from(v % n).expect("v % n < n ≤ u32::MAX");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_reference_values() {
        // Giá trị tham chiếu của SplitMix64 với seed 0 (Vigna), giống boom-core.
        let mut r = Rng::new(0);
        assert_eq!(r.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(r.next_u64(), 0x6e78_9e6a_a1b9_65f4);
    }

    #[test]
    fn below_luon_trong_khoang() {
        let mut r = Rng::new(42);
        for n in [1u32, 2, 3, 7, 1000] {
            for _ in 0..200 {
                assert!(r.below(n) < n);
            }
        }
    }
}
