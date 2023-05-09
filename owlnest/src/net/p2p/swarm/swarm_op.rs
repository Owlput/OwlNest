use libp2p::{swarm::{DialError, derive_prelude::ListenerId, AddressRecord}, PeerId, Multiaddr, TransportError};
use serde::{Serialize,Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Dial(Multiaddr),
    Listen(Multiaddr),
    AddExternalAddress(Multiaddr, Option<u32>),
    RemoveExternalAddress(Multiaddr),
    // BanByPeerId(PeerId),
    // UnbanByPeerId(PeerId),
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
    // BanByPeerId,
    // UnbanByPeerId,
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