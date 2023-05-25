use std::str::FromStr;

use libp2p::{
    kad::{record::Key, store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent, QueryResult},
    PeerId,
};
use tokio::sync::oneshot;
use tracing::{info, warn};

use crate::{event_bus::{Handle, ToEventIdentifier}, net::p2p::swarm};

use self::event_listener::Kind;

pub type Behaviour = Kademlia<MemoryStore>;
pub type Config = KademliaConfig;
pub type OutEvent = KademliaEvent;
pub use libp2p::kad::PROTOCOL_NAME;
pub mod cli;

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
    PeerLookup(Vec<QueryResult>),
    BehaviourEvent(OutEvent),
}
impl Into<swarm::BehaviourOpResult> for OpResult {
    fn into(self) -> swarm::BehaviourOpResult {
        swarm::BehaviourOpResult::Kad(self)
    }
}

pub async fn map_in_event(behav: &mut Behaviour, ev_bus_handle: &Handle, ev: InEvent) {
    let (op, callback) = ev.into_inner();
    match op {
        Op::PeerLookup(peer_id) => {
            let mut listener = ev_bus_handle.add(Kind::OnOutboundQueryProgressed.event_identifier()).unwrap();
            let query_id = behav.get_record(Key::new(&peer_id.to_bytes()));
            tokio::spawn(async move {
                let mut results = Vec::new();
                loop {
                    match listener.recv().await {
                        Ok(ev) => {
                            let ev_ref = ev.downcast_ref::<OutEvent>().expect("downcast to succeed");
                            if let OutEvent::OutboundQueryProgressed {
                                id, result, step, ..
                            } = ev_ref
                            {
                                if query_id != *id {
                                    continue;
                                }
                                results.push(result.clone());
                                if step.last {
                                    drop(listener);
                                    callback.send(OpResult::PeerLookup(results).into()).unwrap();
                                    break;
                                }
                            }
                        }
                        Err(e) => warn!("{:?}", e),
                    }
                }
            });
        }
    }
}

pub fn ev_dispatch(ev: &OutEvent) {
    match ev{
        KademliaEvent::InboundRequest { request } => info!("Incoming request: {:?}",request),
        KademliaEvent::OutboundQueryProgressed { id, result, stats, step } => info!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => info!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        KademliaEvent::UnroutablePeer { peer } => info!("Peer {} is now unreachable",peer),
        KademliaEvent::RoutablePeer { peer, address } => info!("Peer {} is reachable with address {}",peer,address),
        KademliaEvent::PendingRoutablePeer { peer, address } => info!("Pending peer {} with address {}",peer,address),
    }
}

pub mod event_listener;
