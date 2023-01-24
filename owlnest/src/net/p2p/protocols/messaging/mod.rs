use super::*;
use std::{string::FromUtf8Error, time::SystemTime};

pub mod behaviour;
mod handler;
pub use behaviour::Behaviour;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    time: u128,
    from: PeerId,
    to: PeerId,
    msg: String,
}
impl Message {
    pub fn new(from: &PeerId, to: &PeerId, msg: String) -> Self {
        Self {
            time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            from: from.clone(),
            to: to.clone(),
            msg,
        }
    }
    #[inline]
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

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
    PostMessage(PeerId,Message,oneshot::Sender<CallbackResult>),
}

#[derive(Debug)]
pub enum CallbackResult{
    SuccessfulPost(Duration),
    Error(Error)
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

#[derive(Debug)]
pub enum Error {
    ConnectionClosed,
    VerifierMismatch,
    Timeout,
    UnrecognizedMessage(serde_json::Error, Result<String, FromUtf8Error>),
    IO(std::io::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionClosed => f.write_str("Connection Closed"),
            Self::VerifierMismatch => {
                f.write_str("Message verifier mismatch")
            }
            Self::Timeout => f.write_str("Message timed out"),
            Self::UnrecognizedMessage(e, broken_msg) => f.write_str(&format!(
                "Failed to deserialize message with error: {}, possible raw data: {:?}",
                e, broken_msg
            )),
            Self::IO(e) => f.write_str(&format!("IO error: {}", e)),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IO(e) => e.source(),
            _ => None,
        }
    }
}

pub async fn ev_dispatch(ev: OutEvent, _dispatch: &mpsc::Sender<OutEvent>) {

    match ev {
        OutEvent::IncomingMessage { .. } => {
            println!("Incoming message: {:?}", ev);
            // match dispatch.send(ev).await {
            //     Ok(_) => {}
            //     Err(e) => println!("Failed to send message with error {}", e),
            // };
        }
        OutEvent::Error(e) => println!("{:#?}", e),
        OutEvent::Unsupported(peer) => {
            println!("Peer {} doesn't support /owlput/messaging/0.0.1", peer)
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
    pub const PROTOCOL_NAME: &[u8] = b"/owlput/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{send,recv};
}