use crate::net::p2p::swarm::InEvent;
use libp2p::{
    swarm::{derive_prelude::ListenerId, DialError},
    Multiaddr, PeerId, TransportError,
};
use owlnest_macro::{generate_handler_method, generate_handler_method_blocking};
use tokio::sync::mpsc;

/// The handle to the swarm.  
/// This handle may be used to manage connections,
/// but not `NetworkBehaviour`s.
#[derive(Debug, Clone)]
pub struct SwarmHandle {
    sender: mpsc::Sender<InEvent>,
}
impl SwarmHandle {
    pub(crate) fn new(buffer: usize) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self { sender: tx }, rx)
    }
    generate_handler_method_blocking!(
        /// Dial the address.
        /// Should be used in synchronous contexts.
        Dial:dial_blocking(addr: Multiaddr) -> Result<(), DialError>;
        /// Listion on the address.
        /// Should be used in synchronous contexts.
        Listen:listen_blocking(addr: Multiaddr) -> Result<ListenerId, TransportError<std::io::Error>>;
        /// Manually confirm the address to be publicly reachable.
        /// Should be used in sychronous contexts
        AddExternalAddress:add_external_address_blocking(addr:Multiaddr)->();
        /// Check if the local peer is connected to a remote peer.
        /// Should be used in sychronous contexts
        IsConnectedToPeerId:is_connected_blocking(peer_id: PeerId) -> bool;
        /// List all active listeners.
        /// Should be used in sychronous contexts
        ListListeners:list_listeners_blocking()->Box<[Multiaddr]>;
        /// List all publicly reachable listen addresses.
        /// Should be used in sychronous contexts
        ListExternalAddresses:list_external_addresses_blocking()->Box<[Multiaddr]>;
        /// Disconnect from the peer.
        /// Should be used in sychronous contexts
        DisconnectFromPeerId:disconnect_peer_id_blocking(peer_id:PeerId)->Result<(),()>;
        /// Remove an address from all known publicly reachable addresses.
        /// Should be used in sychronous contexts
        RemoveExternalAddress:remove_external_address_blocking(addr:Multiaddr)->();
    );
    generate_handler_method!(
        /// Dial the address.
        /// Should be used in asynchronous contexts.
        Dial:dial(addr: Multiaddr) -> Result<(), DialError>;
        /// Listion on the address.
        /// Should be used in asynchronous contexts.
        Listen:listen(addr: Multiaddr) -> Result<ListenerId, TransportError<std::io::Error>>;
        /// Manually confirm the address to be publicly reachable.
        /// Should be used in asychronous contexts
        AddExternalAddress:add_external_address(addr:Multiaddr)->();
        /// Check if the local peer is connected to a remote peer.
        /// Should be used in asychronous contexts
        IsConnectedToPeerId:is_connected(peer_id: PeerId) -> bool;
        /// List all active listeners.
        /// Should be used in asychronous contexts
        ListListeners:list_listeners()->Box<[Multiaddr]>;
        /// List all publicly reachable listen addresses.
        /// Should be used in asychronous contexts
        ListExternalAddresses:list_external_addresses()->Box<[Multiaddr]>;
        /// Disconnect from the peer.
        /// Should be used in asychronous contexts
        DisconnectFromPeerId:disconnect_peer_id(peer_id:PeerId)->Result<(),()>;
        /// Remove an address from all known publicly reachable addresses.
        /// Should be used in asychronous contexts
        RemoveExternalAddress:remove_external_address(addr:Multiaddr)->();
    );
}
