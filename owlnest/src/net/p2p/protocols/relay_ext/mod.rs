use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    event_bus::{self, listened_event::Listenable,},
    with_timeout, single_value_filter,
};

pub mod behaviour;
pub mod cli;
mod handler;
pub mod in_event;

pub use behaviour::Behaviour;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutEvent {
    QueryAnswered { from: PeerId, list: Vec<PeerId> },
    StoppedProviding,
    StartedProviding,
    StartedAdvertising(PeerId),
    StoppedAdvertising(PeerId),
    Error(Error),
    Unsupported(PeerId),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
}
impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        format!("{}:OutEvent", protocol::PROTOCOL_NAME)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    ConnectionClosed,
    VerifierMismatch,
    NotProviding,
    Timeout,
    UnrecognizedMessage(String), // Serialzied not available on the original type
    IO(String),                  // Serialize not available on the original type
    Channel,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            ConnectionClosed => f.write_str("Connection Closed"),
            VerifierMismatch => f.write_str("Message verifier mismatch"),
            Timeout => f.write_str("Message timed out"),
            NotProviding => f.write_str(&format!("Relay is not providing")),
            UnrecognizedMessage(msg) => f.write_str(msg),
            IO(msg) => f.write_str(msg),
            Channel => f.write_str("Callback channel closed unexpectedly"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/relay_ext/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}

#[derive(Debug)]
pub enum InEvent {
    QueryAdvertisedPeer(PeerId),
    QueryProviderStatus,
    StartProviding,
    StopProviding,
    StartAdvertiseSelf(PeerId),
    StopAdvertiseSelf(PeerId),
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_bus_handle: event_bus::Handle,
}
impl Handle {
    pub fn new(
        buffer: usize,
        event_bus_handle: &crate::event_bus::Handle,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_bus_handle: event_bus_handle.clone(),
            },
            rx,
        )
    }
    pub async fn query_advertised_peer(&self, relay: PeerId) -> Result<Vec<PeerId>, Error> {
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .map_err(|_| Error::Channel)?;
        let handle = tokio::spawn(async move {
            while let Ok(v) = listener.recv().await {
                if let OutEvent::QueryAnswered { list, .. } =
                    v.downcast_ref::<OutEvent>().expect("downcast to succeed")
                {
                    return Ok(list.clone());
                }
            }
            Err(Error::Channel)
        });
        let ev = InEvent::QueryAdvertisedPeer(relay);
        self.sender.send(ev).await.unwrap();
        handle.await.expect("listen to succeed")
    }
    pub async fn start_providing(&self) {
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .expect("listener regsitration to succeed");
        let fut = single_value_filter!(listener::<OutEvent>, |v|{
            matches!(v, OutEvent::StartedProviding)
        });
        let ev = InEvent::StartProviding;
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10)
            .expect("request to finish in 10 seconds")
            .expect("listen to succeed");
    }
    pub async fn stop_providing(&self) {
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .expect("listener regsitration to succeed");
        let fut = single_value_filter!(listener::<OutEvent>, |v|{
            matches!(v, OutEvent::StoppedProviding)
        });
        let ev = InEvent::StopProviding;
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10)
            .expect("request to finish in 10 seconds")
            .expect("listen to succeed");
    }
    pub async fn provider_status(&self) -> bool {
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .expect("listener regsitration to succeed");
        let fut = single_value_filter!(listener::<OutEvent>,|v|{
            matches!(v, OutEvent::StartedProviding | OutEvent::StoppedProviding)
        });
        let ev = InEvent::QueryProviderStatus;
        self.sender.send(ev).await.expect("send to succeed");
        if let OutEvent::StartedProviding = with_timeout!(fut, 10)
            .expect("listen to succeed").unwrap()
        {
            return true;
        }
        false
    }
    // TODO: rewrite with macro
}
