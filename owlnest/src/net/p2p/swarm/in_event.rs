use crate::net::p2p::protocols::tethering::TetheringOpError;

use super::*;
use libp2p::{
    swarm::{derive_prelude::ListenerId, DialError},
    TransportError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<OpResult>,
}
impl InEvent {
    pub fn new(op: Op, callback: oneshot::Sender<OpResult>) -> Self {
        InEvent { op, callback }
    }
    pub fn into_inner(self) -> (Op, oneshot::Sender<OpResult>) {
        (self.op, self.callback)
    }
}

#[derive(Debug)]
pub enum OpResult {
    Dial(Result<(), DialError>),
    Listen(Result<ListenerId, TransportError<std::io::Error>>),
    AddExternalAddress(AddExternalAddressResult),
    RemoveExternalAddress(bool),
    BanByPeerId,
    UnbanByPeerId,
    DisconnectFromPeerId(Result<(), ()>),
    ListExternalAddresses(Vec<AddressRecord>),
    ListListeners(Vec<Multiaddr>),
    IsConnectedToPeerId(bool),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Dial(Multiaddr),
    Listen(Multiaddr),
    AddExternalAddress(Multiaddr, Option<u32>),
    RemoveExternalAddress(Multiaddr),
    BanByPeerId(PeerId),
    UnbanByPeerId(PeerId),
    DisconnectFromPeerId(PeerId),
    ListExternalAddresses,
    ListListeners,
    IsConnectedToPeerId(PeerId),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AddExternalAddressResult {
    Inserted,
    Updated,
}

#[derive(Debug)]
pub enum BehaviourOp {
    Messaging(messaging::Op),
    Tethering(tethering::Op),
    Kad(kad::Op)
}

#[derive(Debug)]
pub enum BehaviourOpResult {
    Messaging(messaging::OpResult),
    Tethering(Result<tethering::HandleOk,tethering::HandleError>),
    Kad(kad::OpResult)
}
impl TryInto<messaging::OpResult> for BehaviourOpResult{
    type Error = ();
    fn try_into(self) -> Result<messaging::OpResult, Self::Error> {
        match self{
            Self::Messaging(result)=>Ok(result),
            _=>Err(())
        }
    }
}
impl TryInto<Result<tethering::HandleOk,tethering::HandleError>> for BehaviourOpResult{
    type Error = ();
    fn try_into(self) -> Result<Result<tethering::HandleOk,tethering::HandleError>, Self::Error> {
        match self{
            Self::Tethering(result)=>Ok(result),
            _=>Err(())
        }
    }
}
impl TryInto<kad::OpResult> for BehaviourOpResult{
    type Error = ();
    fn try_into(self) -> Result<kad::OpResult, Self::Error> {
        match self{
            Self::Kad(result)=>Ok(result),
            _=>Err(())
        }
    }
}



#[derive(Debug)]
pub enum BehaviourInEvent {
    Messaging(messaging::InEvent),
    Tethering(tethering::InEvent),
    Kad(kad::InEvent)
}
