use hyper::{Request, Response};
use owlnest_prelude::lib_prelude::*;
use tokio::sync::oneshot;
use tokio_util::bytes::Bytes;
use tracing::trace;

mod behaviour;
mod config;
mod error;
mod handler;

pub use behaviour::Behaviour;
pub use config::Config;
pub use error::Error;
pub use protocol::PROTOCOL_NAME;

#[derive(Debug)]
pub enum InEvent {
    SendRequest(PeerId, Request<String>, oneshot::Sender<Response<Bytes>>),
}

#[derive(Debug)]
pub enum OutEvent {
    Response { inner: Bytes, ticket: u64 },
    Error(Error),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
    Unsupported(PeerId),
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/hyper/0.0.1";
}
