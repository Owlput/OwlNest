use crate::net::p2p::swarm::SwarmEvent;
pub use libp2p::autonat::Behaviour;
pub use libp2p::autonat::Config;
pub use libp2p::autonat::Event as OutEvent;
pub use libp2p::autonat::NatStatus;
use libp2p::{Multiaddr, PeerId};
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::info;

#[derive(Debug)]
pub(crate) enum InEvent {
    AddServer(PeerId, Option<Multiaddr>),
    RemoveServer(PeerId),
    Probe(Multiaddr),
    GetNatStatus(oneshot::Sender<(NatStatus, usize)>),
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Handle {
    swarm_source: broadcast::Sender<Arc<SwarmEvent>>,
    sender: mpsc::Sender<InEvent>,
}
impl Handle {
    pub(crate) fn new(
        buffer: usize,
        swarm_source: &broadcast::Sender<Arc<SwarmEvent>>,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                swarm_source: swarm_source.clone(),
                sender: tx,
            },
            rx,
        )
    }
    generate_handler_method!(
        AddServer:add_server(peer:PeerId,address:Option<Multiaddr>,)->();
        RemoveServer:remove_server(peer:PeerId)->();
        Probe:probe(candidate:Multiaddr)->();
    );
    generate_handler_method!(GetNatStatus:get_nat_status()->(NatStatus,usize););
}

pub(crate) fn map_in_event(behaviour: &mut Behaviour, ev: InEvent) {
    match ev {
        InEvent::AddServer(peer, address) => behaviour.add_server(peer, address),
        InEvent::RemoveServer(peer) => behaviour.remove_server(&peer),
        InEvent::GetNatStatus(callback) => {
            handle_callback_sender!((behaviour.nat_status(),behaviour.confidence())=>callback)
        }
        InEvent::Probe(candidate) => behaviour.probe_address(candidate),
    }
}

pub(crate) fn ev_dispatch(ev: &OutEvent) {
    match ev {
        OutEvent::InboundProbe(probe) => info!("Incoming probe: {:?}", probe),
        OutEvent::OutboundProbe(probe) => info!("Probe sent: {:?}", probe),
        OutEvent::StatusChanged { old, new } => {
            info!("Nat statue changed from {:?} to {:?}", old, new)
        }
    }
}
