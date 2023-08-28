use crate::{event_bus, net::p2p::swarm::manager::HandleBundle};
use futures::StreamExt;
use libp2p::{core::ConnectedPoint, Multiaddr, PeerId};
use std::{fmt::Debug, sync::Arc};
use tokio::select;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

pub mod behaviour;
pub mod cli;
pub mod event_listener;
pub(crate) mod in_event;
pub mod manager;
pub mod op;
pub mod out_event;

pub use event_listener::*;
pub use in_event::*;
pub use out_event::ToSwarmEvent;

use self::manager::{Manager, Rx};

use super::SwarmConfig;

pub type Swarm = libp2p::Swarm<behaviour::Behaviour>;
pub(crate) type SwarmEvent = libp2p::swarm::SwarmEvent<ToSwarmEvent,<<behaviour::Behaviour as libp2p::swarm::NetworkBehaviour>::ConnectionHandler as libp2p::swarm::ConnectionHandler>::Error>;

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
        ev_tap: event_bus::bus::EventTap,
    ) -> Manager {
        let ident = self.config.local_ident.clone();

        use crate::net::p2p::protocols::*;
        use libp2p::kad::store::MemoryStore;
        let kad_store = MemoryStore::new(ident.get_peer_id());
        let (relayed_transport, relay_client) = libp2p::relay::client::new(ident.get_peer_id());
        let behaviour = behaviour::Behaviour {
            kad: kad::Behaviour::new(ident.get_peer_id(), kad_store),
            mdns: mdns::Behaviour::new(self.config.mdns, ident.get_peer_id()).unwrap(),
            identify: identify::Behaviour::new(self.config.identify),
            messaging: messaging::Behaviour::new(self.config.messaging),
            tethering: tethering::Behaviour::new(self.config.tethering),
            relay_server: libp2p::relay::Behaviour::new(
                self.config.local_ident.get_peer_id(),
                self.config.relay_server,
            ),
            relay_client,
        };
        let transport = upgrade_transport(
            libp2p::Transport::boxed(libp2p::core::transport::OrTransport::new(
                libp2p::tcp::tokio::Transport::default(),
                relayed_transport,
            )),
            &ident,
        );

        let (handle_bundle, mut rx_bundle) = HandleBundle::new(buffer_size);

        tokio::spawn(async move {
            let mut swarm = libp2p::swarm::SwarmBuilder::with_tokio_executor(
                transport,
                behaviour,
                ident.get_peer_id(),
            )
            .build();
            loop {
                select! {
                    Some(ev) = rx_bundle.next() => handle_incoming_event(ev, &mut swarm, &ev_bus_handle),
                    out_event = swarm.select_next_some() => {
                        handle_swarm_event(&out_event,&mut swarm);
                        if let libp2p_swarm::SwarmEvent::Behaviour(ev) = out_event{
                            ev_tap.send(ev.into()).await.unwrap()
                        }
                    }
                };
            }
        });
        manager::Manager::new(Arc::new(handle_bundle))
    }
}

#[inline]
fn handle_swarm_event(ev: &SwarmEvent, swarm: &mut Swarm) {
    match ev {
        SwarmEvent::Behaviour(event) => handle_behaviour_event(swarm, event),
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
fn handle_behaviour_event(swarm: &mut Swarm, ev: &ToSwarmEvent) {
    use super::protocols::*;
    use out_event::ToSwarmEvent::*;
    match ev {
        Kad(ev) => kad::ev_dispatch(ev),
        Identify(ev) => identify::ev_dispatch(ev),
        Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        Messaging(ev) => messaging::ev_dispatch(ev),
        Tethering(ev) => tethering::ev_dispatch(ev),
        RelayServer(ev) => relay_server::ev_dispatch(ev),
        RelayClient(ev) => relay_client::ev_dispatch(ev),
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

use futures::{AsyncRead, AsyncWrite};
fn upgrade_transport<StreamSink>(
    transport: libp2p::core::transport::Boxed<StreamSink>,
    ident: &super::identity::IdentityUnion,
) -> libp2p::core::transport::Boxed<(PeerId, libp2p::core::muxing::StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    libp2p::Transport::upgrade(transport, libp2p::core::upgrade::Version::V1)
        .authenticate(libp2p::noise::Config::new(&ident.get_keypair()).unwrap())
        .multiplex(libp2p::yamux::Config::default())
        .boxed()
}
