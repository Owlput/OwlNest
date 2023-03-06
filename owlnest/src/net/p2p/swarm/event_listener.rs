use tokio::sync::mpsc;
use crate::net::p2p::protocols::*;

use super::SwarmEvent;


pub enum EventListenerOp{
    Add(EventListener),
    Remove(u128)
}


pub enum SwarmEventListener{
    OnNewListenerAddress(mpsc::Sender<SwarmEvent>)
}

pub enum EventListener
{
    BehaviourEventListener(BehaviourEventListener),
    SwarmEventListener(SwarmEventListener)
}

pub enum BehaviourEventListener{
    Messaging(messaging::event_listener::EventListener)
}