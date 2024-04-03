use crate::net::p2p::swarm::manager::HandleBundle;
use futures::StreamExt;
use libp2p::PeerId;
use std::{fmt::Debug, sync::Arc};
use tokio::select;
use tracing::warn;

pub mod behaviour;
pub mod cli;
mod event_handlers;
pub mod handle;
pub mod manager;
pub mod out_event;

pub use libp2p::core::ConnectedPoint;
pub use libp2p::swarm::ConnectionId;
pub use manager::Manager;
pub type EventSender = tokio::sync::broadcast::Sender<Arc<SwarmEvent>>;

use super::SwarmConfig;
use behaviour::BehaviourEvent;
use event_handlers::*;

pub type Swarm = libp2p::Swarm<behaviour::Behaviour>;
pub type SwarmEvent = libp2p::swarm::SwarmEvent<BehaviourEvent>;

pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build(self, buffer_size: usize, executor: tokio::runtime::Handle) -> Manager {
        let guard = executor.enter();
        let ident = self.config.local_ident.clone();
        use crate::net::p2p::protocols::*;
        #[cfg(feature = "libp2p-protocols")]
        let kad_store = libp2p::kad::store::MemoryStore::new(ident.get_peer_id());
        let (swarm_event_out, _) = tokio::sync::broadcast::channel(16);
        let (handle_bundle, mut rx_bundle) = HandleBundle::new(buffer_size, &swarm_event_out);
        let manager = manager::Manager::new(
            Arc::new(handle_bundle),
            ident.clone(),
            executor.clone(),
            swarm_event_out.clone(),
        );
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            let event_out = swarm_event_out;
            let _manager = manager_clone;
            let mut swarm = libp2p::SwarmBuilder::with_existing_identity(ident.get_keypair())
                .with_tokio()
                .with_tcp(
                    Default::default(),
                    libp2p_noise::Config::new,
                    libp2p_yamux::Config::default,
                )
                .expect("transport upgrade to succeed")
                .with_quic()
                .with_dns()
                .expect("upgrade to succeed")
                .with_relay_client(libp2p_noise::Config::new, libp2p_yamux::Config::default)
                .expect("transport upgrade to succeed")
                .with_behaviour(|_key, #[allow(unused)]relay| behaviour::Behaviour {
                    #[cfg(feature = "owlnest-protocols")]
                    blob: blob::Behaviour::new(Default::default()),
                    #[cfg(feature = "owlnest-protocols")]
                    advertise: advertise::Behaviour::new(),
                    #[cfg(feature = "owlnest-protocols")]
                    messaging: messaging::Behaviour::new(self.config.messaging),
                    #[cfg(feature = "libp2p-protocols")]
                    kad: kad::Behaviour::new(ident.get_peer_id(), kad_store),
                    #[cfg(feature = "libp2p-protocols")]
                    mdns: mdns::Behaviour::new(self.config.mdns, ident.get_peer_id()).unwrap(),
                    #[cfg(feature = "libp2p-protocols")]
                    identify: identify::Behaviour::new(self.config.identify),
                    #[cfg(feature = "libp2p-protocols")]
                    relay_server: libp2p::relay::Behaviour::new(
                        self.config.local_ident.get_peer_id(),
                        libp2p::relay::Config {
                            max_circuit_bytes: u64::MAX,
                            ..Default::default()
                        },
                    ),
                    #[cfg(feature = "libp2p-protocols")]
                    relay_client: relay,
                    #[cfg(feature = "libp2p-protocols")]
                    dcutr: dcutr::Behaviour::new(ident.get_peer_id()),
                    #[cfg(feature = "libp2p-protocols")]
                    autonat: autonat::Behaviour::new(ident.get_peer_id(), Default::default()),
                    #[cfg(feature = "libp2p-protocols")]
                    upnp: upnp::Behaviour::default(),
                    #[cfg(feature = "libp2p-protocols")]
                    ping: ping::Behaviour::new(Default::default()),
                    // hyper:hyper::Behaviour::new(Default::default())
                })
                .expect("behaviour incorporation to succeed")
                .build();
            loop {
                let timer = futures_timer::Delay::new(std::time::Duration::from_millis(200));
                select! {
                    Some(ev) = rx_bundle.next() => handle_incoming_event(ev, &mut swarm),
                    out_event = swarm.select_next_some(), if event_out.len() < 12 => {
                        handle_swarm_event(&out_event,&mut swarm).await;
                        let _ = event_out.send(Arc::new(out_event));
                    }
                    _ = timer =>{
                        if event_out.len() > 5 {
                            warn!("Slow receiver for swarm events detected.")
                        }
                    }
                };
            }
        });
        drop(guard);
        manager
    }
}

use libp2p::{Multiaddr, TransportError};
use libp2p_swarm::{derive_prelude::ListenerId, DialError};
use tokio::sync::oneshot::*;

#[derive(Debug)]
pub enum InEvent {
    Dial(Multiaddr, Sender<Result<(), DialError>>),
    Listen(
        Multiaddr,
        Sender<Result<ListenerId, TransportError<std::io::Error>>>,
    ),
    AddExternalAddress(Multiaddr, Sender<()>),
    RemoveExternalAddress(Multiaddr, Sender<()>),
    DisconnectFromPeerId(PeerId, Sender<Result<(), ()>>),
    ListExternalAddresses(Sender<Vec<Multiaddr>>),
    ListListeners(Sender<Vec<Multiaddr>>),
    IsConnectedToPeerId(PeerId, Sender<bool>),
}
