//! Lỗi tầng transport (ADR 0006).
//!
//! Đây là lỗi kỹ thuật của kênh truyền, **không phải luật chơi**: không có giá trị nào ở đây nằm
//! trong `Rules` hay `data/rules/*.toml`.

use std::fmt;
use std::io;

/// Lỗi của transport.
#[derive(Debug)]
#[non_exhaustive]
pub enum NetError {
    /// Lỗi I/O dưới đáy (socket hỏng, đọc/ghi lỗi).
    Io(io::Error),
    /// Frame dài vượt trần của transport: bị **từ chối**, không cắt bớt/đệm.
    FrameTooLarge {
        /// Độ dài frame (byte) bị từ chối.
        len: usize,
        /// Trần đang áp dụng (byte).
        max: usize,
    },
    /// Kết nối đã đóng: không gửi/nhận được nữa (ta đóng, đối phương đóng, hoặc socket hỏng).
    Closed,
    /// Đối phương đóng giữa chừng một frame.
    TruncatedFrame {
        /// Số byte chờ đọc.
        expected: usize,
        /// Số byte đọc được trước khi kết nối đứt.
        received: usize,
    },
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "lỗi I/O mạng: {e}"),
            Self::FrameTooLarge { len, max } => {
                write!(f, "frame {len} byte vượt trần {max} byte")
            }
            Self::Closed => write!(f, "kết nối đã đóng"),
            Self::TruncatedFrame { expected, received } => {
                write!(f, "frame bị cắt: nhận {received}/{expected} byte")
            }
        }
    }
}

impl std::error::Error for NetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for NetError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_ro_luoi_hien_thi_ly_do() {
        assert_eq!(
            NetError::FrameTooLarge {
                len: 70_000,
                max: 65_536
            }
            .to_string(),
            "frame 70000 byte vượt trần 65536 byte"
        );
        assert_eq!(NetError::Closed.to_string(), "kết nối đã đóng");
        assert_eq!(
            NetError::TruncatedFrame {
                expected: 10,
                received: 3
            }
            .to_string(),
            "frame bị cắt: nhận 3/10 byte"
        );
    }

    #[test]
    fn loi_io_giu_nguon_goc() {
        use std::error::Error as _;
        let e = NetError::Io(io::Error::new(io::ErrorKind::ConnectionReset, "reset"));
        assert!(e.source().is_some());
    }
}
