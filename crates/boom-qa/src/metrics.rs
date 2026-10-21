//! Số liệu đo được của harness QA: độ trễ input, RTT ping/pong và phân vị.
//!
//! Mọi phép tính dùng số nguyên để kết quả không phụ thuộc float/OS (ADR 0004).

use serde::Serialize;

/// Một tập mẫu số nguyên có phân vị; caller quy định đơn vị (ms hoặc mpx).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Samples {
    values: Vec<u64>,
}

impl Samples {
    /// Tập rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Thêm một mẫu.
    pub fn push(&mut self, value: u64) {
        self.values.push(value);
    }

    /// Số mẫu.
    pub fn count(&self) -> usize {
        self.values.len()
    }

    /// Mẫu nhỏ nhất.
    pub fn min(&self) -> Option<u64> {
        self.values.iter().copied().min()
    }

    /// Mẫu lớn nhất.
    pub fn max(&self) -> Option<u64> {
        self.values.iter().copied().max()
    }

    /// Trung bình (làm tròn xuống).
    pub fn mean(&self) -> Option<u64> {
        if self.values.is_empty() {
            return None;
        }
        let total: u64 = self.values.iter().sum();
        Some(total / self.values.len() as u64)
    }

    /// Phân vị theo phương pháp nearest-rank: `percent` là số nguyên 0..=100.
    ///
    /// Không có mẫu nào thì `None`; không dùng 0 để tránh đọc nhầm "không đo được" là "0 ms".
    pub fn percentile(&self, percent: u32) -> Option<u64> {
        if self.values.is_empty() {
            return None;
        }
        let percent = u64::from(percent.min(100));
        let mut sorted = self.values.clone();
        sorted.sort_unstable();
        // nearest-rank: rank = ceil(percent * n / 100), lập chỉ số từ 1.
        let n = sorted.len() as u64;
        let rank = (percent * n).div_ceil(100).max(1);
        let index = (rank - 1).min(n - 1);
        sorted.get(usize::try_from(index).unwrap_or(0)).copied()
    }

    /// Tất cả mẫu (theo thứ tự thêm).
    pub fn values(&self) -> &[u64] {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_nearest_rank() {
        let mut samples = Samples::new();
        for value in 1..=100 {
            samples.push(value);
        }
        assert_eq!(samples.percentile(50), Some(50));
        assert_eq!(samples.percentile(95), Some(95));
        assert_eq!(samples.percentile(100), Some(100));
        assert_eq!(samples.percentile(0), Some(1));
        assert_eq!(samples.min(), Some(1));
        assert_eq!(samples.max(), Some(100));
        assert_eq!(samples.mean(), Some(50));
    }

    #[test]
    fn empty_samples_have_no_percentile() {
        let samples = Samples::new();
        assert_eq!(samples.percentile(95), None);
        assert_eq!(samples.mean(), None);
        assert_eq!(samples.count(), 0);
    }

    #[test]
    fn single_sample_is_every_percentile() {
        let mut samples = Samples::new();
        samples.push(7);
        assert_eq!(samples.percentile(50), Some(7));
        assert_eq!(samples.percentile(95), Some(7));
    }
}
