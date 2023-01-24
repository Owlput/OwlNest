use super::*;

mod behaviour;
mod handler;
pub mod tether_ops;
pub mod callback_result;
pub mod error;

pub use behaviour::Behaviour;
pub use protocol::PROTOCOL_NAME;
pub use tether_ops::TetherOps;
pub use callback_result::CallbackResult;
pub use error::Error;

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
pub struct InEvent {
    to: Option<PeerId>,
    op: TetherOps,
    callback:oneshot::Sender<CallbackResult>
}
impl InEvent{
    pub fn new(to:Option<PeerId>,op:TetherOps,callback:oneshot::Sender<CallbackResult>)->Self{
        Self { to, op,callback }
    }
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingOp { from: PeerId, inner: TetherOps },
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

pub async fn ev_dispatch(ev: OutEvent, _dispatch: &mpsc::Sender<OutEvent>) {

    match ev {
        OutEvent::IncomingOp { .. } => {
            println!("Incoming message: {:?}", ev);
            // match dispatch.send(ev).await {
            //     Ok(_) => {}
            //     Err(e) => println!("Failed to send message with error {}", e),
            // };
        }
        OutEvent::Error(e) => println!("{:#?}", e),
        OutEvent::Unsupported(peer) => {
            println!("Peer {} doesn't support /owlput/tethering/0.0.1", peer)
        }
        OutEvent::InboundNegotiated(peer) => println!(
            "Successfully negotiated inbound connection from peer {}",
            peer
        ),
        OutEvent::OutboundNegotiated(peer) => println!(
            "Successfully negotiated outbound connection to peer {}",
            peer
        ),
    }
}

mod protocol{
    pub const PROTOCOL_NAME:&[u8] = b"/owlput/tethering/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv,send};
}