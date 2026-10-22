//! Rào IO cho đồng hồ ảo: chỉ tiến bước sau khi reader đã enqueue hết frame.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use boom_net::{MemoryTransport, NetError, Transport};

use crate::harness::QaError;

#[derive(Clone, Default)]
pub(crate) struct Progress {
    pub(crate) frames: Arc<AtomicU64>,
    pub(crate) closed: Arc<AtomicBool>,
}

impl Progress {
    pub(crate) fn wait(&self, target: u64) -> Result<(), QaError> {
        wait_until(|| self.frames.load(Ordering::Acquire) >= target)
    }
}

pub(crate) fn wait_until(mut done: impl FnMut() -> bool) -> Result<(), QaError> {
    let start = Instant::now();
    while !done() {
        if start.elapsed() > Duration::from_secs(10) {
            return Err(QaError::Net("rào IO không hoàn tất".into()));
        }
        thread::yield_now();
    }
    Ok(())
}

pub(crate) struct EnqueuedReader {
    inner: MemoryTransport,
    progress: Progress,
    pending: bool,
}

impl EnqueuedReader {
    pub(crate) fn new(inner: MemoryTransport) -> (Self, Progress) {
        let progress = Progress::default();
        (
            Self {
                inner,
                progress: progress.clone(),
                pending: false,
            },
            progress,
        )
    }
}

impl Transport for EnqueuedReader {
    fn send_frame(&mut self, frame: &[u8]) -> Result<(), NetError> {
        self.inner.send_frame(frame)
    }
    fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, NetError> {
        // Server gọi recv kế tiếp sau khi đã đẩy frame trước vào hàng đợi inbound.
        if self.pending {
            self.progress.frames.fetch_add(1, Ordering::Release);
        }
        let frame = self.inner.recv_frame()?;
        self.pending = frame.is_some();
        Ok(frame)
    }
    fn close(&mut self) -> Result<(), NetError> {
        self.inner.close()
    }
    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

impl Drop for EnqueuedReader {
    fn drop(&mut self) {
        // Reader thread đã enqueue Closed trước khi drop reader.
        self.progress.closed.store(true, Ordering::Release);
    }
}

pub(crate) struct Io {
    pub(crate) reader: Option<MemoryTransport>,
    pub(crate) retired: Arc<AtomicBool>,
    pub(crate) up: Progress,
}
