//! Transport V1 (ADR 0006): trait transport, TCP với frame có độ dài tiền tố, transport trong bộ nhớ.
//!
//! Mọi code gameplay/phòng đi qua trait [`Transport`], không gọi socket trực tiếp từ
//! `boom-client`/`boom-server`. Frame là **byte**: crate không biết kiểu message nào, nên không
//! phụ thuộc DTO của `boom-proto` — envelope 16 byte của ADR 0009 nằm *trong* nội dung frame.
//!
//! - [`FrameLimits`]: tham số framing (trần 64 KiB theo ADR 0009), là **tham số giao thức**,
//!   không phải luật chơi và không nằm trong `Rules`.
//! - [`TcpTransport`]: một kết nối TCP mỗi client, `TCP_NODELAY`.
//! - [`MemoryTransport`]: mạng ảo trong bộ nhớ, mô phỏng trễ/mất/trùng/đảo thứ tự theo seed cho
//!   fixture mạng V1-027.
//!
//! Không chứa luật chơi; dedup/ack theo sequence là việc của tầng ứng dụng (ADR 0009).
//!
//! ```
//! use boom_net::{MemoryConfig, MemoryTransport, Transport};
//!
//! let (mut client, mut server) = MemoryTransport::pair(MemoryConfig::default());
//! client.send_frame(b"ping").unwrap();
//! assert_eq!(server.recv_frame().unwrap().as_deref(), Some(&b"ping"[..]));
//! ```

mod error;
mod frame;
mod memory;
mod rng;
mod tcp;
mod transport;

pub use error::NetError;
pub use frame::{
    DEFAULT_MAX_FRAME_BYTES, FrameLimits, LENGTH_PREFIX_BYTES, read_frame, write_frame,
};
pub use memory::{MemoryConfig, MemoryCounters, MemoryFaults, MemoryTransport, Timing};
pub use tcp::TcpTransport;
pub use transport::Transport;
