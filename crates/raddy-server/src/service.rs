use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use http::{Request, Response, StatusCode};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use raddy_executor::{ExecError, ExecRequest, Executor};
use tokio::net::{TcpListener, TcpStream};
use tracing::{Instrument, info, info_span};

use crate::admit::{Admission, RespBody, empty, status_response};
use crate::envelope::envelope_from_http;
use crate::io::{GuestBody, IncomingRead};

/// Timeouts and admission for one listener.
#[derive(Clone, Debug)]
pub struct ServerLimits {
    pub concurrency: usize,
    pub request_timeout: Duration,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            concurrency: 256,
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Failures binding or accepting.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("listen {addr}: {source}")]
    Listen {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("accept: {0}")]
    Accept(#[from] std::io::Error),
}

/// Bind `addr` and serve until `shutdown` resolves, then drain in-flight work.
pub async fn serve<E>(
    addr: SocketAddr,
    exec: E,
    limits: ServerLimits,
    shutdown: impl Future<Output = ()>,
) -> Result<(), ServerError>
where
    E: Executor + Clone,
{
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| ServerError::Listen { addr, source })?;
    serve_tcp(listener, exec, limits, shutdown).await
}

/// Serve on an already-bound listener (ephemeral ports in tests).
pub async fn serve_tcp<E>(
    listener: TcpListener,
    exec: E,
    limits: ServerLimits,
    shutdown: impl Future<Output = ()>,
) -> Result<(), ServerError>
where
    E: Executor + Clone,
{
    let admit = Admission::new(limits.concurrency);
    let timeout = limits.request_timeout;
    let mut shutdown = std::pin::pin!(shutdown);
    let mut in_flight = tokio::task::JoinSet::new();

    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => break,
            joined = in_flight.join_next(), if !in_flight.is_empty() => {
                let _ = joined;
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let exec = exec.clone();
                let admit = admit.clone();
                in_flight.spawn(async move {
                    serve_one(stream, peer, exec, admit, timeout).await;
                });
            }
        }
    }

    let _ = tokio::time::timeout(timeout, async {
        while in_flight.join_next().await.is_some() {}
    })
    .await;
    Ok(())
}

async fn serve_one<E>(
    stream: TcpStream,
    peer: SocketAddr,
    exec: E,
    admit: Admission,
    timeout: Duration,
) where
    E: Executor + Clone,
{
    let io = TokioIo::new(stream);
    let service = service_fn(move |req: Request<Incoming>| {
        let exec = exec.clone();
        let admit = admit.clone();
        async move { Ok::<_, Infallible>(dispatch(req, exec, peer, admit, timeout).await) }
    });
    let _ = http1::Builder::new().serve_connection(io, service).await;
}

async fn dispatch<E>(
    req: Request<Incoming>,
    exec: E,
    peer: SocketAddr,
    admit: Admission,
    timeout: Duration,
) -> Response<RespBody>
where
    E: Executor,
{
    let method = req.method().clone();
    let uri = req.uri().clone();
    let span = info_span!("request", %method, target = %uri);
    async move {
        match tokio::time::timeout(timeout, respond(req, exec, peer, admit, timeout)).await {
            Ok(resp) => resp,
            Err(_) => status_response(StatusCode::GATEWAY_TIMEOUT),
        }
    }
    .instrument(span)
    .await
}

async fn respond<E>(
    req: Request<Incoming>,
    exec: E,
    peer: SocketAddr,
    admit: Admission,
    timeout: Duration,
) -> Response<RespBody>
where
    E: Executor,
{
    let Some(_permit) = admit.try_enter() else {
        return status_response(StatusCode::SERVICE_UNAVAILABLE);
    };

    let (parts, incoming) = req.into_parts();
    let head = envelope_from_http(
        &parts.method,
        &parts.uri,
        &parts.headers,
        peer,
        timeout.as_millis() as u64,
    );
    info!(request_id = %head.request_id, "admit");
    let exec_req = ExecRequest {
        head,
        body: Box::new(IncomingRead::new(incoming)),
    };
    match exec.execute(exec_req).await {
        Ok(out) => guest_response(out),
        Err(ExecError::Saturated) => status_response(StatusCode::SERVICE_UNAVAILABLE),
        Err(ExecError::DeadlinePreHead) => status_response(StatusCode::GATEWAY_TIMEOUT),
        Err(ExecError::Trap(_) | ExecError::Protocol(_) | ExecError::Artifact(_)) => {
            status_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn guest_response(out: raddy_executor::ExecResponse) -> Response<RespBody> {
    let mut builder = Response::builder().status(out.head.status);
    for (name, value) in &out.head.headers {
        if let (Ok(n), Ok(v)) = (
            http::HeaderName::from_bytes(name.as_bytes()),
            http::HeaderValue::from_str(value),
        ) {
            builder = builder.header(n, v);
        }
    }
    let body = http_body_util::BodyExt::boxed(http_body_util::BodyExt::map_err(
        GuestBody::new(out.body, out.done),
        std::convert::identity,
    ));
    builder.body(body).unwrap_or_else(|_| {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(empty())
            .expect("static 500")
    })
}
