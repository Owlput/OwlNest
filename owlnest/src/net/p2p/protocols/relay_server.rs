use tracing::{debug,info};

pub use libp2p::relay::v2::relay::Config;
pub type Behaviour = libp2p::relay::v2::relay::Relay;
pub type OutEvent = libp2p::relay::v2::relay::Event;

pub fn ev_dispatch(ev: OutEvent) {
    match ev {
        OutEvent::ReservationReqAccepted {
            src_peer_id,
            renewed,
        } => debug!(
            "Reservation from {} accepted, IsRenew:{}",
            src_peer_id, renewed
        ),
        OutEvent::ReservationReqAcceptFailed { src_peer_id, error } => info!(
            "Failed to accept reservation from {}, error:{}",
            src_peer_id, error
        ),
        OutEvent::ReservationReqDenied { src_peer_id } => {
            info!("Denied reservation from {}", src_peer_id)
        }
        OutEvent::ReservationReqDenyFailed { src_peer_id, error } => info!(
            "Failed to deny reservation from {}, error:{}",
            src_peer_id, error
        ),
        OutEvent::ReservationTimedOut { src_peer_id } => {
            info!("Reservation expired for source peer {}", src_peer_id)
        }
        OutEvent::CircuitReqReceiveFailed { src_peer_id, error } => info!(
            "Broken circuit request from {}, error:{}",
            src_peer_id, error
        ),
        OutEvent::CircuitReqDenied {
            src_peer_id,
            dst_peer_id,
        } => info!(
            "Circuit request from {} to peer {} denied",
            src_peer_id, dst_peer_id
        ),
        OutEvent::CircuitReqDenyFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Failed to deny circuit request from {} to peer {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        OutEvent::CircuitReqAccepted {
            src_peer_id,
            dst_peer_id,
        } => debug!(
            "Circuit request from {} to peer {} accepted",
            src_peer_id, dst_peer_id
        ),
        OutEvent::CircuitReqOutboundConnectFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Failed to connect the outbound from {} to {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        OutEvent::CircuitReqAcceptFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Failed to accept circuit request from {} to {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        OutEvent::CircuitClosed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Circuit from {} to {} closed, error?: {:?}",
            src_peer_id, dst_peer_id, error
        ),
    }
}
