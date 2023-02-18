use tokio::sync::mpsc::error::SendError;

use super::*;

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    pub swarm_sender: mpsc::Sender<swarm::InEvent>,
    #[cfg(feature = "messaging")]
    pub messaging_sender: mpsc::Sender<messaging::InEvent>,
    #[cfg(feature = "tethering")]
    pub tethering_sender:mpsc::Sender<tethering::InEvent>
}

impl Manager {
    pub async fn swarm_send(&self, ev: swarm::InEvent) -> Result<(), SendError<swarm::InEvent>> {
        self.swarm_sender.send(ev).await
    }
    #[cfg(feature="messaging")]
    pub async fn messaging_send(&self,ev:messaging::InEvent)->Result<(),SendError<messaging::InEvent>>{
        self.messaging_sender.send(ev).await
    }
    #[cfg(feature="tethering")]
    pub async fn tethering_send(&self,ev:tethering::InEvent)->Result<(),SendError<tethering::InEvent>>{
        self.tethering_sender.send(ev).await
    }
}
