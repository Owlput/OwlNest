use super::in_event;
use super::op::*;
use super::op::swarm::SwarmHandle;
use tokio::sync::{mpsc, oneshot};

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    pub swarm_sender: mpsc::Sender<in_event::swarm::InEvent>,
    pub behaviour_sender: mpsc::Sender<in_event::behaviour::InEvent>,
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

    /// Get a handle for the swarm.
    pub fn swarm_handle(&self)->SwarmHandle{
        SwarmHandle{
            sender:self.swarm_sender.clone()
        }
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
