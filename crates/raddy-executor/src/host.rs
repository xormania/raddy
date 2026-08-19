use std::sync::Mutex;
use std::time::{Duration, Instant};

use bytes::Bytes;
use raddy_abi::{HeadCodec, JsonV1, ResponseHead};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::{mpsc, oneshot};
use wasmtime::{Caller, Linker, Memory};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::WasiP1Ctx;

use crate::ExecError;
use crate::limits::{MAX_REQ_BODY_BYTES, MAX_RESP_CHUNK_BYTES, MAX_RESP_HEAD_BYTES};
use crate::proto::{ProtoEvent, Protocol};

pub(crate) struct HostState {
    head: Vec<u8>,
    body: Mutex<Box<dyn AsyncRead + Send + Unpin>>,
    runtime: Option<tokio::runtime::Handle>,
    read_so_far: usize,
    proto: Protocol,
    head_tx: Option<oneshot::Sender<ResponseHead>>,
    body_tx: Option<mpsc::Sender<Bytes>>,
    fail: Option<ExecError>,
    deadline_at: Instant,
    teardown_deadline: Duration,
    teardown_deadline_at: Option<Instant>,
    pub(crate) wasi: WasiP1Ctx,
}

impl HostState {
    #[allow(dead_code)]
    pub(crate) fn new(
        head: Vec<u8>,
        body: Box<dyn AsyncRead + Send + Unpin>,
        runtime: tokio::runtime::Handle,
        head_tx: oneshot::Sender<ResponseHead>,
        body_tx: mpsc::Sender<Bytes>,
        deadline: Duration,
        teardown_deadline: Duration,
    ) -> Self {
        Self {
            head,
            body: Mutex::new(body),
            runtime: Some(runtime),
            read_so_far: 0,
            proto: Protocol::AwaitHead,
            head_tx: Some(head_tx),
            body_tx: Some(body_tx),
            fail: None,
            deadline_at: Instant::now() + deadline,
            teardown_deadline,
            teardown_deadline_at: None,
            wasi: WasiCtxBuilder::new()
                .allow_blocking_current_thread(true)
                .build_p1(),
        }
    }

    pub(crate) fn vacant() -> Self {
        let (head_tx, _) = oneshot::channel();
        let (body_tx, _) = mpsc::channel(1);
        Self {
            head: Vec::new(),
            body: Mutex::new(Box::new(crate::MemoryBody::new(Vec::new()))),
            runtime: None,
            read_so_far: 0,
            proto: Protocol::AwaitHead,
            head_tx: Some(head_tx),
            body_tx: Some(body_tx),
            fail: None,
            deadline_at: Instant::now() + Duration::from_secs(30),
            teardown_deadline: Duration::from_secs(2),
            teardown_deadline_at: None,
            wasi: WasiCtxBuilder::new()
                .allow_blocking_current_thread(true)
                .build_p1(),
        }
    }

    pub(crate) fn rebind(
        &mut self,
        head: Vec<u8>,
        body: Box<dyn AsyncRead + Send + Unpin>,
        runtime: tokio::runtime::Handle,
        head_tx: oneshot::Sender<ResponseHead>,
        body_tx: mpsc::Sender<Bytes>,
        deadline: Duration,
        teardown_deadline: Duration,
    ) {
        self.head = head;
        self.body = Mutex::new(body);
        self.runtime = Some(runtime);
        self.read_so_far = 0;
        self.proto = Protocol::AwaitHead;
        self.head_tx = Some(head_tx);
        self.body_tx = Some(body_tx);
        self.fail = None;
        self.deadline_at = Instant::now() + deadline;
        self.teardown_deadline = teardown_deadline;
        self.teardown_deadline_at = None;
        self.wasi = WasiCtxBuilder::new()
            .allow_blocking_current_thread(true)
            .build_p1();
    }

    pub(crate) fn take_fail(&mut self) -> Option<ExecError> {
        self.fail.take()
    }

    pub(crate) fn protocol(&self) -> Protocol {
        self.proto
    }

    pub(crate) fn finish_response(&mut self) {
        self.head_tx = None;
        self.body_tx = None;
    }

    fn finish_body(&mut self) {
        self.body_tx = None;
        self.teardown_deadline_at = Some(Instant::now() + self.teardown_deadline);
    }

    pub(crate) fn execution_remaining(&self) -> Duration {
        self.teardown_deadline_at
            .unwrap_or(self.deadline_at)
            .saturating_duration_since(Instant::now())
    }

    pub(crate) fn epoch_window(&self) -> Duration {
        self.execution_remaining().min(self.teardown_deadline)
    }

    fn remaining(&self) -> Duration {
        self.deadline_at.saturating_duration_since(Instant::now())
    }

    fn runtime_handle(&self) -> Result<tokio::runtime::Handle, wasmtime::Error> {
        self.runtime
            .clone()
            .ok_or_else(|| wasmtime::Error::msg("host runtime missing"))
    }

    fn violate<T>(&mut self, kind: &'static str) -> Result<T, wasmtime::Error> {
        self.fail = Some(ExecError::Protocol(kind.into()));
        Err(wasmtime::Error::msg(kind))
    }

    fn apply(&mut self, event: ProtoEvent) -> Result<(), wasmtime::Error> {
        match self.proto.apply(event) {
            Ok(next) => {
                self.proto = next;
                Ok(())
            }
            Err(kind) => self.violate(kind),
        }
    }
}

pub(crate) fn register(linker: &mut Linker<HostState>) -> Result<(), ExecError> {
    linker
        .func_wrap(
            "raddy",
            "raddy_head_len",
            |caller: Caller<'_, HostState>| caller.data().head.len() as i32,
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    linker
        .func_wrap(
            "raddy",
            "raddy_head_read",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             cap: i32|
             -> Result<i32, wasmtime::Error> {
                let src = caller.data().head.clone();
                write_guest(&mut caller, ptr, cap, &src)
            },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    linker
        .func_wrap(
            "raddy",
            "raddy_body_read",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             cap: i32|
             -> Result<i32, wasmtime::Error> { body_read(&mut caller, ptr, cap) },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    linker
        .func_wrap(
            "raddy",
            "raddy_resp_head",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             len: i32|
             -> Result<i32, wasmtime::Error> {
                let bytes = read_guest(&mut caller, ptr, len, MAX_RESP_HEAD_BYTES)?;
                let head = match JsonV1.decode_response(&bytes) {
                    Ok(head) => head,
                    Err(_) => return caller.data_mut().violate("invalid response head"),
                };
                if !(100..600).contains(&head.status) {
                    return caller.data_mut().violate("invalid response status");
                }
                caller.data_mut().apply(ProtoEvent::RespHead)?;
                if let Some(tx) = caller.data_mut().head_tx.take() {
                    let _ = tx.send(head);
                }
                Ok(0)
            },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    linker
        .func_wrap(
            "raddy",
            "raddy_resp_write",
            |mut caller: Caller<'_, HostState>,
             ptr: i32,
             len: i32|
             -> Result<i32, wasmtime::Error> {
                caller.data_mut().apply(ProtoEvent::RespWrite)?;
                let bytes = read_guest(&mut caller, ptr, len, MAX_RESP_CHUNK_BYTES)?;
                send_chunk(&mut caller, bytes)
            },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    linker
        .func_wrap(
            "raddy",
            "raddy_resp_end",
            |mut caller: Caller<'_, HostState>| {
                let state = caller.data_mut();
                state.apply(ProtoEvent::RespEnd)?;
                state.finish_body();
                Ok(0_i32)
            },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    Ok(())
}

fn body_read(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    cap: i32,
) -> Result<i32, wasmtime::Error> {
    if cap <= 0 {
        return Ok(0);
    }
    let want = (cap as usize).min(MAX_REQ_BODY_BYTES);
    let remaining = caller.data().remaining();
    if remaining.is_zero() {
        return caller
            .data_mut()
            .violate("deadline exceeded during body_read");
    }
    let runtime = caller.data().runtime_handle()?;
    let chunk = {
        let host = caller.data();
        let mut guard = host
            .body
            .lock()
            .map_err(|_| wasmtime::Error::msg("request body lock poisoned"))?;
        runtime.block_on(async {
            let mut buf = vec![0_u8; want];
            match tokio::time::timeout(remaining, guard.read(&mut buf)).await {
                Ok(Ok(n)) => {
                    buf.truncate(n);
                    Ok(buf)
                }
                Ok(Err(err)) => Err(wasmtime::Error::msg(err.to_string())),
                Err(_) => Err(wasmtime::Error::msg("deadline exceeded during body_read")),
            }
        })?
    };
    if caller.data().read_so_far.saturating_add(chunk.len()) > MAX_REQ_BODY_BYTES {
        return caller.data_mut().violate("request body exceeds maximum");
    }
    caller.data_mut().read_so_far += chunk.len();
    write_guest(caller, ptr, cap, &chunk)
}

fn send_chunk(caller: &mut Caller<'_, HostState>, bytes: Vec<u8>) -> Result<i32, wasmtime::Error> {
    let remaining = caller.data().remaining();
    if remaining.is_zero() {
        return caller
            .data_mut()
            .violate("deadline exceeded during resp_write");
    }
    let tx = caller
        .data()
        .body_tx
        .clone()
        .ok_or_else(|| wasmtime::Error::msg("response body already ended"))?;
    let runtime = caller.data().runtime_handle()?;
    match runtime
        .block_on(async { tokio::time::timeout(remaining, tx.send(Bytes::from(bytes))).await })
    {
        Ok(Ok(())) => Ok(0),
        Ok(Err(_)) => Ok(-1),
        Err(_) => caller
            .data_mut()
            .violate("deadline exceeded during resp_write"),
    }
}

fn memory(caller: &mut Caller<'_, HostState>) -> Result<Memory, wasmtime::Error> {
    caller
        .get_export("memory")
        .and_then(|ext| ext.into_memory())
        .ok_or_else(|| wasmtime::Error::msg("guest did not export memory"))
}

fn checked_range(
    mem: &Memory,
    caller: &Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    max: usize,
) -> Result<(usize, usize), wasmtime::Error> {
    if ptr < 0 || len < 0 {
        return Err(wasmtime::Error::msg("negative guest pointer or length"));
    }
    let len = len as usize;
    if len > max {
        return Err(wasmtime::Error::msg(
            "guest length exceeds protocol maximum",
        ));
    }
    let start = ptr as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| wasmtime::Error::msg("guest pointer overflow"))?;
    if end > mem.data_size(caller) {
        return Err(wasmtime::Error::msg("guest pointer out of bounds"));
    }
    Ok((start, len))
}

fn write_guest(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    cap: i32,
    src: &[u8],
) -> Result<i32, wasmtime::Error> {
    if ptr < 0 || cap < 0 {
        return Ok(-1);
    }
    let n = src.len().min(cap as usize);
    let mem = memory(caller)?;
    let (start, _) = match checked_range(&mem, caller, ptr, n as i32, usize::MAX) {
        Ok(range) => range,
        Err(_) => return Ok(-1),
    };
    mem.write(&mut *caller, start, &src[..n])
        .map_err(|err| wasmtime::Error::msg(err.to_string()))?;
    Ok(n as i32)
}

fn read_guest(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
    max: usize,
) -> Result<Vec<u8>, wasmtime::Error> {
    let mem = memory(caller)?;
    let (start, n) = match checked_range(&mem, caller, ptr, len, max) {
        Ok(range) => range,
        Err(err) => {
            let kind = if err.to_string().contains("exceeds protocol maximum") {
                "guest length exceeds protocol maximum"
            } else if err.to_string().contains("out of bounds") {
                "guest pointer out of bounds"
            } else if err.to_string().contains("overflow") {
                "guest pointer overflow"
            } else {
                "invalid guest pointer"
            };
            return caller.data_mut().violate(kind);
        }
    };
    let mut buf = vec![0_u8; n];
    mem.read(&mut *caller, start, &mut buf)
        .map_err(|err| wasmtime::Error::msg(err.to_string()))?;
    Ok(buf)
}
