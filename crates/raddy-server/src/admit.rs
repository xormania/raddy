use std::sync::Arc;

use bytes::Bytes;
use http::{Response, StatusCode};
use http_body_util::BodyExt;
use http_body_util::{Empty, combinators::BoxBody};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub type RespBody = BoxBody<Bytes, std::io::Error>;

#[derive(Clone, Debug)]
pub struct Admission {
    sem: Arc<Semaphore>,
}

impl Admission {
    #[must_use]
    pub fn new(permits: usize) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(permits.max(1))),
        }
    }

    pub fn try_enter(&self) -> Option<OwnedSemaphorePermit> {
        self.sem.clone().try_acquire_owned().ok()
    }
}

pub fn status_response(status: StatusCode) -> Response<RespBody> {
    let mut builder = Response::builder().status(status);
    if status == StatusCode::SERVICE_UNAVAILABLE {
        builder = builder.header(http::header::RETRY_AFTER, "1");
    }
    builder.body(empty()).expect("static status response")
}

pub fn empty() -> RespBody {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
