use super::Error;
use crate::net::p2p::protocols::*;
use crate::net::p2p::swarm;
use std::{any::Any, sync::Arc};
use tokio::sync::{broadcast, oneshot};

#[derive(Clone)]
pub struct ListenedEvent(String, Arc<Box<dyn Any + Send + Sync + 'static>>);
impl ListenedEvent {
    pub fn new(ident: String, event: impl Any + Send + Sync) -> Self {
        Self(ident, Arc::new(Box::new(event)))
    }
    pub fn kind(&self) -> String {
        self.0.clone()
    }
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.1.downcast_ref::<T>()
    }
}
impl std::fmt::Debug for ListenedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ListenedEvent").field(&self.0).finish()
    }
}

#[derive(Debug)]
pub(crate) enum EventListenerOp {
    Add(
        String,
        oneshot::Sender<Result<broadcast::Receiver<ListenedEvent>, Error>>,
    ),
}

/// Top-lever wrapper enum around listeners available from different modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Variant for operations on behaviour level.
    Behaviours(BehaviourEventKind),
    /// Variant for operations on swarm level.
    Swarm(swarm::event_listener::Kind),
}

/// Behaviour-level wrapper around listener operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviourEventKind {
    /// Listener operations for `owlnest/messaging`
    Messaging(messaging::Kind),
    /// Listener operations for `owlnest/kad`
    Kad(kad::event_listener::Kind),
}

pub trait AsEventKind {
    type EventKind;
    fn kind(&self) -> Self::EventKind;
}
