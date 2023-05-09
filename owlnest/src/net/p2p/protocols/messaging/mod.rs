use crate::event_bus::listener_event::{BehaviourEvent, ListenedEvent};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

mod behaviour;
pub mod cli;
mod config;
mod error;
pub mod event_listener;
pub(crate) mod handler;
mod message;

pub use behaviour::Behaviour;
pub use config::Config;
pub use error::Error;
pub use message::Message;

use crate::net::p2p::swarm::BehaviourOpResult;

#[derive(Debug)]
pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<BehaviourOpResult>,
}
impl InEvent {
    pub fn new(op: Op, callback: oneshot::Sender<BehaviourOpResult>) -> Self {
        Self { op, callback }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    SendMessage(PeerId, Message),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OpResult {
    SuccessfulPost(Duration),
    Error(Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}
impl Into<ListenedEvent> for OutEvent {
    fn into(self) -> ListenedEvent {
        ListenedEvent::Behaviours(BehaviourEvent::Messaging(self))
    }
}

pub fn ev_dispatch(ev: &OutEvent) {
    match ev {
        OutEvent::IncomingMessage { .. } => {
            println!("Incoming message: {:?}", ev);
            // match dispatch.send(ev).await {
            //     Ok(_) => {}
            //     Err(e) => println!("Failed to send message with error {}", e),
            // };
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
    pub const PROTOCOL_NAME: &'static str = "/owlnest/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}
