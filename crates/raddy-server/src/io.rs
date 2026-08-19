use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body::{Body, Frame};
use hyper::body::Incoming;
use raddy_executor::ExecError;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::{mpsc, oneshot};

/// HTTP request body as [`AsyncRead`] for [`raddy_executor::ExecRequest`].
pub struct IncomingRead {
    incoming: Incoming,
    buf: Bytes,
}

impl IncomingRead {
    #[must_use]
    pub fn new(incoming: Incoming) -> Self {
        Self {
            incoming,
            buf: Bytes::new(),
        }
    }
}

impl AsyncRead for IncomingRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        dest: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if !this.buf.is_empty() {
                let n = this.buf.len().min(dest.remaining());
                dest.put_slice(&this.buf[..n]);
                this.buf = this.buf.slice(n..);
                return Poll::Ready(Ok(()));
            }
            match Pin::new(&mut this.incoming).poll_frame(cx) {
                Poll::Ready(Some(Ok(frame))) => match frame.into_data() {
                    Ok(data) => this.buf = data,
                    Err(_) => return Poll::Ready(Ok(())),
                },
                Poll::Ready(Some(Err(err))) => {
                    return Poll::Ready(Err(io::Error::other(err)));
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Guest body channel as an HTTP/1.1 body. Closes on `done` if the worker fails
/// after the head (C7 mid-body truncate).
pub struct GuestBody {
    rx: mpsc::Receiver<Bytes>,
    done: oneshot::Receiver<Result<(), ExecError>>,
    response_complete: Option<oneshot::Sender<()>>,
    finished: bool,
}

impl GuestBody {
    #[must_use]
    pub fn new(
        rx: mpsc::Receiver<Bytes>,
        done: oneshot::Receiver<Result<(), ExecError>>,
        response_complete: oneshot::Sender<()>,
    ) -> Self {
        Self {
            rx,
            done,
            response_complete: Some(response_complete),
            finished: false,
        }
    }

    fn finish(&mut self) {
        self.finished = true;
        if let Some(response_complete) = self.response_complete.take() {
            let _ = response_complete.send(());
        }
    }
}

impl Drop for GuestBody {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Body for GuestBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if this.finished {
            return Poll::Ready(None);
        }
        match this.rx.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(Frame::data(chunk)))),
            Poll::Ready(None) => {
                this.finish();
                Poll::Ready(None)
            }
            Poll::Pending => match Pin::new(&mut this.done).poll(cx) {
                Poll::Ready(Ok(Err(_))) => {
                    this.finish();
                    Poll::Ready(None)
                }
                Poll::Ready(Ok(Ok(()))) | Poll::Ready(Err(_)) => match this.rx.poll_recv(cx) {
                    Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(Frame::data(chunk)))),
                    _ => {
                        this.finish();
                        Poll::Ready(None)
                    }
                },
                Poll::Pending => Poll::Pending,
            },
        }
    }
}
