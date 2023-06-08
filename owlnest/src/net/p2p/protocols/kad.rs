use crate::net::p2p::swarm;
use crate::net::p2p::swarm::op::behaviour::{self, CallbackSender};
use libp2p::{kad::*, PeerId};
use std::str::FromStr;
use tracing::{info, warn};

use crate::event_bus::{Handle, ToEventIdentifier};

use self::event_listener::Kind;

pub type Behaviour = Kademlia<store::MemoryStore>;
pub type Config = KademliaConfig;
pub type OutEvent = KademliaEvent;
pub use libp2p::kad::PROTOCOL_NAME;
pub mod cli;

#[derive(Debug)]
pub struct InEvent {
    op: Op,
    callback: CallbackSender,
}
impl InEvent {
    pub fn new(op: Op, callback: CallbackSender) -> Self {
        Self { op, callback }
    }
    pub fn into_inner(self) -> (Op, CallbackSender) {
        (self.op, self.callback)
    }
}
impl Into<swarm::in_event::behaviour::InEvent> for InEvent {
    fn into(self) -> swarm::in_event::behaviour::InEvent {
        swarm::in_event::behaviour::InEvent::Kad(self)
    }
}

#[derive(Debug)]
pub enum Op {
    PeerLookup(PeerId),
}
impl Into<behaviour::Op> for Op {
    fn into(self) -> behaviour::Op {
        behaviour::Op::Kad(self)
    }
}

#[derive(Debug)]
pub enum OpResult {
    PeerLookup(Vec<QueryResult>),
    BehaviourEvent(OutEvent),
}
impl Into<behaviour::OpResult> for OpResult {
    fn into(self) -> behaviour::OpResult {
        behaviour::OpResult::Kad(self)
    }
}

pub async fn map_in_event(behav: &mut Behaviour, ev_bus_handle: &Handle, ev: InEvent) {
    let (op, callback) = ev.into_inner();
    match op {
        Op::PeerLookup(peer_id) => {
            let mut listener = ev_bus_handle
                .add(Kind::OnOutboundQueryProgressed.event_identifier())
                .unwrap();
            let query_id = behav.get_record(record::Key::new(&peer_id.to_bytes()));
            tokio::spawn(async move {
                let mut results = Vec::new();
                loop {
                    match listener.recv().await {
                        Ok(ev) => {
                            let ev_ref =
                                ev.downcast_ref::<OutEvent>().expect("downcast to succeed");
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
