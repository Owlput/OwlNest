use super::*;
use behaviour::Behaviour;
use behaviour::BehaviourEvent;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, ConnectedPoint},
    swarm::{AddressRecord, AddressScore},
    Swarm as Libp2pSwarm,
};
use std::fmt::Debug;
use tracing::{debug, info, warn};

mod behaviour;
pub mod cli;
pub mod event_listener;
mod in_event;
pub mod manager;
pub mod out_event;

pub use event_listener::*;
pub use in_event::*;
pub use manager::Manager;
pub use out_event::OutEvent;

pub type Swarm = Libp2pSwarm<Behaviour>;
type SwarmTransport = Boxed<(PeerId, StreamMuxerBox)>;
type SwarmEvent = libp2p::swarm::SwarmEvent<BehaviourEvent,<<Behaviour as libp2p::swarm::NetworkBehaviour>::ConnectionHandler as libp2p::swarm::ConnectionHandler>::Error>;

pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build(self, buffer_size: usize) -> Manager {
        let ident = self.config.local_ident.clone();

        let (swarm_tx, mut swarm_rx) = mpsc::channel(buffer_size);
        let (ev_hook_tx, mut ev_hook_rx) = mpsc::channel(buffer_size);
        let (protocol_tx, mut protocol_rx) = mpsc::channel(buffer_size);
        let manager = Manager {
            swarm_sender: swarm_tx,
            behaviour_sender: protocol_tx,
            ev_hook_sender: ev_hook_tx,
        };
        let (behaviour, transport) = behaviour::Behaviour::new(self.config);
        let mut manager_cloned = manager.clone();
        tokio::spawn(async move {
            let mut swarm =
                libp2p::Swarm::with_tokio_executor(transport, behaviour, ident.get_peer_id());
            loop {
                select! {
                    Some(ev) = swarm_rx.recv() => swarm_op_exec(&mut swarm, ev),
                    Some(ev) = protocol_rx.recv() => map_protocol_ev(&mut swarm,&mut manager_cloned ,ev),
                    Some(ev) = ev_hook_rx.recv() => handle_ev_hook_op(ev),
                    swarm_event = swarm.select_next_some() => {
                        match swarm_event {
                            SwarmEvent::Behaviour(event)=>handle_behaviour_event(&mut swarm, event),
                            SwarmEvent::NewListenAddr{address, ..}=>info!("Listening on {:?}",address),
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint,.. } => kad_add(&mut swarm,peer_id,endpoint),
                            SwarmEvent::ConnectionClosed { peer_id, endpoint,.. } => kad_remove(&mut swarm, peer_id, endpoint),
                            SwarmEvent::IncomingConnection {send_back_addr, local_addr } => debug!("Incoming connection from {} on local address {}",send_back_addr,local_addr),
                            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error } => info!("Incoming connection error from {} on local address {}, error: {:?}", send_back_addr,local_addr,error),
                            SwarmEvent::OutgoingConnectionError { peer_id, error } => info!("Outgoing connection error to peer {:?}: {:?}",peer_id,error),
                            SwarmEvent::BannedPeer { peer_id, endpoint } => info!("Banned peer {} tried to connect to endpoint {:?}",peer_id,endpoint),
                            SwarmEvent::ExpiredListenAddr { address,.. } => info!("Expired listen address: {}",address),
                            SwarmEvent::ListenerClosed { addresses, reason, .. } => info!("Listener on address {:?} closed: {:?}",addresses, reason),
                            SwarmEvent::ListenerError { listener_id, error } => info!("Listener {:?} reported an error {}",listener_id, error),
                            SwarmEvent::Dialing(peer_id) => debug!("Dailing peer {}",peer_id),

                        }
                    }
                };
            }
        });
        manager
    }
}

#[inline]
fn swarm_op_exec(swarm: &mut Swarm, ev: in_event::InEvent) {
    let (op, callback) = ev.into_inner();
    match op {
        Op::Dial(addr) => {
            let result = OpResult::Dial(swarm.dial(addr.clone()).map_err(|e| e.into()));
            handle_callback(callback, result)
        }
        Op::Listen(addr) => {
            let result = OpResult::Listen(swarm.listen_on(addr.clone()).map_err(|e| e.into()));
            handle_callback(callback, result)
        }
        Op::AddExternalAddress(addr, score) => {
            let score = match score {
                Some(v) => AddressScore::Finite(v),
                None => AddressScore::Infinite,
            };
            let result = match swarm.add_external_address(addr.clone(), score.clone()) {
                libp2p::swarm::AddAddressResult::Inserted { .. } => {
                    OpResult::AddExternalAddress(in_event::AddExternalAddressResult::Inserted)
                }
                libp2p::swarm::AddAddressResult::Updated { .. } => {
                    OpResult::AddExternalAddress(in_event::AddExternalAddressResult::Updated)
                }
            };
            handle_callback(callback, result)
        }
        Op::RemoveExternalAddress(addr) => {
            let result = OpResult::RemoveExternalAddress(swarm.remove_external_address(&addr));
            handle_callback(callback, result)
        }
        Op::BanByPeerId(peer_id) => {
            swarm.ban_peer_id(peer_id.clone());
            let result = OpResult::BanByPeerId;
            handle_callback(callback, result)
        }
        Op::UnbanByPeerId(peer_id) => {
            swarm.unban_peer_id(peer_id.clone());
            let result = OpResult::UnbanByPeerId;
            handle_callback(callback, result)
        }
        Op::DisconnectFromPeerId(peer_id) => {
            let result = OpResult::DisconnectFromPeerId(swarm.disconnect_peer_id(peer_id.clone()));
            handle_callback(callback, result)
        }
        Op::ListExternalAddresses => {
            let addr_list = swarm
                .external_addresses()
                .map(|record| record.clone())
                .collect::<Vec<AddressRecord>>();
            let result = OpResult::ListExternalAddresses(
                addr_list.into_iter().map(|v| v.into()).collect::<_>(),
            );
            handle_callback(callback, result)
        }
        Op::ListListeners => {
            let listener_list = swarm
                .listeners()
                .map(|addr| addr.clone())
                .collect::<Vec<Multiaddr>>();
            let result = OpResult::ListListeners(listener_list);
            handle_callback(callback, result)
        }
        Op::IsConnectedToPeerId(peer_id) => {
            let result = OpResult::IsConnectedToPeerId(swarm.is_connected(&peer_id));
            handle_callback(callback, result)
        }
    }
}

#[inline]
fn map_protocol_ev(swarm: &mut Swarm, manager: &mut Manager, ev: BehaviourInEvent) {
    match ev {
        BehaviourInEvent::Messaging(ev) => swarm.behaviour_mut().messaging.push_event(ev),

        BehaviourInEvent::Tethering(ev) => swarm.behaviour_mut().tethering.push_event(ev),
        BehaviourInEvent::Kad(ev) => kad::map_in_event(&mut swarm.behaviour_mut().kad, manager, ev),
    }
}

#[inline]
fn handle_behaviour_event(swarm: &mut Swarm, ev: BehaviourEvent) {
    match ev {
        BehaviourEvent::Kad(ev) => kad::ev_dispatch(ev),
        BehaviourEvent::Identify(ev) => identify::ev_dispatch(ev),
        BehaviourEvent::Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        BehaviourEvent::Messaging(ev) => messaging::ev_dispatch(ev),
        BehaviourEvent::Tethering(ev) => tethering::ev_dispatch(ev),
        BehaviourEvent::RelayServer(ev) => relay_server::ev_dispatch(ev),
        BehaviourEvent::RelayClient(ev) => relay_client::ev_dispatch(ev),
        BehaviourEvent::KeepAlive(_) => {}
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

#[inline]
fn handle_callback<T>(callback: oneshot::Sender<T>, result: T)
where
    T: Debug,
{
    if let Err(v) = callback.send(result) {
        warn!("Failed to send callback: {:?}", v)
    }
}

#[inline]
fn handle_ev_hook_op(ev: EventListenerOp) {
    match ev {
        EventListenerOp::Add(_) => todo!(),
        EventListenerOp::Remove(_) => todo!(),
    }
}
