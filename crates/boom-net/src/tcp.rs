//! Transport TCP (ADR 0006): một kết nối TCP mỗi client, `TCP_NODELAY`, frame độ dài tiền tố.

use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};

use crate::error::NetError;
use crate::frame::{FrameLimits, read_frame, write_frame};
use crate::transport::Transport;

/// Kết nối TCP đã bật `TCP_NODELAY`, mang frame độ dài tiền tố.
///
/// Một transport = một socket. Server nhận nhiều client bằng cách tạo một `TcpTransport` cho mỗi
/// kết nối được chấp nhận.
pub struct TcpTransport {
    /// `None` sau khi đóng: mọi lệnh gửi/nhận sau đó báo `NetError::Closed`.
    stream: Option<TcpStream>,
    limits: FrameLimits,
}

impl TcpTransport {
    /// Kết nối tới `addr` (phía client).
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self, NetError> {
        let stream = TcpStream::connect(addr)?;
        Self::from_stream(stream)
    }

    /// Chấp nhận một kết nối (phía server).
    pub fn accept(listener: &TcpListener) -> Result<Self, NetError> {
        let (stream, _peer) = listener.accept()?;
        Self::from_stream(stream)
    }

    /// Bọc một `TcpStream` đã có, bật `TCP_NODELAY`.
    pub fn from_stream(stream: TcpStream) -> Result<Self, NetError> {
        stream.set_nodelay(true)?;
        Ok(Self {
            stream: Some(stream),
            limits: FrameLimits::default(),
        })
    }

    /// Đặt trần kích thước frame (mặc định 64 KiB theo ADR 0009).
    pub fn with_limits(mut self, limits: FrameLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Địa chỉ cục bộ (socket đã đóng thì `None`).
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.stream.as_ref().and_then(|s| s.local_addr().ok())
    }

    /// Địa chỉ đối phương (socket đã đóng thì `None`).
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.stream.as_ref().and_then(|s| s.peer_addr().ok())
    }

    /// `TCP_NODELAY` đang bật hay không (socket đã đóng thì `None`).
    pub fn nodelay(&self) -> Option<bool> {
        self.stream.as_ref().and_then(|s| s.nodelay().ok())
    }
}

impl Transport for TcpTransport {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        let Some(stream) = self.stream.as_mut() else {
            return Err(NetError::Closed);
        };
        let result = write_frame(stream, frame, self.limits);
        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                // Frame quá trần không làm hỏng socket (chưa ghi byte nào); lỗi I/O thì có.
                if !matches!(&e, NetError::FrameTooLarge { .. }) {
                    self.stream = None;
                }
                Err(e)
            }
        }
    }

    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        let Some(stream) = self.stream.as_mut() else {
            return Err(NetError::Closed);
        };
        let result = read_frame(stream, self.limits);
        match result {
            Ok(Some(frame)) => Ok(Some(frame)),
            Ok(None) => {
                // Đối phương đóng êm: hết dữ liệu, socket không dùng được nữa.
                self.stream = None;
                Ok(None)
            }
            Err(e) => {
                // Dòng byte không còn đáng tin (frame quá trần/cắt) hoặc socket hỏng: đóng.
                self.stream = None;
                Err(e)
            }
        }
    }

    fn close(&mut self) -> Result<(), NetError> {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.stream.is_none()
    }
}
