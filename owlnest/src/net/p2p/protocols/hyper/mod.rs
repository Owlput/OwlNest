use crate::{generate_handler_method, net::p2p::swarm::EventSender};
use hyper::{Request, Response};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tokio_util::bytes::Bytes;
use tracing::trace;

mod behaviour;
mod config;
mod error;
mod handler;
mod op;

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

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    counter: Arc<AtomicU64>,
}
#[allow(unused)]
impl Handle {
    pub fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
                counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    generate_handler_method!(SendRequest:send_request(peer:PeerId,request:Request<String>)->Response<Bytes>;);
    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}
