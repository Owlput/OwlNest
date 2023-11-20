use crate::generate_handler_method;
use crate::net::p2p::swarm::EventSender;
use crate::net::p2p::swarm::Swarm;
pub use libp2p::mdns::tokio::Behaviour;
pub use libp2p::mdns::Config;
pub use libp2p::mdns::Event as OutEvent;
use libp2p::PeerId;
use tokio::sync::{mpsc, oneshot::*};

pub enum InEvent {
    ListDiscoveredNodes(Sender<Vec<PeerId>>),
    HasNode(PeerId, Sender<bool>),
}

#[allow(unused)]
macro_rules! event_op {
    ($listener:ident,$pattern:pat,{$($ops:tt)+}) => {
        loop{
            let ev = handle_listener_result!($listener);
            if let SwarmEvent::Behaviour(BehaviourEvent::Mdns($pattern)) = ev.as_ref() {
                $($ops)+
            } else {
                continue;
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    #[allow(unused)]
    event_tx: EventSender,
}
impl Handle {
    pub fn new(
        buffer: usize,
        event_tx: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
            },
            rx,
        )
    }
    generate_handler_method! {
        ListDiscoveredNodes:list_discovered_node()->Vec<PeerId>;
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
        HasNode(peer_id, callback) => {
            let has_node = behav.discovered_nodes().any(|peer| *peer == peer_id);
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

pub(crate) mod cli {
    use std::str::FromStr;

    use libp2p::PeerId;

    use crate::net::p2p::swarm::manager::Manager;

    pub fn handle_mdns(manager: &Manager, command: Vec<&str>) {
        if command.len() < 2 {
            println!("Missing subcommands. Type `mdns help` for more information");
            return;
        }
        match command[1] {
            "list-discovered" => mdns_listdiscovered(manager),
            "has-node" => mdns_hasnode(manager, command),
            "help" => println!("{}", TOP_HELP_MESSAGE),
            _ => println!("Unrecoginzed command. Type `mdns help` for more information"),
        }
    }

    fn mdns_listdiscovered(manager: &Manager) {
        println!(
            "{:?}",
            manager
                .executor()
                .block_on(manager.mdns().list_discovered_node())
        );
    }
    fn mdns_hasnode(manager: &Manager, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing required argument: <peer ID>. Syntax `mdns has-node <peer ID>`");
            return;
        }
        let peer_id = match PeerId::from_str(command[2]) {
            Ok(peer_id) => peer_id,
            Err(e) => {
                println!("Error: Failed parsing peer ID `{}`: {}", command[1], e);
                return;
            }
        };
        let result = manager
            .executor()
            .block_on(manager.mdns().has_node(peer_id));
        println!("Peer {} discovered? {}", peer_id, result)
    }
    const TOP_HELP_MESSAGE: &str = r"";
}
