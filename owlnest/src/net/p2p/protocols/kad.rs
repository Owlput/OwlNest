use libp2p::{kad::*, PeerId};
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::event_bus::{listened_event::Listenable, Handle as EvHandle};

pub type Behaviour = Kademlia<store::MemoryStore>;
pub type Config = KademliaConfig;
pub type OutEvent = KademliaEvent;
pub use libp2p::kad::PROTOCOL_NAME;
pub mod cli;

#[derive(Debug)]
pub(crate) struct InEvent {
    op: Op,
    callback: oneshot::Sender<OpResult>,
}
impl InEvent {
    pub fn new(op: Op) -> (Self, oneshot::Receiver<OpResult>) {
        let (tx, rx) = oneshot::channel();
        (Self { op, callback: tx }, rx)
    }
    pub fn into_inner(self) -> (Op, oneshot::Sender<OpResult>) {
        (self.op, self.callback)
    }
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
}
impl Handle {
    pub(crate) fn new(buffer: usize) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self { sender: tx }, rx)
    }
    pub async fn lookup(&self, peer_id: PeerId) -> Result<Vec<QueryResult>, ()> {
        let (ev, callback) = InEvent::new(Op::PeerLookup(peer_id));
        self.sender.send(ev).await.unwrap();
        match callback.await {
            Ok(result) => match result {
                OpResult::PeerLookup(v) => Ok(v),
                OpResult::BehaviourEvent(_) => unreachable!(),
            },
            Err(_e) => Err(()),
        }
    }
    pub fn blocking_lookup(&self, peer_id: PeerId) -> Result<Vec<QueryResult>, ()> {
        let (ev, callback) = InEvent::new(Op::PeerLookup(peer_id));
        self.sender.blocking_send(ev).unwrap();
        match callback.blocking_recv() {
            Ok(result) => match result {
                OpResult::PeerLookup(v) => Ok(v),
                OpResult::BehaviourEvent(_) => unreachable!(),
            },
            Err(_e) => Err(()),
        }
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

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour, ev_bus_handle: &EvHandle) {
    let (op, callback) = ev.into_inner();
    match op {
        Op::PeerLookup(peer_id) => {
            let mut listener = ev_bus_handle.add(format!("{}", PROTOCOL_NAME)).unwrap();
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
                                    callback.send(OpResult::PeerLookup(results)).unwrap();
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

pub fn ev_dispatch(ev: &OutEvent,ev_tap:&crate::event_bus::bus::EventTap) {
    
    match ev{
        KademliaEvent::InboundRequest { request } => info!("Incoming request: {:?}",request),
        KademliaEvent::OutboundQueryProgressed { id, result, stats, step } => info!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => info!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        KademliaEvent::UnroutablePeer { peer } => info!("Peer {} is now unreachable",peer),
        KademliaEvent::RoutablePeer { peer, address } => info!("Peer {} is reachable with address {}",peer,address),
        KademliaEvent::PendingRoutablePeer { peer, address } => info!("Pending peer {} with address {}",peer,address),
    }
    ev_tap.blocking_send(ev.clone().into_listened()).unwrap();
}

impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        PROTOCOL_NAME.to_string()
    }
}
