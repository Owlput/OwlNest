use libp2p::relay::client;
use tracing::{debug, info};

/// `Behaviour` of libp2p's `relay` protocol.
pub use client::Behaviour;
pub use client::Event as OutEvent;


pub fn ev_dispatch(ev: &client::Event) {
    use client::Event::*;
    match ev {
        ReservationReqAccepted {
            relay_peer_id,
            renewal,
            limit,
        } => debug!(
            "Reservation sent to relay {} has been accepted. IsRenewal:{}, limit:{:?}",
            relay_peer_id, renewal, limit
        ),
        ReservationReqFailed {
            relay_peer_id,
            renewal,
            error,
        } => info!(
            "Reservation sent to relay {} has failed, IsRenewal:{}, error:{:?}",
            relay_peer_id, renewal, error
        ),
        OutboundCircuitEstablished {
            relay_peer_id,
            limit,
        } => debug!(
            "Outbound circuit to relay {} established, limit:{:?}",
            relay_peer_id, limit
        ),
        OutboundCircuitReqFailed {
            relay_peer_id,
            error,
        } => info!(
            "Outbound circuit request to relay {} failed, error:{:?}",
            relay_peer_id, error
        ),
        InboundCircuitEstablished { src_peer_id, limit } => debug!(
            "Inbound circuit from source peer {} established, limit:{:?}",
            src_peer_id, limit
        ),
        InboundCircuitReqDenied { src_peer_id } => {
            info!("An inbound circuit from {} was denied", src_peer_id)
        }
        InboundCircuitReqDenyFailed { src_peer_id, error } => info!(
            "Iutbound circuit from source peer {} can't be denied, error:{:?}",
            src_peer_id, error
        ),
    }
}
