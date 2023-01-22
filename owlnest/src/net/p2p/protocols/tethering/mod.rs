use super::*;
use std::string::FromUtf8Error;

pub use behaviour::Behaviour;
pub use protocol::PROTOCOL_NAME;
pub use tether_ops::TetherOps;


mod behaviour;
mod handler;
mod protocol;
pub mod tether_ops;

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
    to: PeerId,
    inner: TetherOps,
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingOp { from: PeerId, inner: TetherOps },
    SuccessPost(PeerId, u128, Duration),
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

#[derive(Debug)]
pub enum Error {
    ConnectionClosed,
    StampMismatch(u128),
    Timeout(u128),
    UnrecognizedOp(serde_json::Error, Result<String, FromUtf8Error>),
    IO(std::io::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionClosed => f.write_str("Connection Closed"),
            Self::StampMismatch(stamp) => {
                f.write_str(&format!("Message verifier mismatch with stamp: {}", stamp))
            }
            Self::Timeout(stamp) => f.write_str(&format!("Message timeout for stamp: {}", stamp)),
            Self::UnrecognizedOp(e, broken_op) => f.write_str(&format!(
                "Failed to deserialize message with error: {}, possible raw data: {:?}",
                e, broken_op
            )),
            Self::IO(e) => f.write_str(&format!("IO error: {}", e)),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
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
        OutEvent::SuccessPost(peer, _, rtt) => println!(
            "Successful posted message to peer {}, estimated roundtrip time {}ms",
            peer,
            rtt.as_millis()
        ),
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
