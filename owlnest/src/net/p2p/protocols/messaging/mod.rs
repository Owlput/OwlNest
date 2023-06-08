use libp2p::PeerId;
use owlnest_proc::generate_kind;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};
use crate::net::p2p::swarm::{op::behaviour::CallbackSender,self};

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
    callback: CallbackSender,
}
impl InEvent {
    pub fn new(op: Op, callback: CallbackSender) -> Self {
        Self { op, callback }
    }
}
impl From<InEvent> for swarm::in_event::behaviour::InEvent{
    fn from(value: InEvent) -> Self {
        Self::Messaging(value)
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
    pub const PROTOCOL_NAME: &str = "/owlnest/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}
