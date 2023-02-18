use behaviour::Behaviour;

use super::*;
use behaviour::BehaviourEvent;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    swarm::AddAddressResult,
    Swarm,
};

mod behaviour;
pub mod in_event;
pub mod manager;
pub mod out_event;

pub use in_event::{InEvent, Op, OpResult};
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
        #[cfg(feature = "messaging")]
        let (messaging_in_tx, mut messaging_in_rx) = mpsc::channel(buffer_size);
        #[cfg(feature = "tethering")]
        let (tethering_in_tx, mut tethering_in_rx) = mpsc::channel(buffer_size);
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
            #[cfg(feature = "messaging")]
            messaging_sender: messaging_in_tx,
            #[cfg(feature = "tethering")]
            tethering_sender: tethering_in_tx,
        };
        let (behaviour, transport) = behaviour::Behaviour::new(self.config);
        tokio::spawn(async move 
        {
            let mut swarm =
                libp2p::Swarm::with_tokio_executor(transport, behaviour, ident.get_peer_id());
            loop {
                #[cfg(feature = "full")]
                {
                select! {
                    Some(ev) = swarm_rx.recv() => swarm_op_exec(&mut swarm, ev),
                    Some(op) = messaging_in_rx.recv() => swarm.behaviour_mut().message_op_exec(op),
                    Some(op) = tethering_in_rx.recv() => swarm.behaviour_mut().tether_op_exec(op),
                    swarm_event = swarm.select_next_some() => {
                        match swarm_event {
                            SwarmEvent::NewListenAddr{address, ..}=>info!("Listening on {:?}",address),
                            SwarmEvent::Behaviour(event)=>{
                                match event {
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
                };}
                #[cfg(not(feature = "full"))]
                {
                select! {
                    Some(ev) = swarm_rx.recv() => swarm_op_exec(&mut swarm, ev),
                    Some(op) = tethering_in_rx.recv() => swarm.behaviour_mut().tether_op_exec(op),
                    swarm_event = swarm.select_next_some() =>
                    {
                        match swarm_event{
                            SwarmEvent::NewListenAddr{address, ..}=>info!("Listening on {:?}",address),
                            SwarmEvent::Behaviour(event)=>{
                                match event {
                                    BehaviourEvent::Tethering(ev)=>{tethering::ev_dispatch(ev, &tethering_dispatch).await;},
                                    BehaviourEvent::KeepAlive(_)=>{} 
                                }
                            },
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
                }
            }
            }
        }
    );
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

fn swarm_op_exec(swarm: &mut Swarm<Behaviour>, ev: swarm::InEvent) {
    let swarm::InEvent { op, callback } = ev;
    match op {
        Op::Dial(addr) => {
            let callback_res = match swarm.dial(addr) {
                Ok(_) => callback.send(OpResult::DialOk),
                Err(e) => callback.send(OpResult::DialErr(e)),
            };
            handle_callback_res(callback_res)
        }
        Op::Listen(addr) => {
            let callback_res = match swarm.listen_on(addr) {
                Ok(_) => callback.send(OpResult::ListenOk),
                Err(e) => callback.send(OpResult::ListenErr(e)),
            };
            handle_callback_res(callback_res)
        }
        Op::AddExternalAddress(addr, score) => {
            let score = if score == "INF".to_string() {
                libp2p::swarm::AddressScore::Infinite
            } else {
                match score.parse::<u32>() {
                    Ok(score) => libp2p::swarm::AddressScore::Finite(score),
                    Err(_) => {
                        let callback_res = callback.send(OpResult::AddExternalAddress(
                            "Invalid score, accepting valid `u32` or str `INF`.".into(),
                        ));
                        handle_callback_res(callback_res);
                        return;
                    }
                }
            };
            let callback_res = match swarm.add_external_address(addr, score) {
                AddAddressResult::Inserted { .. } => {
                    callback.send(OpResult::AddExternalAddress("Inserted".into()))
                }
                AddAddressResult::Updated { .. } => {
                    callback.send(OpResult::AddExternalAddress("Updated".into()))
                }
            };
            handle_callback_res(callback_res)
        }
    }
}

#[inline]
fn handle_callback_res(callback_res: Result<(), OpResult>) {
    match callback_res {
        Ok(_) => {}
        Err(op_res) => warn!("Failed to send callback {:?}", op_res),
    }
}
