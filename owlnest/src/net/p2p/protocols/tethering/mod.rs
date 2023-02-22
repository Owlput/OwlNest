use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use subprotocols::*;
use tokio::sync::oneshot;

mod behaviour;
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
pub use result::OpResult;

/// A placeholder struct waiting to be used for interface consistency.
#[derive(Debug, Default)]
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
        oneshot::Sender<exec::result::HandleResult>,
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
    Exec(exec::Op, u128),
    IncomingNotification(String),
    ExecError(exec::handler::Error),
    PushError(push::handler::Error),
    Unsupported(PeerId, Subprotocol),
}

#[derive(Debug)]
pub enum Subprotocol {
    Exec,
    Push,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TetheringOp {
    Trust(PeerId),
    Untrust(PeerId),
}

#[derive(Debug)]
pub enum TetheringOpResult {
    Ok,
    AlreadyTrusted,
    Err(TetheringOpError),
}

#[derive(Debug)]
pub enum TetheringOpError {
    NotFound,
}

pub fn ev_dispatch(_ev: OutEvent) {}
