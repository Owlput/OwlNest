use tokio::sync::mpsc;
use super::*;

#[derive(Debug)]
pub enum OutEvent {
    #[cfg(feature = "messaging")]
    Messaging(messaging::OutEvent),
    #[cfg(feature = "tethering")]
    Tethering(tethering::OutEvent),
    #[cfg(feature = "relay-client")]
    RelayClient(relay_client::OutEvent),
    #[cfg(feature = "relay-server")]
    RelayServer(relay_server::OutEvent),
}

#[cfg(feature = "tethering")]
impl From<tethering::OutEvent> for OutEvent {
    fn from(value: tethering::OutEvent) -> Self {
        super::OutEvent::Tethering(value)
    }
}

#[cfg(feature = "messaging")]
impl From<messaging::OutEvent> for OutEvent {
    fn from(value: messaging::OutEvent) -> Self {
        super::OutEvent::Messaging(value)
    }
}

pub struct OutEventBundle{
    #[cfg(feature = "messaging")]
    pub messaging_rx:mpsc::Receiver<messaging::OutEvent>,
    #[cfg(feature = "tethering")]
    pub tethering_rx:mpsc::Receiver<tethering::OutEvent>,
    #[cfg(feature = "relay-client")]
    pub relay_client_rx:mpsc::Receiver<relay_client::OutEvent>,
    #[cfg(feature = "relay-server")]
    pub relay_server_rx:mpsc::Receiver<relay_server::OutEvent>,
}