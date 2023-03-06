use super::*;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum OutEvent {
    Messaging(messaging::OutEvent),
    Tethering(tethering::OutEvent),
    RelayClient(relay_client::OutEvent),
    RelayServer(relay_server::OutEvent),
}
impl From<tethering::OutEvent> for OutEvent {
    fn from(value: tethering::OutEvent) -> Self {
        super::OutEvent::Tethering(value)
    }
}
impl From<messaging::OutEvent> for OutEvent {
    fn from(value: messaging::OutEvent) -> Self {
        super::OutEvent::Messaging(value)
    }
}


pub struct OutEventBundle {
    pub messaging_rx: mpsc::Receiver<messaging::OutEvent>,
    pub tethering_rx: mpsc::Receiver<tethering::OutEvent>,
    pub relay_client_rx: mpsc::Receiver<relay_client::OutEvent>,
    pub relay_server_rx: mpsc::Receiver<relay_server::OutEvent>,
}
