use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    handle_listener_result,
    net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent},
    with_timeout,
};

pub mod behaviour;
pub mod cli;
mod handler;
pub mod in_event;

pub use behaviour::Behaviour;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutEvent {
    QueryAnswered { from: PeerId, list: Vec<PeerId> },
    ProviderState(bool),
    Error(Error),
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
    SetProviderState(bool),
    QueryProviderState,
    QueryAdvertisedPeer(PeerId),
    SetAdvertisingSelf { remote: PeerId, state: bool },
}

macro_rules! event_op {
    ($listener:ident,$pattern:pat,{$($ops:tt)+}) => {
        loop{
            let ev = crate::handle_listener_result!($listener);
            if let SwarmEvent::Behaviour(BehaviourEvent::RelayExt($pattern)) = ev.as_ref() {
                $($ops)+
            } else {
                continue;
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
}
impl Handle {
    pub fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
            },
            rx,
        )
    }
    pub async fn query_advertised_peer(&self, relay: PeerId) -> Result<Vec<PeerId>, Error> {
        let mut listener = self.event_tx.subscribe();
        let fut = async move{
            event_op!(listener, OutEvent::QueryAnswered { from, list }, {
                if *from == relay {
                    return list.clone();
                }
            })
        };
        let handle = tokio::spawn(fut);
        let ev = InEvent::QueryAdvertisedPeer(relay);
        self.sender.send(ev).await.unwrap();
        match with_timeout!(handle,10){
            Ok(v)=> Ok(v.expect("task to complete")),
            Err(_)=> Err(Error::Timeout)
        }
    }
    pub async fn set_provider_state(&self, state: bool) -> bool {
        let mut listener = self.event_tx.subscribe();
        let fut = async move {
            loop {
                let ev = handle_listener_result!(listener);
                if let SwarmEvent::Behaviour(BehaviourEvent::RelayExt(OutEvent::ProviderState(
                    status,
                ))) = ev.as_ref()
                {
                    return *status;
                }
            }
        };
        let ev = InEvent::SetProviderState(state);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    pub async fn provider_status(&self) -> bool {
        let mut listener = self.event_tx.subscribe();
        let fut = async move {
            loop {
                if let SwarmEvent::Behaviour(BehaviourEvent::RelayExt(OutEvent::ProviderState(
                    state,
                ))) = handle_listener_result!(listener).as_ref()
                {
                    return *state;
                };
            }
        };
        let ev = InEvent::QueryProviderState;
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
}
