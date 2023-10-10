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
pub(crate) enum InEvent {
    PeerLookup(PeerId, oneshot::Sender<QueryId>),
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_bus_handle: EvHandle,
}
impl Handle {
    pub(crate) fn new(buffer: usize, ev_bus_handle: &EvHandle) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_bus_handle: ev_bus_handle.clone(),
            },
            rx,
        )
    }
    pub async fn lookup(&self, peer_id: PeerId) -> Vec<QueryResult> {
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .expect("listener registration to succeed");
        let (callback_tx, callback_rx) = oneshot::channel();
        self.sender
            .send(InEvent::PeerLookup(peer_id, callback_tx))
            .await
            .expect("sending event to succeed");
        let query_id = callback_rx.await.expect("callback to succeed");
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
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("{:?}", e);
                    break;
                }
            }
        }
        results
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour, _ev_bus_handle: &EvHandle) {
    use InEvent::*;
    match ev {
        PeerLookup(peer_id, callback) => {
            let query_id = behav.get_record(record::Key::new(&peer_id.to_bytes()));
            callback.send(query_id).expect("callback to succeed")
        }
    }
}

pub async fn ev_dispatch(ev: &OutEvent, ev_tap: &crate::event_bus::bus::EventTap) {
    match ev{
        KademliaEvent::InboundRequest { request } => info!("Incoming request: {:?}",request),
        KademliaEvent::OutboundQueryProgressed { id, result, stats, step } => info!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => info!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        KademliaEvent::UnroutablePeer { peer } => info!("Peer {} is now unreachable",peer),
        KademliaEvent::RoutablePeer { peer, address } => info!("Peer {} is reachable with address {}",peer,address),
        KademliaEvent::PendingRoutablePeer { peer, address } => info!("Pending peer {} with address {}",peer,address),
    }
    ev_tap.send(ev.clone().into_listened()).await.unwrap();
}

impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        PROTOCOL_NAME.to_string()
    }
}
