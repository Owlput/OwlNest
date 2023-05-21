use std::sync::Arc;

pub use libp2p::relay::Config;
use tracing::{debug, info};

use crate::event_bus::prelude::*;
pub type Behaviour = libp2p::relay::Behaviour;
pub type OutEvent = libp2p::relay::Event;

impl Into<ListenedEvent> for OutEvent {
    fn into(self) -> ListenedEvent {
        ListenedEvent::Behaviours(BehaviourEvent::RelayServer(Arc::new(self)))
    }
}

pub fn ev_dispatch(ev: &OutEvent) {
    use libp2p::relay::Event::*;
    match ev {
        ReservationReqAccepted {
            src_peer_id,
            renewed,
        } => debug!(
            "Reservation from {} accepted, IsRenew:{}",
            src_peer_id, renewed
        ),
        ReservationReqAcceptFailed { src_peer_id, error } => info!(
            "Failed to accept reservation from {}, error:{}",
            src_peer_id, error
        ),
        ReservationReqDenied { src_peer_id } => {
            info!("Denied reservation from {}", src_peer_id)
        }
        ReservationReqDenyFailed { src_peer_id, error } => info!(
            "Failed to deny reservation from {}, error:{}",
            src_peer_id, error
        ),
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
        CircuitReqDenyFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Failed to deny circuit request from {} to peer {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        CircuitReqAccepted {
            src_peer_id,
            dst_peer_id,
        } => debug!(
            "Circuit request from {} to peer {} accepted",
            src_peer_id, dst_peer_id
        ),
        CircuitReqOutboundConnectFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Failed to connect the outbound from {} to {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        CircuitReqAcceptFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Failed to accept circuit request from {} to {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        CircuitClosed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Circuit from {} to {} closed, error?: {:?}",
            src_peer_id, dst_peer_id, error
        ),
    }
}
