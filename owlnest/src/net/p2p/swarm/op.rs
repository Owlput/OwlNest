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
    use crate::net::p2p::swarm::in_event;
    use libp2p::{
        swarm::{derive_prelude::ListenerId, AddressRecord, DialError},
        Multiaddr, PeerId, TransportError,
    };
    use serde::{Deserialize, Serialize};
    use tokio::sync::oneshot;

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
    impl Op {
        pub fn into_event(self, sender: oneshot::Sender<OpResult>) -> in_event::swarm::InEvent {
            in_event::swarm::InEvent::new(self, sender)
        }
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

    #[derive(Debug)]
    pub enum AddExternalAddressResult {
        Inserted,
        Updated,
    }
}

// use libp2p_swarm::Stream;
// pub trait Op {
//     type StreamState;
//     type Handler;
//     type Behaviour;
//     fn execute_behaviour(&mut self, behaviour: &mut Self::Behaviour) {}
//     fn execute_handler(&mut self, handler: &mut Self::Handler) {}
//     fn execute_io(&mut self, stream: Stream) -> Self::StreamState;
// }
