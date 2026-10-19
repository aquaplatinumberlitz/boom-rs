//! FNV-1a 64 bit trên encoding chuẩn little-endian. Tên thuật toán ghi trong replay header.

pub const HASH_ALGO: &str = "fnv1a64-v0";

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Debug)]
pub struct Fnv1a64(u64);

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self(OFFSET)
    }
}

impl Fnv1a64 {
    pub fn bytes(&mut self, data: &[u8]) -> &mut Self {
        for &b in data {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(PRIME);
        }
        self
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.bytes(&[v])
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn i64(&mut self, v: i64) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    /// Chuỗi có tiền tố độ dài để hai trường liền nhau không trộn lẫn.
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.u32(u32::try_from(s.len()).expect("chuỗi quá dài"))
            .bytes(s.as_bytes())
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        // Vector chuẩn của FNV-1a 64.
        assert_eq!(Fnv1a64::default().finish(), 0xcbf2_9ce4_8422_2325);
        assert_eq!(
            Fnv1a64::default().bytes(b"a").finish(),
            0xaf63_dc4c_8601_ec8c
        );
        assert_eq!(
            Fnv1a64::default().bytes(b"foobar").finish(),
            0x8594_4171_f739_67e8
        );
    }
}
