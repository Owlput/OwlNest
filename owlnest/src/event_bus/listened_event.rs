use std::sync::Arc;

use tokio::sync::{broadcast, oneshot};
use crate::net::p2p::protocols::*;
use crate::net::p2p::swarm;
use super::Error;

#[derive(Debug)]
pub(crate) enum EventListenerOp {
    Add(
        EventKind,
        oneshot::Sender<Result<broadcast::Receiver<ListenedEvent>, Error>>,
    ),
}

#[derive(Debug, Clone)]
pub enum ListenedEvent {
    Behaviours(BehaviourEvent),
    Swarm(Arc<swarm::out_event::SwarmEvent>),
}

#[derive(Debug,Clone)]
pub enum BehaviourEvent {
    /// Listener operations for `owlnest/messaging`
    Messaging(messaging::OutEvent),
    /// Listener operations for `owlnest/kad`
    Kad(kad::OutEvent),
    RelayClient(Arc<relay_client::OutEvent>),
    RelayServer(Arc<relay_server::OutEvent>),
    Tethering(Arc<tethering::OutEvent>)
}
impl Into<BehaviourEventKind> for &BehaviourEvent{
    fn into(self) -> BehaviourEventKind {
        match self{
            BehaviourEvent::Messaging(ev) => BehaviourEventKind::Messaging(ev.into()),
            BehaviourEvent::Kad(_) => todo!(),
            BehaviourEvent::RelayClient(_) => todo!(),
            BehaviourEvent::RelayServer(_) => todo!(),
            BehaviourEvent::Tethering(_) => todo!(),
        }
    }
}

/// Top-lever wrapper enum around listeners available from different modules.
#[derive(Debug)]
pub enum EventKind {
    /// Variant for operations on behaviour level.
    Behaviours(BehaviourEventKind),
    /// Variant for operations on swarm level.
    Swarm(swarm::event_listener::Kind),
}

/// Behaviour-level wrapper around listener operations.
#[derive(Debug)]
pub enum BehaviourEventKind {
    /// Listener operations for `owlnest/messaging`
    Messaging(messaging::event_listener::Kind),
    /// Listener operations for `owlnest/kad`
    Kad(kad::event_listener::Kind),
    RelayClient(relay_client::Kind),
    RelayServer(relay_server::Kind),
}

pub trait AsEventKind{
    fn kind(&self)->EventKind;
}