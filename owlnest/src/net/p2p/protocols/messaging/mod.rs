use libp2p::PeerId;
use owlnest_proc::generate_kind;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

mod behaviour;
mod cli;
mod config;
mod error;
mod handler;
mod message;
mod op;

pub use behaviour::Behaviour;
pub(crate) use cli::handle_messaging;
pub use config::Config;
pub use error::Error;
pub use message::Message;
pub use op::{Op, OpResult};
pub use protocol::PROTOCOL_NAME;

#[derive(Debug)]
pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<OpResult>,
}
impl InEvent {
    pub fn new(op: Op) -> (Self, oneshot::Receiver<OpResult>) {
        let (tx, rx) = oneshot::channel();
        (Self { op, callback: tx }, rx)
    }
}

const EVENT_IDENT: &str = PROTOCOL_NAME;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[generate_kind]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

pub fn ev_dispatch(ev: &OutEvent) {
    match ev {
        OutEvent::IncomingMessage { .. } => {
            println!("Incoming message: {:?}", ev);
        }
        OutEvent::Error(e) => warn!("{:#?}", e),
        OutEvent::Unsupported(peer) => {
            info!("Peer {} doesn't support /owlput/messaging/0.0.1", peer)
        }
        OutEvent::InboundNegotiated(peer) => debug!(
            "Successfully negotiated inbound connection from peer {}",
            peer
        ),
        OutEvent::OutboundNegotiated(peer) => debug!(
            "Successfully negotiated outbound connection to peer {}",
            peer
        ),
    }
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}

use tokio::sync::{mpsc, oneshot};
#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
}
impl Handle {
    pub fn new(buffer: usize) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self { sender: tx }, rx)
    }
    pub async fn send_message(&self, peer_id: PeerId, message: Message) -> Result<Duration, Error> {
        let (ev, rx) = InEvent::new(Op::SendMessage(peer_id, message));
        if let Err(_) = self.sender.send(ev).await {
            return Err(Error::Channel);
        }
        match rx.await {
            Ok(result) => match result {
                OpResult::Error(e) => Err(e),
                OpResult::SuccessfulPost(rtt) => Ok(rtt),
            },
            Err(_) => Err(Error::Channel),
        }
    }
    pub fn blocking_send_message(&self, peer_id: PeerId, message: Message) -> Result<Duration, Error> {
        let (ev, rx) = InEvent::new(Op::SendMessage(peer_id, message));
        if let Err(_) = self.sender.blocking_send(ev) {
            return Err(Error::Channel);
        }
        match rx.blocking_recv() {
            Ok(result) => match result {
                OpResult::Error(e) => Err(e),
                OpResult::SuccessfulPost(rtt) => Ok(rtt),
            },
            Err(_) => Err(Error::Channel),
        }
    }
}
