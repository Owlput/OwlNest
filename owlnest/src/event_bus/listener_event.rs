use tokio::sync::{mpsc, oneshot};

use crate::net::p2p::protocols::*;
use crate::net::p2p::swarm;

use super::Error;

/// Top-lever wrapper enum around listener operations from different modules.
#[derive(Debug)]
pub enum EventListenerKind {
    /// Variant for operations on behaviour level.
    Behaviours(BehaviourListenerKind),
    /// Variant for operations on swarm level.
    Swarm(swarm::event_listener::Op),
}

/// Behaviour-level wrapper around listener operations.
#[derive(Debug)]
pub enum BehaviourListenerKind {
    /// Listener operations for `owlnest/messaging`
    Messaging(messaging::event_listener::Kind),
    /// Listener operations for `owlnest/kad`
    Kad(kad::event_listener::Kind),
}

#[derive(Debug)]
pub(crate) enum EventListenerOp {
    Add(
        EventListenerKind,
        mpsc::Sender<ListenedEvent>,
        oneshot::Sender<Result<u64, Error>>,
    ),
    Remove(EventListenerKind, u64, oneshot::Sender<Result<(),Error>>),
}

#[derive(Debug)]
pub enum ListenedEvent {
    Behaviours(BehaviourEvent),
    Swarm(swarm::out_event::SwarmEvent),
}

#[derive(Debug)]
pub enum BehaviourEvent {
    /// Listener operations for `owlnest/messaging`
    Messaging(messaging::OutEvent),
    /// Listener operations for `owlnest/kad`
    Kad(kad::OutEvent),
}
