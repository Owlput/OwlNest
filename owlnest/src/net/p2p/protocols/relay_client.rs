use libp2p::relay::v2::client;
use tracing::{info,debug};

pub type Behaviour = libp2p::relay::v2::client::Client;
pub type OutEvent = libp2p::relay::v2::client::Event;



pub fn ev_dispatch(ev:client::Event){
    match ev{
        client::Event::ReservationReqAccepted { relay_peer_id, renewal, limit } => debug!("Reservation sent to relay {} has been accepted. IsRenewal:{}, limit:{:?}",relay_peer_id,renewal,limit),
        client::Event::ReservationReqFailed { relay_peer_id, renewal, error } => info!("Reservation sent to relay {} has failed, IsRenewal:{}, error:{:?}",relay_peer_id,renewal,error),
        client::Event::OutboundCircuitEstablished { relay_peer_id, limit } => debug!("Outbound circuit to relay {} established, limit:{:?}",relay_peer_id,limit),
        client::Event::OutboundCircuitReqFailed { relay_peer_id, error } => info!("Outbound circuit request to relay {} failed, error:{:?}",relay_peer_id,error),
        client::Event::InboundCircuitEstablished { src_peer_id, limit } => debug!("Inbound circuit from source peer {} established, limit:{:?}",src_peer_id,limit),
        client::Event::InboundCircuitReqFailed { relay_peer_id, error } => info!("Relayed inbound circuit from relay {} failed, error:{:?}",relay_peer_id,error),
        client::Event::InboundCircuitReqDenied { src_peer_id } => info!("An inbound circuit from {} was denied",src_peer_id),
        client::Event::InboundCircuitReqDenyFailed { src_peer_id, error } => info!("Iutbound circuit from source peer {} can't be denied, error:{:?}",src_peer_id,error),
    }
}