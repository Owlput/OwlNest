pub(crate) mod behaviour {
    use crate::net::p2p::protocols::*;
    use crate::net::p2p::swarm::in_event;
    use tokio::sync::oneshot;

    #[derive(Debug)]
    pub enum Op {
        Messaging(messaging::Op),
        Tethering(tethering::Op),
        Kad(kad::Op),
    }
    impl Op {
        pub(crate) fn into_event(
            self,
            sender: oneshot::Sender<OpResult>,
        ) -> in_event::behaviour::InEvent {
            match self {
                Op::Messaging(op) => messaging::InEvent::new(op, sender).into(),
                Op::Tethering(op) => tethering::InEvent::new(op, sender).into(),
                Op::Kad(op) => kad::InEvent::new(op, sender).into(),
            }
        }
    }

    #[derive(Debug)]
    pub enum OpResult {
        Messaging(messaging::OpResult),
        Tethering(Result<tethering::HandleOk, tethering::HandleError>),
        Kad(kad::OpResult),
    }
    impl TryInto<messaging::OpResult> for OpResult {
        type Error = ();
        fn try_into(self) -> Result<messaging::OpResult, Self::Error> {
            match self {
                Self::Messaging(result) => Ok(result),
                _ => Err(()),
            }
        }
    }
    impl TryInto<Result<tethering::HandleOk, tethering::HandleError>> for OpResult {
        type Error = ();
        fn try_into(
            self,
        ) -> Result<Result<tethering::HandleOk, tethering::HandleError>, Self::Error> {
            match self {
                Self::Tethering(result) => Ok(result),
                _ => Err(()),
            }
        }
    }
    impl TryInto<kad::OpResult> for OpResult {
        type Error = ();
        fn try_into(self) -> Result<kad::OpResult, Self::Error> {
            match self {
                Self::Kad(result) => Ok(result),
                _ => Err(()),
            }
        }
    }
    pub type CallbackSender = oneshot::Sender<OpResult>;
}

pub mod swarm {

    use crate::net::p2p::swarm::in_event::swarm::InEvent;
    use libp2p::{
        swarm::{derive_prelude::ListenerId, DialError},
        Multiaddr, PeerId, TransportError,
    };
    use tokio::sync::{mpsc, oneshot::*};

    pub struct SwarmHandle {
        sender: mpsc::Sender<InEvent>,
    }
    impl SwarmHandle {
        pub fn dial(&self, addr: &Multiaddr) -> Result<(), DialError> {
            let (tx, rx) = channel();
            let ev = InEvent::Dial(addr.clone(), tx);
            self.sender.blocking_send(ev).unwrap();
            rx.blocking_recv().unwrap()
        }
        pub fn listen(
            &self,
            addr: &Multiaddr,
        ) -> Result<ListenerId, TransportError<std::io::Error>> {
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
}
