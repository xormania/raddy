use bytes::Bytes;
use raddy_abi::{HeadCodec, JsonV1, ResponseHead};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use wasmtime::{Caller, Linker, Memory};

use crate::ExecError;
use crate::proto::{ProtoEvent, Protocol};

pub(crate) struct HostState {
    head: Vec<u8>,
    body: Vec<u8>,
    body_off: usize,
    proto: Protocol,
    head_tx: Option<oneshot::Sender<ResponseHead>>,
    body_tx: mpsc::Sender<Bytes>,
    fail: Option<ExecError>,
}

impl HostState {
    pub(crate) fn new(
        head: Vec<u8>,
        body: Vec<u8>,
        head_tx: oneshot::Sender<ResponseHead>,
        body_tx: mpsc::Sender<Bytes>,
    ) -> Self {
        Self {
            head,
            body,
            body_off: 0,
            proto: Protocol::AwaitHead,
            head_tx: Some(head_tx),
            body_tx,
            fail: None,
        }
    }

    pub(crate) fn take_fail(&mut self) -> Option<ExecError> {
        self.fail.take()
    }

    fn violate(&mut self, kind: &'static str) -> Result<(), wasmtime::Error> {
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
             -> Result<i32, wasmtime::Error> {
                if cap <= 0 {
                    return Ok(0);
                }
                let (chunk, new_off) = {
                    let host = caller.data();
                    let available = host.body.len().saturating_sub(host.body_off);
                    let n = available.min(cap as usize);
                    let chunk = host.body[host.body_off..host.body_off + n].to_vec();
                    (chunk, host.body_off + n)
                };
                let wrote = write_guest(&mut caller, ptr, cap, &chunk)?;
                caller.data_mut().body_off = new_off;
                Ok(wrote)
            },
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
                caller.data_mut().apply(ProtoEvent::RespHead)?;
                let bytes = read_guest(&mut caller, ptr, len)?;
                let head = JsonV1
                    .decode_response(&bytes)
                    .map_err(|err| wasmtime::Error::msg(err.to_string()))?;
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
                let bytes = read_guest(&mut caller, ptr, len)?;
                let tx = caller.data().body_tx.clone();
                if tx.blocking_send(Bytes::from(bytes)).is_err() {
                    return Ok(-1);
                }
                Ok(0)
            },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    linker
        .func_wrap(
            "raddy",
            "raddy_resp_end",
            |mut caller: Caller<'_, HostState>| {
                caller.data_mut().apply(ProtoEvent::RespEnd)?;
                Ok(0_i32)
            },
        )
        .map_err(|err| ExecError::Artifact(err.to_string()))?;

    Ok(())
}

fn memory(caller: &mut Caller<'_, HostState>) -> Result<Memory, wasmtime::Error> {
    caller
        .get_export("memory")
        .and_then(|ext| ext.into_memory())
        .ok_or_else(|| wasmtime::Error::msg("guest did not export memory"))
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
    mem.write(&mut *caller, ptr as usize, &src[..n])
        .map_err(|err| wasmtime::Error::msg(err.to_string()))?;
    Ok(n as i32)
}

fn read_guest(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, wasmtime::Error> {
    if ptr < 0 || len < 0 {
        return Err(wasmtime::Error::msg("negative guest pointer or length"));
    }
    let mut buf = vec![0_u8; len as usize];
    let mem = memory(caller)?;
    mem.read(&mut *caller, ptr as usize, &mut buf)
        .map_err(|err| wasmtime::Error::msg(err.to_string()))?;
    Ok(buf)
}
