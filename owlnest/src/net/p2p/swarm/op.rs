use crate::net::p2p::swarm::in_event::InEvent;
use libp2p::{
    swarm::{derive_prelude::ListenerId, DialError},
    Multiaddr, PeerId, TransportError,
};
use tokio::sync::{mpsc, oneshot::*};

#[derive(Debug, Clone)]
pub struct SwarmHandle {
    sender: mpsc::Sender<InEvent>,
}
impl SwarmHandle {
    pub fn new(buffer:usize) -> (Self,mpsc::Receiver<InEvent>) {
        let (tx,rx) = mpsc::channel(buffer);
        (Self { sender:tx },rx)
    }
    pub fn dial(&self, addr: &Multiaddr) -> Result<(), DialError> {
        let (tx, rx) = channel();
        let ev = InEvent::Dial(addr.clone(), tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn listen(&self, addr: &Multiaddr) -> Result<ListenerId, TransportError<std::io::Error>> {
        let (tx, rx) = channel();
        let ev = InEvent::Listen(addr.clone(), tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn add_external_address(&self, addr: &Multiaddr) {
        let (tx, rx) = channel();
        let ev = InEvent::AddExternalAddress(addr.clone(), tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn remove_external_address(&self, addr: &Multiaddr) {
        let (tx, rx) = channel();
        let ev = InEvent::RemoveExternalAddress(addr.clone(), tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn disconnect_peer_id(&self, peer_id: &PeerId) -> Result<(), ()> {
        let (tx, rx) = channel();
        let ev = InEvent::DisconnectFromPeerId(*peer_id, tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn list_external_addresses(&self) -> Vec<Multiaddr> {
        let (tx, rx) = channel();
        let ev = InEvent::ListExternalAddresses(tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn list_listeners(&self) -> Vec<Multiaddr> {
        let (tx, rx) = channel();
        let ev = InEvent::ListListeners(tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        let (tx, rx) = channel();
        let ev = InEvent::IsConnectedToPeerId(*peer_id, tx);
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
}
