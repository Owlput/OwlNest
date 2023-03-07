use std::str::FromStr;

use libp2p::{
    kad::{record::Key, store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent},
    PeerId,
};
use tokio::sync::oneshot;
use tracing::{info, warn};

use crate::net::p2p::swarm;

pub type Behaviour = Kademlia<MemoryStore>;
pub type Config = KademliaConfig;
pub type OutEvent = KademliaEvent;

#[derive(Debug)]
pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<swarm::BehaviourOpResult>,
}
impl InEvent {
    pub fn new(op: Op, callback: oneshot::Sender<swarm::BehaviourOpResult>) -> Self {
        Self { op, callback }
    }
    pub fn into_inner(self) -> (Op, oneshot::Sender<swarm::BehaviourOpResult>) {
        (self.op, self.callback)
    }
}

#[derive(Debug)]
pub enum Op {
    PeerLookup(PeerId),
}

#[derive(Debug)]
pub enum OpResult {
    BehaviourEvent(OutEvent),
}

pub fn map_in_event(behav: &mut Behaviour, manager: &mut swarm::Manager, ev: InEvent) {
    let (op, callback) = ev.into_inner();
    match op {
        Op::PeerLookup(peer_id) => {
            let query_id = behav.get_record(Key::new(&peer_id.to_bytes()));
            if let Err(v) = callback.send(todo!()) {
                warn!(
                    "Receiver dropped when sending callback `PeerLookup` with {}: {:?}",
                    peer_id, v
                )
            }
        }
    }
}

pub enum EventListener {
    PeerLookup(OutEvent),
}

pub fn ev_dispatch(ev: OutEvent) {
    match ev{
        KademliaEvent::InboundRequest { request } => info!("Incoming request: {:?}",request),
        KademliaEvent::OutboundQueryProgressed { id, result, stats, step } => info!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => info!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        KademliaEvent::UnroutablePeer { peer } => info!("Peer {} is now unreachable",peer),
        KademliaEvent::RoutablePeer { peer, address } => info!("Peer {} is reachable with address {}",peer,address),
        KademliaEvent::PendingRoutablePeer { peer, address } => info!("Pending peer {} with address {}",peer,address),
    }
}

pub fn handle_kad(manager: &swarm::Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Missing subcommands. Type \"kad help\" for more information")
    }
    match command[1] {
        "lookup" => handle_kad_lookup(manager, command),
        "help" => println!("{}", TOP_HELP_MESSAGE),
        _ => println!("Unrecoginzed subcommands. Type \"kad help\" for more information"),
    }
}

fn handle_kad_lookup(manager: &swarm::Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing required argument: <peer ID>")
    }
    let peer_id = match PeerId::from_str(command[2]) {
        Ok(peer_id) => peer_id,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[1], e);
            return;
        }
    };
    let (tx, rx) = oneshot::channel::<swarm::BehaviourOpResult>();
    manager.blocking_behaviour_exec(swarm::BehaviourOp::Kad(Op::PeerLookup(peer_id)));
    match rx.blocking_recv() {
        Ok(v) => println!("Lookup request sent. Query ID: {:?}", v),
        Err(_) => {
            warn!("Sender dropped when waiting for callback");
            println!("Error: unable to receive callback - sender dropped")
        }
    }
}

const TOP_HELP_MESSAGE: &str = r#"
Protocol `/ipfs/kad/1.0.0`

Available Subcommands:
lookup <peer ID>        
                Initiate a lookup for the given peer.
"#;

pub mod event_listener;