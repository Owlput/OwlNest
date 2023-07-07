pub mod swarm {
    use libp2p::{Multiaddr, TransportError, PeerId};
    use libp2p_swarm::{DialError, derive_prelude::ListenerId};
    use tokio::sync::oneshot::*;
    #[derive(Debug)]
    pub enum InEvent {
        Dial(Multiaddr, Sender<Result<(),DialError>>),
        Listen(Multiaddr,Sender<Result<ListenerId,TransportError<std::io::Error>>>),
        AddExternalAddress(Multiaddr,Sender<()>),
        RemoveExternalAddress(Multiaddr,Sender<()>),
        DisconnectFromPeerId(PeerId,Sender<Result<(),()>>),
        ListExternalAddresses(Sender<Vec<Multiaddr>>),
        ListListeners(Sender<Vec<Multiaddr>>),
        IsConnectedToPeerId(PeerId,Sender<bool>), 
    }
}

pub mod behaviour {

    use crate::net::p2p::protocols::*;
    #[derive(Debug)]
    pub(crate) enum InEvent {
        Messaging(messaging::InEvent),
        Tethering(tethering::InEvent),
        Kad(kad::InEvent),
    }

}
