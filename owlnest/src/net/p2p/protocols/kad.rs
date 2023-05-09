use std::str::FromStr;

use libp2p::{
    kad::{record::Key, store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent, QueryResult},
    PeerId,
};
use tokio::sync::oneshot;
use tracing::info;

use crate::{
    event_bus::{
        listener_event::{BehaviourEvent, ListenedEvent},
        EventBusHandle,
    },
    net::p2p::swarm,
};

use self::event_listener::Kind;

pub type Behaviour = Kademlia<MemoryStore>;
pub type Config = KademliaConfig;
pub type OutEvent = KademliaEvent;
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

pub async fn map_in_event(behav: &mut Behaviour, ev_bus_handle: &EventBusHandle, ev: InEvent) {
    let (op, callback) = ev.into_inner();
    match op {
        Op::PeerLookup(peer_id) => {
            let (mut listener_rx, listener_id) = ev_bus_handle
                .add(Kind::OnOutboundQueryProgressed, 4)
                .unwrap();
            let query_id = behav.get_record(Key::new(&peer_id.to_bytes()));
            let ev_bus_handle = ev_bus_handle.clone();
            tokio::spawn(async move {
                let mut results = Vec::new();
                loop {
                    if let Some(v) = listener_rx.recv().await {
                        let ev: OutEvent = v.try_into().unwrap();
                        if let OutEvent::OutboundQueryProgressed {
                            id, result, step, ..
                        } = ev
                        {
                            if query_id != id {
                                continue;
                            }
                            results.push(result);
                            if step.last {
                                let _ = ev_bus_handle
                                    .remove(Kind::OnOutboundQueryProgressed, listener_id)
                                    .unwrap();
                                callback.send(OpResult::PeerLookup(results).into()).unwrap();
                                break;
                            }
                        }
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

impl TryFrom<ListenedEvent> for OutEvent {
    type Error = ();

    fn try_from(value: ListenedEvent) -> Result<Self, Self::Error> {
        if let ListenedEvent::Behaviours(BehaviourEvent::Kad(ev)) = value {
            return Ok(ev);
        }
        Err(())
    }
}
impl Into<ListenedEvent> for OutEvent{
    fn into(self) -> ListenedEvent {
        ListenedEvent::Behaviours(BehaviourEvent::Kad(self))
    }
}
