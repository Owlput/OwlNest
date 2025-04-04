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
        Dial:dial_blocking(address: <&Multiaddr>) -> Result<(), DialError>;

        /// Listion on the address.
        /// Should be used in synchronous contexts.
        Listen:listen_blocking(address: <&Multiaddr>) -> Result<ListenerId, TransportError<std::io::Error>>;
        /// List all active listeners.
        /// Should be used in synchronous contexts.
        ListListeners:list_listeners_blocking()->Box<[Multiaddr]>;
        /// Remove a listener.
        /// Should be used in synchronous contexts.
        RemoveListeners:remove_listener_blocking(listener_id: &ListenerId)->bool;

        /// Manually confirm the address to be publicly reachable.
        /// Should be used in synchronous contexts.
        AddExternalAddress:add_external_address_blocking(address: <&Multiaddr>)->();
        /// List all publicly reachable listen addresses.
        /// Should be used in synchronous contexts.
        ListExternalAddresses:list_external_addresses_blocking()->Box<[Multiaddr]>;
        /// Remove an address from all known publicly reachable addresses.
        /// Should be used in synchronous contexts.
        RemoveExternalAddress:remove_external_address_blocking(address:<&Multiaddr>)->();

        /// List all peers that is currently connected.
        /// Should be used in synchronous contexts.
        ListConnected:list_connected_blocking()->Box<[PeerId]>;
        /// Check if the local peer is connected to a remote peer.
        /// Should be used in synchronous contexts.
        IsConnectedToPeerId:is_connected_blocking(peer_id: &PeerId) -> bool;
        /// Disconnect from the peer.
        /// Should be used in synchronous contexts.
        DisconnectFromPeerId:disconnect_peer_id_blocking(peer_id:&PeerId)->Result<(),()>;

    );
    generate_handler_method!(
        /// Dial the address.
        /// Should be used in asynchronous contexts.
        Dial:dial(address: <&Multiaddr>) -> Result<(), DialError>;

        /// Listion on the address.
        /// Should be used in asynchronous contexts.
        Listen:listen(address: <&Multiaddr>) -> Result<ListenerId, TransportError<std::io::Error>>;
        /// List all active listeners.
        /// Should be used in asynchronous contexts
        ListListeners:list_listeners()->Box<[Multiaddr]>;
        /// Remove a listener.
        /// Should be used in asynchronous contexts.
        RemoveListeners:remove_listener(listener_id: &ListenerId)->bool;

        /// Manually confirm the address to be publicly reachable.
        /// Should be used in asynchronous contexts
        AddExternalAddress:add_external_address(address:<&Multiaddr>)->();
        /// List all publicly reachable listen addresses.
        /// Should be used in asynchronous contexts
        ListExternalAddresses:list_external_addresses()->Box<[Multiaddr]>;
        /// Remove an address from all known publicly reachable addresses.
        /// Should be used in asynchronous contexts
        RemoveExternalAddress:remove_external_address(address:<&Multiaddr>)->();

        /// List all peers that is currently connected.
        /// Should be used in asynchronous contexts.
        ListConnected:list_connected()->Box<[PeerId]>;
        /// Check if the local peer is connected to a remote peer.
        /// Should be used in asynchronous contexts
        IsConnectedToPeerId:is_connected(peer_id: &PeerId)->bool;
        /// Disconnect from the peer.
        /// Should be used in asynchronous contexts
        DisconnectFromPeerId:disconnect_peer_id(peer_id:&PeerId)->Result<(),()>;
    );
}
