use crate::net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{trace, warn};

mod behaviour;
pub(crate) mod cli;
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

#[derive(Debug)]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    SendResult(Result<Duration, SendError>, u64),
    Error(Error),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
    Unsupported(PeerId),
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}

use tokio::sync::mpsc;

use crate::with_timeout;
macro_rules! event_op {
    ($listener:ident,$pattern:pat,{$($ops:tt)+}) => {
        async move{
        loop{
            let ev = crate::handle_listener_result!($listener);
            if let SwarmEvent::Behaviour(BehaviourEvent::Messaging($pattern)) = ev.as_ref() {
                $($ops)+
            } else {
                continue;
            }
        }}
    };
}
use self::error::SendError;
use std::sync::atomic::{AtomicU64, Ordering};
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
                event_tx: event_tx.clone(),
                counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    pub async fn send_message(
        &self,
        peer_id: PeerId,
        message: Message,
    ) -> Result<Duration, SendError> {
        let op_id = self.next_id();
        let ev = InEvent::SendMessage(peer_id, message, op_id);
        let mut listener = self.event_tx.subscribe();
        let fut = event_op!(listener, OutEvent::SendResult(result, id), {
            if *id != op_id {
                continue;
            }
            return result.clone();
        });
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(fut, 10) {
            Ok(v) => v,
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(SendError::Timeout)
            }
        }
    }
    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}
