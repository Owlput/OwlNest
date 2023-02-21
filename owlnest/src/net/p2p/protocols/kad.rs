use libp2p::kad::{Kademlia, store::MemoryStore, KademliaConfig,KademliaEvent};
use tracing::info;

pub type Behaviour = Kademlia<MemoryStore>;
pub type Config = KademliaConfig;
pub type OutEvent = KademliaEvent;
pub enum InEvent{

}

pub fn map_in_event(_behav:&mut Behaviour,_ev:InEvent){
    
}

pub fn ev_dispatch(ev:OutEvent){
    match ev{
        KademliaEvent::InboundRequest { request } => info!("Incoming request: {:?}",request),
        KademliaEvent::OutboundQueryProgressed { id, result, stats, step } => info!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => info!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        KademliaEvent::UnroutablePeer { peer } => info!("Peer {} is now unreachable",peer),
        KademliaEvent::RoutablePeer { peer, address } => info!("Peer {} is reachable with address {}",peer,address),
        KademliaEvent::PendingRoutablePeer { peer, address } => info!("Pending peer {} with address {}",peer,address),
    }
}