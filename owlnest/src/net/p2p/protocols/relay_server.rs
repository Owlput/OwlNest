pub use libp2p::relay::Config;
use tracing::{debug, info};

pub type Behaviour = libp2p::relay::Behaviour;
pub type OutEvent = libp2p::relay::Event;

/// Log the events to tracing
pub(crate) fn ev_dispatch(ev: &OutEvent) {
    use libp2p::relay::Event::*;
    match ev {
        ReservationReqAccepted {
            src_peer_id,
            renewed,
        } => debug!(
            "Reservation from {} accepted, IsRenew:{}",
            src_peer_id, renewed
        ),
        ReservationReqDenied { src_peer_id } => {
            info!("Denied reservation from {}", src_peer_id)
        }
        ReservationTimedOut { src_peer_id } => {
            info!("Reservation expired for source peer {}", src_peer_id)
        }
        CircuitReqDenied {
            src_peer_id,
            dst_peer_id,
        } => info!(
            "Circuit request from {} to peer {} denied",
            src_peer_id, dst_peer_id
        ),
        CircuitReqAccepted {
            src_peer_id,
            dst_peer_id,
        } => debug!(
            "Circuit request from {} to peer {} accepted",
            src_peer_id, dst_peer_id
        ),
        CircuitClosed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Circuit from {} to {} closed, error?: {:?}",
            src_peer_id, dst_peer_id, error
        ),
        _ => {}
    }
}
