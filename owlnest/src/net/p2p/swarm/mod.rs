use crate::net::p2p::swarm::manager::HandleBundle;
use futures::StreamExt;
use libp2p::{Multiaddr, PeerId};
use libp2p_swarm::DialError;
use std::{fmt::Debug, sync::Arc};
use tokio::select;
use tokio::sync::oneshot;
use tracing::{debug, info, trace, warn};

pub mod behaviour;
pub mod cli;
pub mod handle;
pub(crate) mod in_event;
pub mod manager;
pub mod out_event;

pub use in_event::*;
pub use libp2p::core::ConnectedPoint;
pub use libp2p::swarm::ConnectionId;
pub use manager::Manager;
pub type EventSender = tokio::sync::broadcast::Sender<Arc<SwarmEvent>>;

use self::{behaviour::BehaviourEvent, manager::Rx};

use super::SwarmConfig;

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
        use libp2p::kad::store::MemoryStore;
        let kad_store = MemoryStore::new(ident.get_peer_id());
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
                .with_behaviour(|_key, relay| behaviour::Behaviour {
                    kad: kad::Behaviour::new(ident.get_peer_id(), kad_store),
                    mdns: mdns::Behaviour::new(self.config.mdns, ident.get_peer_id()).unwrap(),
                    identify: identify::Behaviour::new(self.config.identify),
                    messaging: messaging::Behaviour::new(self.config.messaging),
                    #[cfg(feature = "tethering")]
                    tethering: tethering::Behaviour::new(self.config.tethering),
                    relay_server: libp2p::relay::Behaviour::new(
                        self.config.local_ident.get_peer_id(),
                        self.config.relay_server,
                    ),
                    relay_client: relay,
                    relay_ext: relay_ext::Behaviour::new(),
                    dcutr: dcutr::Behaviour::new(ident.get_peer_id()),
                    blob_transfer: blob_transfer::Behaviour::new(Default::default()),
                    autonat: autonat::Behaviour::new(ident.get_peer_id(), Default::default()),
                    upnp: upnp::Behaviour::default(),
                    ping:ping::Behaviour::new(Default::default()),
                })
                .expect("behaviour incorporation to succeed")
                .build();
            loop {
                let timer = futures_timer::Delay::new(std::time::Duration::from_millis(200));
                select! {
                    Some(ev) = rx_bundle.next() => handle_incoming_event(ev, &mut swarm),
                    out_event = swarm.select_next_some(), if event_out.len() > 12 => {
                        handle_swarm_event(&out_event,&mut swarm).await;
                        let _ = event_out.send(Arc::new(out_event));
                    }
                    _ = timer => warn!("Slow receiver for swarm events detected.")
                };
            }
        });
        drop(guard);
        manager
    }
}

#[inline]
async fn handle_swarm_event(ev: &SwarmEvent, swarm: &mut Swarm) {
    match ev {
        SwarmEvent::Behaviour(event) => handle_behaviour_event(swarm, event),
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
        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
            use libp2p::TransportError;
            if let DialError::Transport(transport_err) = error {
                let closure =
                    |err: &(Multiaddr, libp2p::TransportError<std::io::Error>)| match &err.1 {
                        TransportError::MultiaddrNotSupported(addr) => {
                            (addr.clone(), "MultiaddrNotSupported".to_string())
                        }
                        TransportError::Other(e) => (err.0.clone(), e.kind().to_string()),
                    };
                let info = transport_err
                    .iter()
                    .map(closure)
                    .collect::<Vec<(Multiaddr, String)>>();
                info!("Outgoing connection error: {:?}", info)
            }

            info!(
                "Outgoing connection error to peer {:?}: {:?}",
                peer_id, error
            );
        }
        SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {:?}", address),
        SwarmEvent::ExpiredListenAddr { address, .. } => {
            info!("Expired listen address: {}", address)
        }
        SwarmEvent::ListenerClosed {
            addresses, reason, ..
        } => trace!("Listener on address {:?} closed: {:?}", addresses, reason),
        SwarmEvent::ListenerError { listener_id, error } => {
            info!("Listener {:?} reported an error {}", listener_id, error)
        }
        SwarmEvent::Dialing { peer_id, .. } => trace!("Dailing peer? {:?}", peer_id),
        SwarmEvent::NewExternalAddrCandidate { address } => {
            info!(
                "A possible external address has been discovered: {}",
                address
            );
        }
        SwarmEvent::ExternalAddrConfirmed { address } => {
            info!(
                "A possible external address has been confirmed: {}",
                address
            );
        }
        SwarmEvent::ExternalAddrExpired { address } => {
            debug!("A possible external address has expired: {}", address);
        }
        _ => unimplemented!("New branch not covered"),
    }
}

#[inline]
fn handle_incoming_event(ev: Rx, swarm: &mut Swarm) {
    use crate::net::p2p::protocols::*;
    use Rx::*;
    match ev {
        Kad(ev) => kad::map_in_event(ev, &mut swarm.behaviour_mut().kad),
        Messaging(ev) => swarm.behaviour_mut().messaging.push_event(ev),
        #[cfg(feature = "tethering")]
        Tethering(ev) => swarm.behaviour_mut().tethering.push_event(ev),
        Mdns(ev) => mdns::map_in_event(ev, &mut swarm.behaviour_mut().mdns),
        Swarm(ev) => swarm_op_exec(swarm, ev),
        RelayExt(ev) => swarm.behaviour_mut().relay_ext.push_event(ev),
        BlobTransfer(ev) => swarm.behaviour_mut().blob_transfer.push_event(ev),
        AutoNat(ev) => autonat::map_in_event(&mut swarm.behaviour_mut().autonat, ev),
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
fn handle_behaviour_event(swarm: &mut Swarm, ev: &BehaviourEvent) {
    use super::protocols::*;
    use behaviour::BehaviourEvent::*;
    match ev {
        Kad(ev) => kad::ev_dispatch(ev),
        Identify(ev) => identify::ev_dispatch(ev),
        Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        #[cfg(feature = "tethering")]
        Tethering(ev) => tethering::ev_dispatch(ev),
        RelayServer(ev) => relay_server::ev_dispatch(ev),
        Dcutr(ev) => dcutr::ev_dispatch(ev),
        AutoNat(ev) => autonat::ev_dispatch(ev),
        Upnp(ev) => upnp::ev_dispatch(ev),
        Ping(ev)=> ping::ev_dispatch(ev),
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
