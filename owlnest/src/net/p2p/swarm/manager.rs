use std::marker::PhantomData;

use super::{*, in_event::{BehaviourOpResult, BehaviourInEvent}};
use tokio::sync::{mpsc, oneshot};

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    pub swarm_sender: mpsc::Sender<swarm::in_event::InEvent>,
    pub behaviour_sender: mpsc::Sender<in_event::BehaviourInEvent>,
    pub ev_listener_sender:mpsc::Sender<swarm::EventListenerOp>,
}

impl Manager {
    /// Send event to the that manages local swarm
    /// ## Panic
    /// Panics when receiver in the swarm task get dropped or
    /// callback sender get dropped.
    pub async fn swarm_exec(&self, op: swarm_op::Op) -> swarm_op::OpResult {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender
            .send(swarm::in_event::InEvent::new(op, tx))
            .await
            .unwrap();
        rx.await.unwrap()
    }
    /// Blocking version of method `swarm_exec`.
    /// ## Panic
    /// Panics when receiver in the swarm task get dropped or
    /// callback sender get dropped.
    pub fn blocking_swarm_exec(&self, op: swarm_op::Op) -> swarm_op::OpResult {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender
            .blocking_send(swarm::in_event::InEvent::new(op, tx))
            .unwrap();
        rx.blocking_recv().unwrap()
    }
    pub async fn behaviour_exec(&self,op:BehaviourOp)->BehaviourOpResult{
        let (tx,rx) = oneshot::channel();
        let event = match op{
            BehaviourOp::Messaging(op) => BehaviourInEvent::Messaging(messaging::InEvent::new(op, tx)),
            BehaviourOp::Tethering(op)=>BehaviourInEvent::Tethering(tethering::InEvent::new(op,tx)),
            BehaviourOp::Kad(op)=>BehaviourInEvent::Kad(kad::InEvent::new(op, tx))
        };
        self.behaviour_sender.send(event).await.unwrap();
        rx.await.unwrap()
    }
    pub fn blocking_behaviour_exec(&self,op:in_event::BehaviourOp)->BehaviourOpResult{
        let (tx,rx) = oneshot::channel();
        let event = match op{
            BehaviourOp::Messaging(op) => BehaviourInEvent::Messaging(messaging::InEvent::new(op, tx)),
            BehaviourOp::Tethering(op)=>BehaviourInEvent::Tethering(tethering::InEvent::new(op,tx)),
            BehaviourOp::Kad(op)=>BehaviourInEvent::Kad(kad::InEvent::new(op, tx))
        };
        self.behaviour_sender.blocking_send(event).unwrap();
        rx.blocking_recv().unwrap()
    }
    pub async fn add_event_listener(&self,listener_type:EventListener,queue_legth:usize)->(u16,mpsc::Receiver<ListenedEvent>)
    {
        let (op_tx,op_rx) = oneshot::channel();
        let (ev_tx,ev_rx) = mpsc::channel(queue_legth);
        let ev = EventListenerOp::Add(listener_type,ev_tx,op_tx);
        self.ev_listener_sender.send(ev).await.unwrap();
        (op_rx.await.unwrap(),ev_rx)
    }
}

