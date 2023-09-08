pub use libp2p::identify::Behaviour;
pub use libp2p::identify::Config;
pub use libp2p::identify::Event as OutEvent;
pub use libp2p::identify::PROTOCOL_NAME;
use tracing::info;

use crate::event_bus::listened_event::Listenable;

pub fn ev_dispatch(ev: &OutEvent) {
    use OutEvent::*;
    match ev {
        Received { peer_id, info } => info!("Identified peer {} with {:?}", peer_id, info),
        Sent { peer_id } => info!(
            "Identification information has been sent to peer {} as response",
            peer_id
        ),
        Pushed { peer_id } => info!(
            "Identification information has been sent to peer {} for identification",
            peer_id
        ),
        Error { peer_id, error } => {
            info!("Error when identifying peer {} with {}", peer_id, error)
        }
    }
}

impl Listenable for OutEvent{
    fn as_event_identifier()->String {
        PROTOCOL_NAME.to_string()
    }
}