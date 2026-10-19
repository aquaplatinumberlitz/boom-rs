//! Danh sách có trần: **không tin** length prefix của frame (ADR 0009 §5.2).
//!
//! Postcard đọc length prefix của một `Vec` rồi mới gọi visitor. Length prefix là dữ liệu từ
//! mạng nên không được tin: visitor ở đây **đếm phần tử thật** và dừng ngay khi vượt trần, nên
//! một frame độc hại không thể ép cấp phát danh sách khổng lồ. `SeqAccess::size_hint` của postcard
//! không dùng được vì nó trả `None` khi số byte còn lại ít hơn length prefix khai — tức là đúng
//! trường hợp cần chặn.
//!
//! Trần ở đây là **trần định dạng** (`limits::WIRE_*`), không phải miền của phiên: miền phiên
//! (`ProtocolLimits`) được kiểm sau decode và trả lỗi rõ ràng.

use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::de::{Deserializer, SeqAccess, Visitor};

/// Decode một `Vec` với tối đa `MAX` phần tử.
pub(crate) fn bounded<'de, D, T, const MAX: usize>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T> {
        max: usize,
        marker: PhantomData<fn() -> T>,
    }

    impl<'de, T> Visitor<'de> for Bounded<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "danh sách không quá {} phần tử", self.max)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Vec<T>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(value) = seq.next_element()? {
                if out.len() == self.max {
                    return Err(serde::de::Error::custom(format!(
                        "danh sách vượt trần định dạng {} phần tử",
                        self.max
                    )));
                }
                out.push(value);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_seq(Bounded {
        max: MAX,
        marker: PhantomData,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Holder {
        items: Vec<u32>,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Limited {
        #[serde(deserialize_with = "limited_u32")]
        items: Vec<u32>,
    }

    fn limited_u32<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
        bounded::<D, u32, 4>(d)
    }

    #[test]
    fn accepts_within_limit() {
        let bytes = postcard::to_allocvec(&Holder {
            items: vec![1, 2, 3],
        })
        .unwrap();
        let got: Limited = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(got.items, vec![1, 2, 3]);
    }

    #[test]
    fn rejects_count_over_limit() {
        // Length prefix khai 5 phần tử nhưng trần là 4.
        let bytes = postcard::to_allocvec(&Holder {
            items: vec![1, 2, 3, 4, 5],
        })
        .unwrap();
        let err = postcard::from_bytes::<Limited>(&bytes).unwrap_err();
        assert_eq!(err, postcard::Error::SerdeDeCustom);
    }

    #[test]
    fn hostile_length_prefix_does_not_allocate() {
        // Length prefix varint = 2^38 nhưng không có phần tử nào sau đó: decoder phải dừng khi
        // hết byte, không cấp phát theo length prefix.
        let bytes = vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x08];
        let err = postcard::from_bytes::<Limited>(&bytes).unwrap_err();
        assert_eq!(err, postcard::Error::DeserializeUnexpectedEnd);
    }
}
