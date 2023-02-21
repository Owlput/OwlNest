use std::fmt::Debug;

use behaviour::Behaviour;

use super::*;
use behaviour::BehaviourEvent;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    Swarm,
};

mod behaviour;
mod in_event;
pub mod manager;
pub mod out_event;
use in_event::InEvent;

pub use in_event::{Op, ProtocolInEvent};
pub use manager::Manager;
pub use out_event::{OutEvent, OutEventBundle};
use tracing::{debug, info, warn};

type SwarmTransport = Boxed<(PeerId, StreamMuxerBox)>;

pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build(self, buffer_size: usize) -> (Manager, OutEventBundle) {
        let ident = self.config.local_ident.clone();

        let (swarm_tx, mut swarm_rx) = mpsc::channel(buffer_size);
        let (protocol_tx, mut protocol_rx) = mpsc::channel(buffer_size);
        #[cfg(feature = "messaging")]
        let (messaging_dispatch, messaging_rx) = mpsc::channel(8);
        #[cfg(feature = "tethering")]
        let (tethering_dispatch, tethering_rx) = mpsc::channel(8);
        #[cfg(feature = "relay-client")]
        let (_relay_client_dispatch, relay_client_rx) = mpsc::channel(8);
        #[cfg(feature = "relay-server")]
        let (_relay_server_dispatch, relay_server_rx) = mpsc::channel(8);
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

                            SwarmEvent::Behaviour(event)=>{
                                match event {
                                    BehaviourEvent::Kad(ev)=>{kad::ev_dispatch(ev)}
                                    BehaviourEvent::Identify(ev)=>{identify::ev_dispatch(ev)}
                                    BehaviourEvent::Mdns(ev)=>{mdns::ev_dispatch(ev)}
                                    #[cfg(feature="messaging")]
                                    BehaviourEvent::Messaging(ev)=>{messaging::ev_dispatch(ev,&messaging_dispatch).await;}
                                    #[cfg(feature="tethering")]
                                    BehaviourEvent::Tethering(ev)=>{tethering::ev_dispatch(ev, &tethering_dispatch).await;},
                                    #[cfg(feature="relay-server")]
                                    BehaviourEvent::RelayServer(ev) =>{relay_server::ev_dispatch(ev)},
                                    #[cfg(feature="relay-client")]
                                    BehaviourEvent::RelayClient(ev) =>{relay_client::ev_dispatch(ev)}
                                    BehaviourEvent::KeepAlive(_)=>{}
                                }
                            },
                            SwarmEvent::NewListenAddr{address, ..}=>info!("Listening on {:?}",address),
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, concurrent_dial_errors } => debug!("Connection to peer {} established, endpoint(s): {:?}, total connection to that peer: {}, error(s)?: {:?}",peer_id,endpoint,num_established,concurrent_dial_errors),
                            SwarmEvent::ConnectionClosed { .. } => info!("Connection closed"),
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
        let out_bundle = OutEventBundle {
            #[cfg(feature = "messaging")]
            messaging_rx,
            #[cfg(feature = "tethering")]
            tethering_rx,
            #[cfg(feature = "relay-client")]
            relay_client_rx,
            #[cfg(feature = "relay-server")]
            relay_server_rx,
        };
        (manager, out_bundle)
    }
}

#[inline]
fn swarm_op_exec(swarm: &mut Swarm<Behaviour>, ev: swarm::InEvent) {
    match ev {
        InEvent::Dial(addr, callback) => {
            let callback_res = callback.send(swarm.dial(addr));
            handle_callback_res(callback_res)
        }
        InEvent::Listen(addr, callback) => {
            let callback_res = callback.send(swarm.listen_on(addr));
            handle_callback_res(callback_res)
        }
    }
}

#[inline]
fn map_protocol_ev(swarm: &mut Swarm<behaviour::Behaviour>, ev: ProtocolInEvent) {
    match ev {
        #[cfg(feature = "messaging")]
        ProtocolInEvent::Messaging(ev) => {
            swarm.behaviour_mut().messaging.push_event(ev)
        },
        #[cfg(feature = "tethering")]
        ProtocolInEvent::Tethering(ev) => {
            swarm.behaviour_mut().tethering.push_event(ev)
        },
    }
}

#[inline]
fn handle_callback_res<T>(callback_res: Result<(), T>)
where
    T: Debug,
{
    match callback_res {
        Ok(_) => {}
        Err(op_res) => warn!("Failed to send callback {:?}", op_res),
    }
}
