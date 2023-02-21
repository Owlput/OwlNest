use libp2p::identify::Event;

use tracing::info;

pub use libp2p::identify::Behaviour;
pub use libp2p::identify::Config;
pub type OutEvent = Event;

pub fn ev_dispatch(ev:OutEvent){
    match ev{
        Event::Received { peer_id, info } => info!("Identified peer {} with {:?}",peer_id,info),
        Event::Sent { peer_id } => info!("Identification information has been sent to peer {} as response",peer_id),
        Event::Pushed { peer_id } => info!("Identification information has been sent to peer {} for identification",peer_id),
        Event::Error { peer_id, error } => info!("Error when identifying peer {} with {}",peer_id,error),
    }
}