//! Framing của transport: frame có **độ dài tiền tố** (ADR 0006).
//!
//! Mỗi frame là một chuỗi byte có độ dài cố định trên đường truyền: 4 byte độ dài
//! (`u32` little-endian) rồi tới nội dung. Tầng này không biết kiểu message — nội dung frame
//! có thể là envelope 16 byte của `boom-proto` (ADR 0009) hoặc bất kỳ byte nào khác.
//!
//! Trần kích thước là **tham số giao thức** (`ProtocolLimits` ở ADR 0009), không phải luật chơi
//! và không nằm trong `Rules`.

use std::io::{self, Read, Write};

use crate::error::NetError;

/// Số byte của tiền tố độ dài frame.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Trần kích thước frame mặc định: 64 KiB, tính cả envelope (ADR 0009 §"Giới hạn khung").
pub const DEFAULT_MAX_FRAME_BYTES: usize = 64 * 1024;

/// Tham số framing của một transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameLimits {
    /// Trần số byte của **nội dung** một frame (không tính tiền tố độ dài).
    pub max_frame_bytes: usize,
}

impl FrameLimits {
    /// Tham số V1: 64 KiB, khớp `MAX_FRAME_BYTES` của ADR 0009.
    pub const V1: FrameLimits = FrameLimits {
        max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
    };

    /// Đặt trần kích thước frame.
    pub const fn new(max_frame_bytes: usize) -> Self {
        Self { max_frame_bytes }
    }
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self::V1
    }
}

/// Ghi một frame: tiền tố độ dài + nội dung.
///
/// Frame vượt trần bị từ chối **trước khi** ghi byte nào ra đường truyền, để hai đầu không lệch
/// nhịp. Lỗi I/O được trả về nguyên vẹn.
pub fn write_frame<W: Write>(
    writer: &mut W,
    frame: &[u8],
    limits: FrameLimits,
) -> Result<(), NetError> {
    if frame.len() > limits.max_frame_bytes {
        return Err(NetError::FrameTooLarge {
            len: frame.len(),
            max: limits.max_frame_bytes,
        });
    }
    let Ok(len) = u32::try_from(frame.len()) else {
        return Err(NetError::FrameTooLarge {
            len: frame.len(),
            max: limits.max_frame_bytes,
        });
    };
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(frame)?;
    writer.flush()?;
    Ok(())
}

/// Đọc một frame.
///
/// - `Ok(Some(frame))`: đã đọc trọn một frame.
/// - `Ok(None)`: đối phương đóng êm đúng ranh giới frame (EOF), không còn frame nào.
/// - `Err(NetError::FrameTooLarge)`: tiền tố độ dài vượt trần — dòng byte không còn đáng tin,
///   người gọi phải đóng kết nối thay vì đọc tiếp.
/// - `Err(NetError::TruncatedFrame)`: kết nối đứt giữa chừng frame.
pub fn read_frame<R: Read>(
    reader: &mut R,
    limits: FrameLimits,
) -> Result<Option<Vec<u8>>, NetError> {
    let mut prefix = [0u8; LENGTH_PREFIX_BYTES];
    let got = read_up_to(reader, &mut prefix)?;
    if got == 0 {
        return Ok(None);
    }
    if got < LENGTH_PREFIX_BYTES {
        return Err(NetError::TruncatedFrame {
            expected: LENGTH_PREFIX_BYTES,
            received: got,
        });
    }
    let len = usize::try_from(u32::from_le_bytes(prefix)).unwrap_or(usize::MAX);
    if len > limits.max_frame_bytes {
        return Err(NetError::FrameTooLarge {
            len,
            max: limits.max_frame_bytes,
        });
    }
    // Cấp phát sau khi đã kiểm trần: độ dài độc hại không cấp phát vô hạn (ADR 0009 §5.2).
    let mut payload = vec![0u8; len];
    let got = read_up_to(reader, &mut payload)?;
    if got < len {
        return Err(NetError::TruncatedFrame {
            expected: len,
            received: got,
        });
    }
    Ok(Some(payload))
}

/// Đọc tối đa `buf.len()` byte; trả về số byte đã đọc (ít hơn `buf.len()` chỉ khi gặp EOF).
fn read_up_to<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<usize, NetError> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(NetError::Io(e)),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghi_doc_khoi_phuc_nguyen_frame() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"hello", FrameLimits::V1).unwrap();
        write_frame(&mut buf, b"", FrameLimits::V1).unwrap();
        write_frame(&mut buf, &[0xABu8; 300], FrameLimits::V1).unwrap();

        let mut cursor = buf.as_slice();
        assert_eq!(
            read_frame(&mut cursor, FrameLimits::V1).unwrap(),
            Some(b"hello".to_vec())
        );
        assert_eq!(
            read_frame(&mut cursor, FrameLimits::V1).unwrap(),
            Some(Vec::new())
        );
        assert_eq!(
            read_frame(&mut cursor, FrameLimits::V1).unwrap(),
            Some(vec![0xABu8; 300])
        );
        assert_eq!(read_frame(&mut cursor, FrameLimits::V1).unwrap(), None);
    }

    #[test]
    fn tien_to_dung_tien_dinh() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"abc", FrameLimits::V1).unwrap();
        assert_eq!(&buf[..4], &[3, 0, 0, 0]);
        assert_eq!(buf.len(), LENGTH_PREFIX_BYTES + 3);
    }

    #[test]
    fn frame_vuot_tran_bi_tu_choi_khi_ghi() {
        let mut buf = Vec::new();
        let err = write_frame(&mut buf, &[0u8; 17], FrameLimits::new(16)).unwrap_err();
        assert!(matches!(err, NetError::FrameTooLarge { len: 17, max: 16 }));
        assert!(buf.is_empty(), "phải từ chối trước khi ghi byte nào");
    }

    #[test]
    fn frame_vuot_tran_bi_tu_choi_khi_doc() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&1000u32.to_le_bytes());
        raw.extend_from_slice(&[0u8; 1000]);
        let mut cursor = raw.as_slice();
        let err = read_frame(&mut cursor, FrameLimits::new(16)).unwrap_err();
        assert!(matches!(
            err,
            NetError::FrameTooLarge { len: 1000, max: 16 }
        ));
    }

    #[test]
    fn ket_noi_dut_giua_frame_bao_loi() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&10u32.to_le_bytes());
        raw.extend_from_slice(b"abc");
        let mut cursor = raw.as_slice();
        let err = read_frame(&mut cursor, FrameLimits::V1).unwrap_err();
        assert!(matches!(
            err,
            NetError::TruncatedFrame {
                expected: 10,
                received: 3
            }
        ));
    }

    #[test]
    fn ket_noi_dut_giua_tien_to_bao_loi() {
        let raw = vec![1u8, 0];
        let mut cursor = raw.as_slice();
        let err = read_frame(&mut cursor, FrameLimits::V1).unwrap_err();
        assert!(matches!(
            err,
            NetError::TruncatedFrame {
                expected: LENGTH_PREFIX_BYTES,
                received: 2
            }
        ));
    }
}
