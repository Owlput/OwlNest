use super::*;

mod behaviour;
pub mod error;
mod handler;
pub mod op_result;
pub mod push_handles;

/// Protocol `/owlnest/tethering` is divided into two subprotocols.
/// `/owlnest/tethering/exec` for operation execution,
/// `/owlnest/tethering/push` for notification pushing.
/// Both subprotocols will perform a handshake in TCP style(aka three-way handshake).
pub mod subprotocols;

pub use behaviour::Behaviour;
pub use error::Error;
pub use op_result::{LocalOpResult, OpResult};
pub use protocol::PROTOCOL_NAME;
pub use tether_ops::{Op, PushEvent, RemoteOp};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct Config {
    timeout: Duration,
}
impl Config {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(60),
        }
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum InEvent {
    LocalExec(Op, oneshot::Sender<OpResult>),
    RemoteExec(PeerId, Op, oneshot::Sender<OpResult>),
    Push(PeerId, PushEvent, oneshot::Sender<OpResult>),
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingOp(RemoteOp),
    IncomingPush(PushEvent),
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

pub async fn ev_dispatch(ev: OutEvent, _dispatch: &mpsc::Sender<OutEvent>) {
    match ev {
        OutEvent::IncomingOp { .. } => {
            println!("Incoming message: {:?}", ev);
        }
        OutEvent::IncomingPush { .. } => {
            println!("Incoming message: {:?}", ev);
        }
        OutEvent::Error(e) => warn!("{:#?}", e),
        OutEvent::Unsupported(peer) => {
            info!("Peer {} doesn't support /owlput/tethering/0.0.1", peer)
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
    pub const PROTOCOL_NAME: &[u8] = b"/owlput/tethering/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}
