use crate::{
    handle_listener_result,
    net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent}
};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tracing::{debug, trace, warn};

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
pub use protocol::PROTOCOL_NAME;

#[derive(Debug)]
pub enum InEvent {
    SendMessage(PeerId, Message, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    SuccessfulSend(u64),
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

pub fn ev_dispatch(ev: &OutEvent) {
    use OutEvent::*;
    match ev {
        IncomingMessage { .. } => {
            println!("Incoming message: {:?}\n", ev);
        }
        Error(e) => debug!("{:#?}", e),
        Unsupported(peer) => {
            trace!("Peer {} doesn't support /owlput/messaging/0.0.1", peer)
        }
        InboundNegotiated(peer) => trace!(
            "Successfully negotiated inbound connection from peer {}",
            peer
        ),
        OutboundNegotiated(peer) => trace!(
            "Successfully negotiated outbound connection to peer {}",
            peer
        ),
        SuccessfulSend(_) => {}
    }
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}

use tokio::sync::mpsc;

use crate::with_timeout;
#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    counter: Arc<AtomicU64>,
}
impl Handle {
    pub fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx:event_tx.clone(),
                counter: Arc::new(AtomicU64::new(1)),
            },
            rx,
        )
    }
    pub async fn send_message(&self, peer_id: PeerId, message: Message) -> Result<(), Error> {
        let op_id = self.counter.fetch_add(1, Ordering::SeqCst);
        let ev = InEvent::SendMessage(peer_id, message, op_id);
        let listener = self.event_tx.subscribe();
        self.sender.send(ev).await.expect("send to succeed");
        let fut = async move {
            let mut listener = listener;
            loop {
                let ev = handle_listener_result!(listener);
                if let SwarmEvent::Behaviour(BehaviourEvent::Messaging(ev)) = ev.as_ref() {
                    match ev {
                        OutEvent::Error(e) => {
                            if let Error::PeerNotFound(peer) = e {
                                if *peer == peer_id {
                                    return Err(e.clone());
                                }
                            }
                        }
                        OutEvent::SuccessfulSend(id) if *id == op_id => return Ok(()),
                        _ => {}
                    }
                    continue;
                }
            }
        };
        match with_timeout!(fut, 10) {
            Ok(v) => v,
            Err(_) => {
                warn!("a timeout reached for a timed future");
                Err(Error::Timeout)
            }
        }
    }
}
