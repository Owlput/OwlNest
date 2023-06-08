use crate::net::p2p::swarm::Swarm;
pub use libp2p::mdns::tokio::Behaviour;
pub use libp2p::mdns::Config;
pub use libp2p::mdns::Event as OutEvent;
use libp2p::PeerId;
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
    Ok,
    ListDiscoveredNodes(Vec<PeerId>),
    HasNode(bool),
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    let InEvent { op, callback } = ev;
    match op {
        Op::ListDiscoveredNodes => {
            let node_list = behav
                .discovered_nodes().copied()
                .collect::<Vec<PeerId>>();
            callback
                .send(OpResult::ListDiscoveredNodes(node_list))
                .unwrap();
        }
        Op::ForceExpire(peer_id) => {
            behav.expire_node(&peer_id);
            callback.send(OpResult::Ok).unwrap();
        }
        Op::HasNode(peer_id) => {
            let has_node = behav.has_node(&peer_id);
            callback.send(OpResult::HasNode(has_node)).unwrap();
        }
    }
}

pub fn ev_dispatch(ev: &OutEvent, swarm: &mut Swarm) {
    match ev.clone() {
        libp2p::mdns::Event::Discovered(list) => {
            list.into_iter()
                .map(|(peer, addr)| swarm.behaviour_mut().kad.add_address(&peer, addr))
                .count();
        }
        libp2p::mdns::Event::Expired(list) => {
            list.into_iter()
                .map(|(peer, addr)| swarm.behaviour_mut().kad.remove_address(&peer, &addr))
                .count();
        }
    }
}
