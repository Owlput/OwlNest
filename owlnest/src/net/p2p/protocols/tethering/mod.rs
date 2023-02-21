use self::{
    subprotocols::{
        exec::{self, result::HandleResult, Op},
        push,
    },
};
use std::fmt::Debug;

use super::*;

pub mod error;
pub mod result;
pub mod behaviour;

/// Protocol `/owlnest/tethering` is divided into two subprotocols.
/// `/owlnest/tethering/exec` for operation execution,
/// `/owlnest/tethering/push` for notification pushing.
/// Both subprotocols will perform a handshake in TCP style(aka three-way handshake).
pub mod subprotocols;
pub use error::Error;
pub use result::OpResult;
pub use behaviour::Behaviour;

/// A placeholder struct waiting to be used for interface consistency.
#[derive(Debug,Default)]
pub struct Config;

#[derive(Debug)]
pub enum InEvent {
    RemoteExec(
        PeerId,
        exec::Op,
        oneshot::Sender<exec::result::HandleResult>,
        oneshot::Sender<exec::result::OpResult>,
    ),
    RemoteCallback(
        PeerId,
        u128,
        exec::result::OpResult,
        oneshot::Sender<HandleResult>,
    ),
    LocalExec(TetheringOp, oneshot::Sender<TetheringOpResult>),
    Push(
        PeerId,
        push::PushType,
        oneshot::Sender<push::result::OpResult>,
    ),
}

#[derive(Debug)]
pub enum OutEvent {
    Exec(Op,u128),
    IncomingNotification(String),
    ExecError(exec::handler::Error),
    PushError(push::handler::Error),
    Unsupported(PeerId,Subprotocol)
}

#[derive(Debug)]
pub enum Subprotocol{
    Exec,
    Push
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TetheringOp {
    Trust(PeerId),
    Untrust(PeerId),
}

#[derive(Debug)]
pub enum TetheringOpResult{
    Ok,
    AlreadyTrusted,
    Err(TetheringOpError),
}

#[derive(Debug)]
pub enum TetheringOpError{
    NotFound
}

pub fn ev_dispatch(_ev: OutEvent) {}
