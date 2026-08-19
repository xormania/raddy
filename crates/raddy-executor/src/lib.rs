//! Wasmtime pipeline: engine facade, typestate slots, and the `Executor` port.

mod body;
mod engine;
mod error;
mod guest_sdk;
mod host;
mod limits;
mod mock;
mod proto;
mod slot;
mod toy;

pub use body::{MemoryBody, OpenBody};
pub use engine::{EngineBuilder, EngineFacade};
pub use error::ExecError;
pub use guest_sdk::is_wasi_sdk_33;
pub use limits::{MAX_REQ_BODY_BYTES, MAX_RESP_CHUNK_BYTES, MAX_RESP_HEAD_BYTES};
pub use mock::MockCapabilities;
pub use proto::{ProtoEvent, Protocol};
pub use slot::{Cold, Executing, InstanceSlot, Warm};
pub use toy::{ToyExecutor, toy_guest_wasm};

use std::future::Future;

use bytes::Bytes;
use raddy_abi::{Envelope, ResponseHead};
use tokio::io::AsyncRead;
use tokio::sync::mpsc;

/// One request handed to an [`Executor`].
pub struct ExecRequest {
    pub head: Envelope,
    pub body: Box<dyn AsyncRead + Send + Unpin>,
}

/// Response whose body is streamed as the guest produces chunks.
pub struct ExecResponse {
    pub head: ResponseHead,
    /// Closed by the executor when the guest calls `raddy_resp_end`.
    pub body: mpsc::Receiver<Bytes>,
    /// Completes when the worker finishes. `Ok(())` only on a clean end.
    pub done: tokio::sync::oneshot::Receiver<Result<(), ExecError>>,
}

impl std::fmt::Debug for ExecRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecRequest")
            .field("head", &self.head)
            .field("body", &"<async-read>")
            .finish()
    }
}

impl std::fmt::Debug for ExecResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecResponse")
            .field("head", &self.head)
            .field("body", &"<channel>")
            .field("done", &"<oneshot>")
            .finish()
    }
}

/// The port the server calls. Resolves when the guest emits its response head.
pub trait Executor: Send + Sync + 'static {
    fn execute(
        &self,
        req: ExecRequest,
    ) -> impl Future<Output = Result<ExecResponse, ExecError>> + Send;
}

/// How a cold slot becomes warm. Snapshot lands in Stage 4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreStrategy {
    Fresh,
}
