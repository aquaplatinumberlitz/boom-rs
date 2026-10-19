//! Trait transport (ADR 0006).
//!
//! Mọi code gameplay/phòng đi qua trait này, không gọi socket trực tiếp từ `boom-client`/`boom-server`.

use crate::error::NetError;

/// Kênh truyền **frame byte**.
///
/// Frame là byte thô: crate không biết kiểu message nào, nên không phụ thuộc DTO của `boom-proto`.
/// Envelope 16 byte của ADR 0009 nằm *trong* nội dung frame, do tầng ứng dụng đóng gói.
///
/// Mọi lỗi trả về là [`NetError`]; không có trạng thái "lỗi âm thầm". Dedup/ack theo sequence là
/// việc của tầng ứng dụng, không dựa vào transport (ADR 0006).
pub trait Transport {
    /// Gửi một frame.
    ///
    /// Lỗi `Err(NetError::FrameTooLarge)` nếu frame vượt trần của transport (không ghi gì ra đường
    /// truyền), `Err(NetError::Closed)` nếu kết nối đã đóng.
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError>;

    /// Nhận frame kế tiếp, chặn cho tới khi có.
    ///
    /// - `Ok(Some(frame))`: đã nhận trọn một frame.
    /// - `Ok(None)`: đối phương đóng êm (EOF) đúng ranh giới frame; không còn frame nào nữa.
    /// - `Err(_)`: lỗi I/O, frame vượt trần, frame bị cắt, hoặc transport đã đóng.
    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError>;

    /// Đóng kết nối từ phía ta. Gọi lại nhiều lần vẫn trả `Ok`.
    fn close(&mut self) -> Result<(), NetError>;

    /// Đã đóng chưa (đóng cục bộ, đối phương đóng, hoặc socket hỏng).
    fn is_closed(&self) -> bool;
}
