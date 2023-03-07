use tokio::sync::{mpsc, oneshot};
use crate::net::p2p::protocols::*;

use super::SwarmEvent;

pub enum ListenedEvent{
    Swarm(SwarmEvent),
    Kad(kad::OutEvent),
    Messaging(messaging::OutEvent)
}

#[derive(Debug)]
pub enum EventListenerOp{
    Add(EventListener,mpsc::Sender<ListenedEvent>,oneshot::Sender<u16>),
    Remove(u128,oneshot::Sender<()>)
}

#[derive(Debug)]
pub enum SwarmEventListener{
    OnNewListenerAddress,
    OnDialing,
}

#[derive(Debug)]
pub enum EventListener
{
    BehaviourEventListener(BehaviourEventListener),
    SwarmEventListener(SwarmEventListener),
}

#[derive(Debug)]
pub enum BehaviourEventListener{
    Messaging(messaging::event_listener::EventListener),
    Kad(kad::event_listener::EventListener)
}