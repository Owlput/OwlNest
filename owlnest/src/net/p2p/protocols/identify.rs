pub use libp2p::identify::Behaviour;
pub use libp2p::identify::Config;
pub use libp2p::identify::Event as OutEvent;
pub use libp2p::identify::PROTOCOL_NAME;
use tracing::{debug, trace, warn};

use crate::event_bus::listened_event::Listenable;

pub fn ev_dispatch(ev: &OutEvent) {
    use OutEvent::*;
    match ev {
        Received { peer_id, info } => {
            debug!("Identified peer {} : {}", peer_id, info.protocol_version)
        }
        Sent { peer_id } => trace!(
            "Identification information has been sent to peer {} as response",
            peer_id
        ),
        Pushed { peer_id, info } => trace!(
            "Information {:?} has been sent to peer {} for identification",
            info,
            peer_id
        ),
        Error { peer_id, error } => {
            warn!("Error when identifying peer {} with {}", peer_id, error)
        }
    }
}

impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        PROTOCOL_NAME.to_string()
    }
}
