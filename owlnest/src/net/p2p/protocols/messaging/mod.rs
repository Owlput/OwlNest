use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, string::FromUtf8Error, time::Duration};

pub mod behaviour;
mod handler;
#[allow(dead_code)]
mod inbox;
mod protocol;

pub use behaviour::Behaviour;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    time: u128,
    from: PeerId,
    to: PeerId,
    msg: String,
}
impl Message {
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

pub enum InEvent {
    PostMessage(Message),
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    SuccessPost(PeerId, u128, Duration),
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}

#[derive(Debug)]
pub enum Error {
    ConnectionClosed,
    VerifierMismatch(u128),
    Timeout(u128),
    DroppedMessage(u128),
    UnrecognizedMessage(serde_json::Error, Result<String, FromUtf8Error>),
    IO(std::io::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionClosed => f.write_str("Connection Closed"),
            Self::VerifierMismatch(stamp) => {
                f.write_str(&format!("Message verifier mismatch with stamp: {}", stamp))
            }
            Self::Timeout(stamp) => f.write_str(&format!("Message timeout for stamp: {}", stamp)),
            Self::DroppedMessage(stamp) => {
                f.write_str(&format!("Message dropped with stamp: {}", stamp))
            }
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
