
use libp2p::{Multiaddr, PeerId, TransportError};
use libp2p_swarm::{derive_prelude::ListenerId, DialError};
use tokio::sync::oneshot::*;

#[derive(Debug)]
pub enum InEvent {
    Dial(Multiaddr, Sender<Result<(), DialError>>),
    Listen(
        Multiaddr,
        Sender<Result<ListenerId, TransportError<std::io::Error>>>,
    ),
    AddExternalAddress(Multiaddr, Sender<()>),
    RemoveExternalAddress(Multiaddr, Sender<()>),
    DisconnectFromPeerId(PeerId, Sender<Result<(), ()>>),
    ListExternalAddresses(Sender<Vec<Multiaddr>>),
    ListListeners(Sender<Vec<Multiaddr>>),
    IsConnectedToPeerId(PeerId, Sender<bool>),
}
