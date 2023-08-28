/// A behaviour that allows remote control of a node. 

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use subprotocols::*;
use tokio::sync::{mpsc, oneshot};

/// `Behaviour` of this protocol.
mod behaviour;
/// Command-line interface for this protocol.
pub mod cli;
/// Errors produced by this behaviour.
pub mod error;
/// Results produced by this behaviour.
mod result;
/// Protocol `/owlnest/tethering` consists of two subprotocols.
/// `/owlnest/tethering/exec` for operation execution,
/// `/owlnest/tethering/push` for notification pushing.
/// Both subprotocols will perform a handshake in TCP style(aka three-way handshake).
mod subprotocols;

pub use subprotocols::Subprotocol;
pub use behaviour::Behaviour;
pub use error::Error;
pub use result::*;

/// A placeholder struct waiting to be used for interface consistency.
#[derive(Debug, Default)]
pub struct Config;

type CallbackSender = oneshot::Sender<Result<HandleOk, HandleError>>;

#[derive(Debug)]
pub(crate) struct InEvent {
    op: Op,
    callback: CallbackSender,
}
impl InEvent {
    pub fn into_inner(self) -> (Op, CallbackSender) {
        (self.op, self.callback)
    }
}

#[derive(Debug)]
pub enum Op {
    RemoteExec(PeerId, exec::op::Op, oneshot::Sender<exec::result::OpResult>),
    RemoteCallback(PeerId, u128, exec::result::OpResult),
    LocalExec(TetheringOp),
    Push(PeerId, push::PushType),
}

#[derive(Debug)]
pub enum OutEvent {
    Exec(exec::op::Op, u128),
    IncomingNotification(String),
    ExecError(exec::handler::Error),
    PushError(push::handler::Error),
    Unsupported(PeerId, Subprotocol),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TetheringOp {
    Trust(PeerId),
    Untrust(PeerId),
}

#[derive(Debug, Clone)]
pub enum TetheringOpError {
    NotFound,
    AlreadyTrusted,
}

pub fn ev_dispatch(_ev: &OutEvent) {}

#[derive(Debug,Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
}
impl Handle {
    pub(crate) fn new(buffer: usize) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self { sender: tx }, rx)
    }

    pub fn blocking_trust(&self,peer_id:PeerId)->Result<HandleOk,HandleError>{
        let (tx,rx) = oneshot::channel();
        let ev = InEvent{
            op:Op::LocalExec(TetheringOp::Trust(peer_id)),
            callback:tx
        };
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
    
    pub fn blocking_untrust(&self,peer_id:PeerId)->Result<HandleOk,HandleError>{
        let (tx,rx) = oneshot::channel();
        let ev = InEvent{
            op:Op::LocalExec(TetheringOp::Untrust(peer_id)),
            callback:tx
        };
        self.sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
}
