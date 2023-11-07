use crate::{
    event_bus::{self, bus::EventTap},
    net::p2p::swarm::manager::HandleBundle,
};
use futures::StreamExt;
use libp2p::{core::ConnectedPoint, Multiaddr, PeerId};
use std::{fmt::Debug, sync::Arc};
use tokio::select;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

pub mod behaviour;
pub mod cli;
pub(crate) mod in_event;
pub mod manager;
pub mod op;
pub mod out_event;

pub use in_event::*;

use self::{manager::{Manager, Rx}, behaviour::BehaviourEvent};

use super::SwarmConfig;

pub type Swarm = libp2p::Swarm<behaviour::Behaviour>;
pub(crate) type SwarmEvent = libp2p::swarm::SwarmEvent<BehaviourEvent,<<behaviour::Behaviour as libp2p::swarm::NetworkBehaviour>::ConnectionHandler as libp2p::swarm::ConnectionHandler>::Error>;

pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build(
        self,
        buffer_size: usize,
        ev_bus_handle: event_bus::Handle,
        ev_tap: EventTap,
        executor: tokio::runtime::Handle,
    ) -> Manager {
        let ident = self.config.local_ident.clone();

        use crate::net::p2p::protocols::*;
        use libp2p::kad::store::MemoryStore;
        let kad_store = MemoryStore::new(ident.get_peer_id());

        let (handle_bundle, mut rx_bundle) = HandleBundle::new(buffer_size, &ev_bus_handle);
        let manager = manager::Manager::new(Arc::new(handle_bundle), executor);
        let manager_clone = manager.clone();
        tokio::spawn(async move {
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
                .with_relay_client(
                    libp2p_noise::Config::new,
                    libp2p_yamux::Config::default,
                )
                .expect("transport upgrade to succeed")
                .with_behaviour(|_key, relay| behaviour::Behaviour {
                    kad: kad::Behaviour::new(ident.get_peer_id(), kad_store),
                    mdns: mdns::Behaviour::new(self.config.mdns, ident.get_peer_id()).unwrap(),
                    identify: identify::Behaviour::new(self.config.identify),
                    messaging: messaging::Behaviour::new(self.config.messaging),
                    tethering: tethering::Behaviour::new(self.config.tethering),
                    relay_server: libp2p::relay::Behaviour::new(
                        self.config.local_ident.get_peer_id(),
                        self.config.relay_server,
                    ),
                    relay_client:relay,
                    relay_ext: relay_ext::Behaviour::new(),
                    dcutr: dcutr::Behaviour::new(ident.get_peer_id())
                })
                .expect("behaviour incorporation to succeed")
                .build();

            loop {
                select! {
                    Some(ev) = rx_bundle.next() => handle_incoming_event(ev, &mut swarm, &ev_bus_handle),
                    out_event = swarm.select_next_some() => {
                        handle_swarm_event(&out_event,&mut swarm,&ev_tap).await;
                    }
                };
            }
        });
        manager
    }
}

#[inline]
async fn handle_swarm_event(ev: &SwarmEvent, swarm: &mut Swarm, ev_tap: &EventTap) {
    match ev {
        SwarmEvent::Behaviour(event) => handle_behaviour_event(swarm, event, ev_tap).await,
        SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {:?}", address),
        SwarmEvent::ConnectionEstablished {
            peer_id, endpoint, ..
        } => kad_add(swarm, *peer_id, endpoint.clone()),
        SwarmEvent::ConnectionClosed {
            peer_id, endpoint, ..
        } => kad_remove(swarm, *peer_id, endpoint.clone()),
        SwarmEvent::IncomingConnection {
            send_back_addr,
            local_addr,
            ..
        } => debug!(
            "Incoming connection from {} on local address {}",
            send_back_addr, local_addr
        ),
        SwarmEvent::IncomingConnectionError {
            local_addr,
            send_back_addr,
            error,
            ..
        } => info!(
            "Incoming connection error from {} on local address {}, error: {:?}",
            send_back_addr, local_addr, error
        ),
        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => info!(
            "Outgoing connection error to peer {:?}: {:?}",
            peer_id, error
        ),
        SwarmEvent::ExpiredListenAddr { address, .. } => {
            info!("Expired listen address: {}", address)
        }
        SwarmEvent::ListenerClosed {
            addresses, reason, ..
        } => info!("Listener on address {:?} closed: {:?}", addresses, reason),
        SwarmEvent::ListenerError { listener_id, error } => {
            info!("Listener {:?} reported an error {}", listener_id, error)
        }
        SwarmEvent::Dialing { peer_id, .. } => debug!("Dailing peer? {:?}", peer_id),
    }
}

#[inline]
fn handle_incoming_event(ev: Rx, swarm: &mut Swarm, ev_bus_handle: &event_bus::Handle) {
    use crate::net::p2p::protocols::*;
    use Rx::*;
    match ev {
        Kad(ev) => kad::map_in_event(ev, &mut swarm.behaviour_mut().kad, ev_bus_handle),
        Messaging(ev) => swarm.behaviour_mut().messaging.push_event(ev),
        Tethering(ev) => swarm.behaviour_mut().tethering.push_event(ev),
        Mdns(ev) => mdns::map_in_event(ev, &mut swarm.behaviour_mut().mdns),
        Swarm(ev) => swarm_op_exec(swarm, ev),
        _ => {}
    }
}

#[inline]
fn swarm_op_exec(swarm: &mut Swarm, ev: InEvent) {
    use InEvent::*;
    match ev {
        Dial(addr, callback) => {
            let result = swarm.dial(addr);
            handle_callback(callback, result)
        }
        Listen(addr, callback) => {
            let result = swarm.listen_on(addr);
            handle_callback(callback, result)
        }
        AddExternalAddress(addr, callback) => {
            swarm.add_external_address(addr);
            handle_callback(callback, ())
        }
        RemoveExternalAddress(addr, callback) => {
            swarm.remove_external_address(&addr);
            handle_callback(callback, ())
        }
        DisconnectFromPeerId(peer_id, callback) => {
            let result = swarm.disconnect_peer_id(peer_id);
            handle_callback(callback, result)
        }
        ListExternalAddresses(callback) => {
            let addr_list = swarm
                .external_addresses()
                .cloned()
                .collect::<Vec<Multiaddr>>();
            handle_callback(callback, addr_list)
        }
        ListListeners(callback) => {
            let listener_list = swarm.listeners().cloned().collect::<Vec<Multiaddr>>();
            handle_callback(callback, listener_list)
        }
        IsConnectedToPeerId(peer_id, callback) => {
            let result = swarm.is_connected(&peer_id);
            handle_callback(callback, result)
        }
    }
}

#[inline]
async fn handle_behaviour_event(swarm: &mut Swarm, ev: &BehaviourEvent, ev_tap: &EventTap) {
    use super::protocols::*;
    use behaviour::BehaviourEvent::*;
    match ev {
        Kad(ev) => kad::ev_dispatch(ev, ev_tap).await,
        Identify(ev) => identify::ev_dispatch(ev),
        Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        Messaging(ev) => messaging::ev_dispatch(ev,ev_tap).await,
        Tethering(ev) => tethering::ev_dispatch(ev),
        RelayServer(ev) => relay_server::ev_dispatch(ev),
        RelayClient(ev) => relay_client::ev_dispatch(ev),
        _ => {}
    }
}

#[inline]
fn kad_add(swarm: &mut Swarm, peer_id: PeerId, endpoint: ConnectedPoint) {
    match endpoint {
        libp2p::core::ConnectedPoint::Dialer { address, .. } => {
            swarm.behaviour_mut().kad.add_address(&peer_id, address);
        }
        libp2p::core::ConnectedPoint::Listener { send_back_addr, .. } => {
            swarm
                .behaviour_mut()
                .kad
                .add_address(&peer_id, send_back_addr);
        }
    }
}

#[inline]
fn kad_remove(swarm: &mut Swarm, peer_id: PeerId, endpoint: ConnectedPoint) {
    match endpoint {
        libp2p::core::ConnectedPoint::Dialer { address, .. } => {
            swarm.behaviour_mut().kad.remove_address(&peer_id, &address);
        }
        libp2p::core::ConnectedPoint::Listener { send_back_addr, .. } => {
            swarm
                .behaviour_mut()
                .kad
                .remove_address(&peer_id, &send_back_addr);
        }
    }
}

fn handle_callback<T>(callback: oneshot::Sender<T>, result: T)
where
    T: Debug,
{
    if let Err(v) = callback.send(result) {
        warn!("Failed to send callback: {:?}", v)
    }
}

// use futures::{AsyncRead, AsyncWrite};
// fn upgrade_transport<StreamSink>(
//     transport: libp2p::core::transport::Boxed<StreamSink>,
//     keypair: &Keypair,
// ) -> libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>
// where
//     StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
// {
//     libp2p::Transport::upgrade(transport, libp2p::core::upgrade::Version::V1)
//         .authenticate(libp2p::noise::Config::new(keypair).unwrap())
//         .multiplex(libp2p::yamux::Config::default())
//         .boxed()
// }
