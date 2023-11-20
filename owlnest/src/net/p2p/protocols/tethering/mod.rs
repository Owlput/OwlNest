/// A behaviour that allows remote control of a node.
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use subprotocols::*;
use tokio::sync::mpsc;

/// `Behaviour` of this protocol.
mod behaviour;

/// Command-line interface for this protocol.
pub mod cli;

/// Errors produced by this behaviour.
pub mod error;

/// Protocol `/owlnest/tethering` consists of two subprotocols.
/// `/owlnest/tethering/exec` for operation execution,
/// `/owlnest/tethering/push` for notification pushing.
/// Both subprotocols will perform a handshake in TCP style(aka three-way handshake).
mod subprotocols;

pub use behaviour::Behaviour;
pub use error::Error;
pub use subprotocols::Subprotocol;

use self::subprotocols::exec::OpResult;
use crate::{
    handle_listener_result,
    net::p2p::swarm::{SwarmEvent,behaviour::BehaviourEvent, EventSender},
    with_timeout,
};

/// A placeholder struct waiting to be used for interface consistency.
#[derive(Debug, Default)]
pub struct Config;

#[allow(unused)]
#[derive(Debug)]
pub(crate) enum InEvent {
    RemoteExec(PeerId, exec::op::Op, u64),
    RemoteCallback(PeerId, u64, exec::result::OpResult),
    LocalExec(TetheringOp, u64),
    Push(PeerId, push::PushType, u64),
}

#[derive(Debug)]
pub enum OutEvent {
    LocalExec(Result<(), ()>, u64),
    Exec(exec::op::Op, u64),
    RemoteExecResult(OpResult, u64),
    IncomingNotification(String),
    ExecError(exec::handler::Error),
    PushError(push::handler::Error),
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

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    counter: Arc<AtomicU64>,
}
impl Handle {
    pub(crate) fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
                counter: Arc::new(AtomicU64::new(1)),
            },
            rx,
        )
    }

    pub async fn trust(&self, peer_id: PeerId) -> Result<(), ()> {
        let ev_id = self.counter.fetch_add(1, Ordering::SeqCst);
        let ev = InEvent::LocalExec(TetheringOp::Trust(peer_id), ev_id);
        self.sender.send(ev).await.expect("send to succeed");
        let mut listener = self.event_tx.subscribe();
        let fut = async move {
            loop {
                if let SwarmEvent::Behaviour(BehaviourEvent::Tethering(OutEvent::LocalExec(
                    result,
                    id,
                ))) = handle_listener_result!(listener).as_ref()
                {
                    if *id == ev_id {
                        return result.clone();
                    }
                }
            }
        };
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }

    pub async fn untrust(&self, peer_id: PeerId) -> Result<(), ()> {
        let ev_id = self.counter.fetch_add(1, Ordering::SeqCst);
        let ev = InEvent::LocalExec(TetheringOp::Untrust(peer_id), ev_id);
        self.sender.send(ev).await.expect("send to succeed");
        Ok(())
    }
}
