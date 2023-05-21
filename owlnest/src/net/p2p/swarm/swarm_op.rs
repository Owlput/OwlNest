use libp2p::{
    swarm::{derive_prelude::ListenerId, AddressRecord, DialError},
    Multiaddr, PeerId, TransportError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Dial(Multiaddr),
    Listen(Multiaddr),
    AddExternalAddress(Multiaddr, Option<u32>),
    RemoveExternalAddress(Multiaddr),
    DisconnectFromPeerId(PeerId),
    ListExternalAddresses,
    ListListeners,
    IsConnectedToPeerId(PeerId),
}

#[derive(Debug)]
pub enum OpResult {
    Dial(Result<(), DialError>),
    Listen(Result<ListenerId, TransportError<std::io::Error>>),
    AddExternalAddress(AddExternalAddressResult),
    RemoveExternalAddress(bool),
    DisconnectFromPeerId(Result<(), ()>),
    ListExternalAddresses(Vec<AddressRecord>),
    ListListeners(Vec<Multiaddr>),
    IsConnectedToPeerId(bool),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AddExternalAddressResult {
    Inserted,
    Updated,
}
