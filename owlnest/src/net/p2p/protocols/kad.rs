use libp2p::{
    kad::{self, Mode, NoKnownPeers, QueryId, RoutingUpdate},
    Multiaddr, PeerId,
};
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, trace};

use crate::{
    net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent},
    with_timeout,
};

pub use kad::Config;
pub type Behaviour = kad::Behaviour<kad::store::MemoryStore>;
pub type OutEvent = kad::Event;
pub use libp2p::kad::PROTOCOL_NAME;

pub mod cli;

#[derive(Debug)]
pub(crate) enum InEvent {
    PeerLookup(PeerId, oneshot::Sender<kad::QueryId>),
    BootStrap(oneshot::Sender<Result<QueryId, NoKnownPeers>>),
    InsertNode(PeerId, Multiaddr, oneshot::Sender<RoutingUpdate>),
    SetMode(Option<Mode>),
}

macro_rules! event_op {
    ($listener:ident,$pattern:pat,{$($ops:tt)+}) => {
        loop{
            let ev = crate::handle_listener_result!($listener);
            if let SwarmEvent::Behaviour(BehaviourEvent::Kad($pattern)) = ev.as_ref() {
                $($ops)+
            } else {
                continue;
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender_swarm: mpsc::Sender<InEvent>,
    event_tx: EventSender,
}
impl Handle {
    pub(crate) fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender_swarm: tx,
                event_tx: event_tx.clone(),
            },
            rx,
        )
    }
    pub async fn bootstrap(&self) -> Result<(), NoKnownPeers> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::BootStrap(tx);
        self.sender_swarm
            .send(ev)
            .await
            .expect("sending event to succeed");
        rx.await.expect("callback to succeed").map(|_| ())
    }
    pub async fn insert_node(&self, peer_id: PeerId, address: Multiaddr) -> RoutingUpdate {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::InsertNode(peer_id, address, tx);
        self.sender_swarm.send(ev).await.expect("send to succeed");
        rx.await.expect("callback to succeed")
    }
    pub async fn lookup(&self, peer_id: PeerId) -> Vec<kad::QueryResult> {
        let mut listener = self.event_tx.subscribe();
        let (callback_tx, callback_rx) = oneshot::channel();
        self.sender_swarm
            .send(InEvent::PeerLookup(peer_id, callback_tx))
            .await
            .expect("sending event to succeed");
        let query_id = callback_rx.await.expect("callback to succeed");
        let mut results = Vec::new();
        event_op!(listener,OutEvent::OutboundQueryProgressed { id, result, step,.. },{
            if query_id != *id {
                continue;
            }
            results.push(result.clone());
            if step.last {
                drop(listener);
                break;
            }
        });
        results
    }
    async fn set_mode(&self, mode: Option<Mode>) -> Result<Mode, ()> {
        let ev = InEvent::SetMode(mode);
        let mut listener = self.event_tx.subscribe();
        self.sender_swarm.send(ev).await.expect("send to succeed");
        let fut = async move {
            event_op!(listener,OutEvent::ModeChanged { new_mode },{
                break new_mode.clone();
            })
        };
        match with_timeout!(fut, 10) {
            Ok(result) => return Ok(result),
            Err(_) => return Err(()),
        };
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    use InEvent::*;

    match ev {
        PeerLookup(peer_id, callback) => {
            let query_id = behav.get_record(kad::RecordKey::new(&peer_id.to_bytes()));
            callback.send(query_id).expect("callback to succeed")
        }
        BootStrap(callback) => {
            let result = behav.bootstrap();
            callback.send(result).expect("callback to succeed");
        }
        SetMode(mode) => behav.set_mode(mode),
        InsertNode(peer, address, callback) => {
            let result = behav.add_address(&peer, address);
            callback.send(result).expect("callback to succeed");
        }
    }
}

pub fn ev_dispatch(ev: &OutEvent) {
    use kad::Event::*;
    match ev{
        InboundRequest { request } => info!("Incoming request: {:?}",request),
        OutboundQueryProgressed { id, result, stats, step } => debug!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => trace!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        ModeChanged { new_mode } => info!("The mode of this peer has been changed to {}",new_mode),
        _=>{}
    }
}
