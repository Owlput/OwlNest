use super::*;
use libp2p::{swarm::DialError, TransportError};
use serde::{Serialize,Deserialize};

#[derive(Debug)]
pub struct InEvent{
    pub op:Op,
    pub callback:oneshot::Sender<OpResult>
}

#[derive(Debug, Serialize, Deserialize)]
    pub enum Op {
        Dial(Multiaddr),
        Listen(Multiaddr),
        AddExternalAddress(Multiaddr, String), // Should be AddressScore but it can't be serialzied, which is disappointing
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
