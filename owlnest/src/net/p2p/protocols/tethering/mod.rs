use crate::net::p2p::swarm::{self, op::behaviour::CallbackSender};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use subprotocols::*;
use tokio::sync::oneshot;

/// `Behaviour` of this protocol.
mod behaviour;
/// Command-line interface for this protocol.
pub mod cli;
/// Errors produced by this behaviour.
pub mod error;
/// Results produced by this behaviour.
mod result;
/// Protocol `/owlnest/tethering` is divided into two subprotocols.
/// `/owlnest/tethering/exec` for operation execution,
/// `/owlnest/tethering/push` for notification pushing.
/// Both subprotocols will perform a handshake in TCP style(aka three-way handshake).
pub mod subprotocols;

pub use behaviour::Behaviour;
pub use error::Error;
pub use result::*;

/// A placeholder struct waiting to be used for interface consistency.
#[derive(Debug, Default)]
pub struct Config;

#[derive(Debug)]
pub(crate) struct InEvent {
    op: Op,
    callback: CallbackSender,
}
impl InEvent {
    pub fn new(op: Op, callback: CallbackSender) -> Self {
        Self { op, callback }
    }
    pub fn into_inner(self) -> (Op, CallbackSender) {
        (self.op, self.callback)
    }
}
impl From<InEvent> for swarm::in_event::behaviour::InEvent {
    fn from(val: InEvent) -> Self {
        swarm::in_event::behaviour::InEvent::Tethering(val)
    }
}

#[derive(Debug)]
pub enum Op {
    RemoteExec(PeerId, exec::Op, oneshot::Sender<exec::result::OpResult>),
    RemoteCallback(PeerId, u128, exec::result::OpResult),
    LocalExec(TetheringOp),
    Push(PeerId, push::PushType),
}
impl From<Op> for swarm::op::behaviour::Op {
    fn from(val: Op) -> Self {
        swarm::op::behaviour::Op::Tethering(val)
    }
}

#[derive(Debug)]
pub enum OutEvent {
    Exec(exec::Op, u128),
    IncomingNotification(String),
    ExecError(exec::handler::Error),
    PushError(push::handler::Error),
    Unsupported(PeerId, Subprotocol),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subprotocol {
    Exec,
    Push,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TetheringOp {
    Trust(PeerId),
    Untrust(PeerId),
}
impl From<TetheringOp> for swarm::op::behaviour::Op {
    fn from(val: TetheringOp) -> Self {
        swarm::op::behaviour::Op::Tethering(Op::LocalExec(val))
    }
}

#[derive(Debug, Clone)]
pub enum TetheringOpError {
    NotFound,
    AlreadyTrusted,
}

pub fn ev_dispatch(_ev: &OutEvent) {}
