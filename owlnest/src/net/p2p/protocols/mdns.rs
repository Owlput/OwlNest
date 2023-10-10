use crate::event_bus::listened_event::Listenable;
use crate::generate_handler_method;
use crate::net::p2p::swarm::Swarm;
pub use libp2p::mdns::tokio::Behaviour;
pub use libp2p::mdns::Config;
pub use libp2p::mdns::Event as OutEvent;
use libp2p::PeerId;
use tokio::sync::{mpsc, oneshot::*};

pub enum InEvent {
    ListDiscoveredNodes(Sender<Vec<PeerId>>),
    ForceExpire(PeerId, Sender<()>),
    HasNode(PeerId, Sender<bool>),
}
impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        "/libp2p/mdns:OutEvent".into()
    }
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    #[allow(unused)]
    event_bus_handle: crate::event_bus::Handle,
}
impl Handle {
    pub fn new(
        buffer: usize,
        event_bus_handle: &crate::event_bus::Handle,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_bus_handle: event_bus_handle.clone(),
            },
            rx,
        )
    }
    generate_handler_method! {
        ListDiscoveredNodes:list_discovered_node()->Vec<PeerId>;
        ForceExpire:force_expire(peer_id:PeerId)->();
        HasNode:has_node(peer_id:PeerId)->bool;
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    use InEvent::*;
    match ev {
        ListDiscoveredNodes(callback) => {
            let node_list = behav.discovered_nodes().copied().collect::<Vec<PeerId>>();
            callback.send(node_list).unwrap();
        }
        ForceExpire(peer_id, callback) => {
            behav.expire_node(&peer_id);
            callback.send(()).unwrap();
        }
        HasNode(peer_id, callback) => {
            let has_node = behav.has_node(&peer_id);
            callback.send(has_node).unwrap();
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
