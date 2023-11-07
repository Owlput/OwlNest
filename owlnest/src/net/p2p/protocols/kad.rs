use libp2p::{
    kad::{self, Mode, NoKnownPeers, QueryId},
    PeerId,
};
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

use crate::{
    event_bus::{listened_event::Listenable, Handle as EvHandle},
    single_value_filter, with_timeout,
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
    SetMode(Option<Mode>),
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender_swarm: mpsc::Sender<InEvent>,
    event_bus_handle: EvHandle,
}
impl Handle {
    pub(crate) fn new(buffer: usize, ev_bus_handle: &EvHandle) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender_swarm: tx,
                event_bus_handle: ev_bus_handle.clone(),
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
    pub async fn lookup(&self, peer_id: PeerId) -> Vec<kad::QueryResult> {
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .expect("listener registration to succeed");
        let (callback_tx, callback_rx) = oneshot::channel();
        self.sender_swarm
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
    async fn set_mode(&self, mode: Option<Mode>) -> Result<Mode, ()> {
        let ev = InEvent::SetMode(mode);
        let mut listener = self
            .event_bus_handle
            .add(OutEvent::as_event_identifier())
            .await
            .expect("listener registration to succeed");
        self.sender_swarm.send(ev).await.expect("send to succeed");
        let fut = single_value_filter!(listener::<OutEvent>, |ev| {
            if let OutEvent::ModeChanged { .. } = ev {
                return true;
            }
            false
        });
        let ev = match with_timeout!(fut,10){
            Ok(result)=> result.expect("listen to succeed"),
            Err(_) => return Err(())
        };
        if let OutEvent::ModeChanged { new_mode } = ev{
            return Ok(new_mode)
        }
        unreachable!()
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour, _ev_bus_handle: &EvHandle) {
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
        SetMode(mode) => {
            behav.set_mode(mode)
        }
    }
}

pub async fn ev_dispatch(ev: &OutEvent, ev_tap: &crate::event_bus::bus::EventTap) {
    use kad::Event::*;
    match ev{
        InboundRequest { request } => info!("Incoming request: {:?}",request),
        OutboundQueryProgressed { id, result, stats, step } => info!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => info!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        UnroutablePeer { peer } => info!("Peer {} is now unreachable",peer),
        RoutablePeer { peer, address } => info!("Peer {} is reachable with address {}",peer,address),
        PendingRoutablePeer { peer, address } => info!("Pending peer {} with address {}",peer,address),
        ModeChanged { new_mode } => info!("The mode of this peer has been changed to {}",new_mode)
    }
    ev_tap.send(ev.clone().into_listened()).await.unwrap();
}

impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        PROTOCOL_NAME.to_string()
    }
}
