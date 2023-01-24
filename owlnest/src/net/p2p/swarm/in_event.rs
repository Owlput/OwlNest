use super::*;
use libp2p::{
    swarm::{AddressScore, DialError},
    TransportError,
};

#[derive(Debug)]
pub enum InEvent {
    Swarm(swarm::InEvent),
    #[cfg(feature = "tethering")]
    Tethering(tethering::InEvent),
    #[cfg(feature = "messaging")]
    Messaging(messaging::InEvent),
}

pub mod swarm {
    use super::*;
    #[derive(Debug)]
    pub enum InEvent {
        Dial {
            addr: Multiaddr,
            callback_tx: oneshot::Sender<OpResult>,
        },
        Listen {
            addr: Multiaddr,
            callback_tx: oneshot::Sender<OpResult>,
        },
        AddExternalAddress {
            addr: Multiaddr,
            score: AddressScore,
            callback_tx: oneshot::Sender<OpResult>,
        },
    }
}

#[derive(Debug)]
pub enum OpResult {
    DialOk,
    DialErr(DialError),
    ListenOk,
    ListenErr(TransportError<std::io::Error>),
    AddExternalAddress(String),
    EmitOk,
    EmitErr(mpsc::error::SendError<InEvent>),
    CallbackRecvErr(oneshot::error::RecvError),
}
