use super::*;
use libp2p::{swarm::{DialError, derive_prelude::ListenerId}, TransportError};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum InEvent {
    Dial(Multiaddr,oneshot::Sender<Result<(),DialError>>),
    Listen(Multiaddr,oneshot::Sender<Result<ListenerId,TransportError<std::io::Error>>>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Dial(Multiaddr),
    Listen(Multiaddr),
    AddExternalAddress(Multiaddr, String), // Should be AddressScore but it can't be serialzied, which is disappointing
}

#[derive(Debug)]
pub enum ProtocolInEvent{
    #[cfg(feature="messaging")]
    Messaging(messaging::InEvent),
    #[cfg(feature="tethering")]
    Tethering(tethering::InEvent)
}