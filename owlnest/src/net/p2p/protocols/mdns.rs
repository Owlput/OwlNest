pub use libp2p::mdns::tokio::Behaviour;
use libp2p::PeerId;
pub type OutEvent = libp2p::mdns::Event;
use crate::net::p2p::swarm::Swarm;
pub use libp2p::mdns::Config;
use tokio::sync::oneshot;

pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<OpResult>,
}

pub enum Op {
    ListDiscoveredNodes,
    ForceExpire(PeerId),
    HasNode(PeerId),
}

#[derive(Debug)]
/// Result type that collects all possible results.
pub enum OpResult {
    ListDiscoveredNodes(Vec<PeerId>),
    HasNode(bool),
    ForceExpire(()),
}

pub fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    let InEvent { op, callback } = ev;
    match op {
        Op::ListDiscoveredNodes => {
            let node_list = behav.discovered_nodes().map(|r|r.clone()).collect::<Vec<PeerId>>();
            callback
                .send(OpResult::ListDiscoveredNodes(node_list))
                .unwrap();
        }
        Op::ForceExpire(peer_id) => {
            behav.expire_node(&peer_id);
            callback.send(OpResult::ForceExpire(())).unwrap();
        } 
        Op::HasNode(peer_id) => {
            let has_node = behav.has_node(&peer_id);
            callback.send(OpResult::HasNode(has_node)).unwrap();
        }
    }
}

pub fn ev_dispatch(ev: OutEvent, swarm: &mut Swarm) {
    match ev {
        libp2p::mdns::Event::Discovered(list) => {
            for (peer, addr) in list {
                swarm.behaviour_mut().kad.add_address(&peer, addr);
            }
        }
        libp2p::mdns::Event::Expired(list) => {
            for (peer, addr) in list {
                swarm.behaviour_mut().kad.remove_address(&peer, &addr);
            }
        }
    }
}
