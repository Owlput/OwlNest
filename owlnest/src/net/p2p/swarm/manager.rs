use super::in_event;
use super::op::*;
use tokio::sync::{mpsc, oneshot};

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    swarm_sender: mpsc::Sender<in_event::swarm::InEvent>,
    behaviour_sender: mpsc::Sender<in_event::behaviour::InEvent>,
}

impl Manager {
    pub(crate) fn new(
        swarm_sender: mpsc::Sender<in_event::swarm::InEvent>,
        behaviour_sender: mpsc::Sender<in_event::behaviour::InEvent>,
    ) -> Self {
        Self {
            swarm_sender,
            behaviour_sender,
        }
    }

    /// Send event to manage local swarm.
    /// ### Panic
    /// Panics when receiver in the swarm task get dropped or
    /// callback sender get dropped.
    pub async fn swarm_exec(&self, op: swarm::Op) -> swarm::OpResult {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender.send(op.into_event(tx)).await.unwrap();
        rx.await.unwrap()
    }

    /// Blocking version of `swarm_exec`.
    /// ### Panic
    /// Panics when receiver in the swarm task get dropped or
    /// callback sender get dropped.
    pub fn blocking_swarm_exec(&self, op: swarm::Op) -> swarm::OpResult {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender.blocking_send(op.into_event(tx)).unwrap();
        rx.blocking_recv().unwrap()
    }

    /// Send event to manage behaviours.
    /// ### Panic
    /// Panics when receiver in the swarm task get dropped or
    /// callback sender get dropped.
    pub async fn behaviour_exec(&self, op: behaviour::Op) -> behaviour::OpResult {
        let (tx, rx) = oneshot::channel();
        self.behaviour_sender.send(op.into_event(tx)).await.unwrap();
        rx.await.unwrap()
    }

    /// Blocking version of `behaviour_exec`.
    /// ### Panic
    /// Panics when receiver in the swarm task get dropped or
    /// callback sender get dropped.
    pub fn blocking_behaviour_exec(&self, op: behaviour::Op) -> behaviour::OpResult {
        let (tx, rx) = oneshot::channel();
        self.behaviour_sender
            .blocking_send(op.into_event(tx))
            .unwrap();
        rx.blocking_recv().unwrap()
    }
}
