use libp2p::relay::client;
use tracing::{debug, info};

/// `Behaviour` of libp2p's `relay` protocol.
pub type Behaviour = client::Behaviour;
pub type OutEvent = client::Event;

pub fn ev_dispatch(ev: &OutEvent) {
    match ev {
        OutEvent::ReservationReqAccepted {
            relay_peer_id,
            renewal,
            limit,
        } => debug!(
            "Reservation sent to relay {} has been accepted. IsRenewal:{}, limit:{:?}",
            relay_peer_id, renewal, limit
        ),
        OutEvent::ReservationReqFailed {
            relay_peer_id,
            renewal,
            error,
        } => info!(
            "Reservation sent to relay {} has failed, IsRenewal:{}, error:{:?}",
            relay_peer_id, renewal, error
        ),
        OutEvent::OutboundCircuitEstablished {
            relay_peer_id,
            limit,
        } => debug!(
            "Outbound circuit to relay {} established, limit:{:?}",
            relay_peer_id, limit
        ),
        OutEvent::OutboundCircuitReqFailed {
            relay_peer_id,
            error,
        } => info!(
            "Outbound circuit request to relay {} failed, error:{:?}",
            relay_peer_id, error
        ),
        OutEvent::InboundCircuitEstablished { src_peer_id, limit } => debug!(
            "Inbound circuit from source peer {} established, limit:{:?}",
            src_peer_id, limit
        ),
        OutEvent::InboundCircuitReqDenied { src_peer_id } => {
            info!("An inbound circuit from {} was denied", src_peer_id)
        }
        OutEvent::InboundCircuitReqDenyFailed { src_peer_id, error } => info!(
            "Iutbound circuit from source peer {} can't be denied, error:{:?}",
            src_peer_id, error
        ),
    }
}
