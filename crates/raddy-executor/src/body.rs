use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

/// In-memory request body for tests and later HTTP adapters.
#[derive(Debug)]
pub struct MemoryBody {
    data: Vec<u8>,
    off: usize,
}

impl MemoryBody {
    #[must_use]
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self {
            data: data.into(),
            off: 0,
        }
    }
}

impl AsyncRead for MemoryBody {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let remaining = self.data.len().saturating_sub(self.off);
        let n = remaining.min(buf.remaining());
        buf.put_slice(&self.data[self.off..self.off + n]);
        self.off += n;
        Poll::Ready(Ok(()))
    }
}

/// A request body that never yields EOF. Used to prove execute does not wait
/// on the client finishing the body before the guest can emit a head.
#[derive(Debug, Default)]
pub struct OpenBody;

impl AsyncRead for OpenBody {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}
