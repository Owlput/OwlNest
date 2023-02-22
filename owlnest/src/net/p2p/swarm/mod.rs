use std::fmt::Debug;

use behaviour::Behaviour;

use super::*;
use behaviour::BehaviourEvent;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, ConnectedPoint},
    Swarm as Libp2pSwarm,
};

mod behaviour;
mod in_event;
pub mod manager;
pub mod out_event;
use in_event::InEvent;

pub use in_event::{Op as SwarmOp, ProtocolInEvent};
pub use manager::Manager;
pub use out_event::{OutEvent, OutEventBundle};
use tracing::{debug, info};

pub type Swarm = Libp2pSwarm<Behaviour>;
type SwarmTransport = Boxed<(PeerId, StreamMuxerBox)>;

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
        let (protocol_tx, mut protocol_rx) = mpsc::channel(buffer_size);
        let manager = Manager {
            swarm_sender: swarm_tx,
            protocol_sender: protocol_tx,
        };
        let (behaviour, transport) = behaviour::Behaviour::new(self.config);
        tokio::spawn(async move {
            let mut swarm =
                libp2p::Swarm::with_tokio_executor(transport, behaviour, ident.get_peer_id());
            loop {
                select! {
                    Some(ev) = swarm_rx.recv() => swarm_op_exec(&mut swarm, ev),
                    Some(ev) = protocol_rx.recv() => map_protocol_ev(&mut swarm, ev),
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
fn swarm_op_exec(swarm: &mut Swarm, ev: swarm::InEvent) {
    match ev {
        InEvent::Dial(addr, callback) => {
            callback.send(swarm.dial(addr)).unwrap();
        }
        InEvent::Listen(addr, callback) => {
            callback.send(swarm.listen_on(addr)).unwrap();
        }
    }
}

#[inline]
fn map_protocol_ev(swarm: &mut Swarm, ev: ProtocolInEvent) {
    match ev {
        #[cfg(feature = "messaging")]
        ProtocolInEvent::Messaging(ev) => swarm.behaviour_mut().messaging.push_event(ev),
        #[cfg(feature = "tethering")]
        ProtocolInEvent::Tethering(ev) => swarm.behaviour_mut().tethering.push_event(ev),
    }
}

fn handle_behaviour_event(swarm: &mut Swarm, ev: BehaviourEvent) {
    match ev {
        BehaviourEvent::Kad(ev) => kad::ev_dispatch(ev),
        BehaviourEvent::Identify(ev) => identify::ev_dispatch(ev),
        BehaviourEvent::Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        #[cfg(feature = "messaging")]
        BehaviourEvent::Messaging(ev) => {
            messaging::ev_dispatch(ev);
        }
        #[cfg(feature = "tethering")]
        BehaviourEvent::Tethering(ev) => {
            tethering::ev_dispatch(ev);
        }
        #[cfg(feature = "relay-server")]
        BehaviourEvent::RelayServer(ev) => relay_server::ev_dispatch(ev),
        #[cfg(feature = "relay-client")]
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
