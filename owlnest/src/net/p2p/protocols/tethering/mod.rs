use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};
use subprotocols::*;
use tokio::sync::oneshot;
use crate::event_bus::prelude::*;

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

use crate::net::p2p::swarm::{BehaviourOp, BehaviourOpResult};

/// A placeholder struct waiting to be used for interface consistency.
#[derive(Debug, Default)]
pub struct Config;

#[derive(Debug)]
pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<BehaviourOpResult>,
}
impl InEvent {
    pub fn new(op: Op, callback: oneshot::Sender<BehaviourOpResult>) -> Self {
        Self { op, callback }
    }
    pub fn into_inner(self) -> (Op, oneshot::Sender<BehaviourOpResult>) {
        (self.op, self.callback)
    }
}

#[derive(Debug)]
pub enum Op {
    RemoteExec(PeerId, exec::Op, oneshot::Sender<exec::result::OpResult>),
    RemoteCallback(PeerId, u128, exec::result::OpResult),
    LocalExec(TetheringOp),
    Push(PeerId, push::PushType),
}

#[derive(Debug)]
pub enum OutEvent {
    Exec(exec::Op, u128),
    IncomingNotification(String),
    ExecError(exec::handler::Error),
    PushError(push::handler::Error),
    Unsupported(PeerId, Subprotocol),
}
impl Into<ListenedEvent> for OutEvent{
    fn into(self) -> ListenedEvent {
        ListenedEvent::Behaviours(BehaviourEvent::Tethering(Arc::new(self)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subprotocol {
    Exec,
    Push,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TetheringOp {
    Trust(PeerId),
    Untrust(PeerId),
}
impl Into<BehaviourOp> for TetheringOp {
    fn into(self) -> BehaviourOp {
        BehaviourOp::Tethering(Op::LocalExec(self))
    }
}

#[derive(Debug, Clone)]
pub enum TetheringOpError {
    NotFound,
    AlreadyTrusted,
}

pub fn ev_dispatch(_ev: &OutEvent) {}
