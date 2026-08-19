//! Hyper ingress: socket bytes to the [`raddy_executor::Executor`] port.

mod admit;
mod envelope;
mod io;
mod service;

pub use envelope::envelope_from_http;
pub use service::{ServerError, ServerLimits, serve, serve_tcp};
