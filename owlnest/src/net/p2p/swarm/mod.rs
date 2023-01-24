use self::behaviour::Behaviour;

use super::*;
use behaviour::BehaviourEvent;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    Swarm, swarm::AddAddressResult,
};

pub mod behaviour;
pub mod in_event;
pub mod out_event;

pub use in_event::{InEvent, OpResult};
pub use out_event::{OutEvent, OutEventBundle};

type SwarmTransport = Boxed<(PeerId, StreamMuxerBox)>;

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    sender: mpsc::Sender<in_event::InEvent>,
}
impl Manager {
    pub async fn execute(&self, op: InEvent) -> OpResult {
        match self.sender.send(op).await {
            Ok(_) => OpResult::EmitOk,
            Err(e) => OpResult::EmitErr(e),
        }
    }
}

pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build(self, buffer_size: usize) -> (Manager, OutEventBundle) {
        let ident = self.config.local_ident.clone();
        let (op_tx, mut op_rx) = mpsc::channel(buffer_size);
        let (behaviour, transport) = behaviour::Behaviour::from_config(self.config);
        #[cfg(feature = "messaging")]
        let (messaging_dispatch, messaging_rx) = mpsc::channel(8);
        #[cfg(feature = "tethering")]
        let (tethering_dispatch, tethering_rx) = mpsc::channel(8);
        #[cfg(feature = "relay-client")]
        let (_relay_client_dispatch, relay_client_rx) = mpsc::channel(8);
        #[cfg(feature = "relay-server")]
        let (_relay_server_dispatch, relay_server_rx) = mpsc::channel(8);
        tokio::spawn(async move {
            let mut swarm =
                libp2p::Swarm::with_tokio_executor(transport, behaviour, ident.get_peer_id());
            loop {
                select! {
                Some(op) = op_rx.recv() =>{
                    match op {
                        InEvent::Swarm(op)=>swarm_op_exec(&mut swarm, op),
                        #[cfg(feature = "tethering")]
                        InEvent::Tethering(op)=>swarm.behaviour_mut().tether_op_exec(op),
                        #[cfg(feature = "messaging")]
                        InEvent::Messaging(op) => swarm.behaviour_mut().message_op_exec(op),
                    }
                }
                swarm_event = swarm.select_next_some() =>{
                    match swarm_event{
                        SwarmEvent::NewListenAddr{address, ..}=>println!("Listening on {:?}",address),
                        SwarmEvent::Behaviour(event)=>{
                            match event{
                                #[cfg(feature="messaging")]
                                BehaviourEvent::Messaging(ev)=>{messaging::ev_dispatch(ev,&messaging_dispatch).await;}
                                #[cfg(feature="tethering")]
                                BehaviourEvent::Tethering(ev)=>{tethering::ev_dispatch(ev, &tethering_dispatch).await;},
                                #[cfg(feature="relay-server")]
                                BehaviourEvent::RelayServer(ev) =>{relay_server::ev_dispatch(ev)},
                                #[cfg(feature="relay-client")]
                                BehaviourEvent::RelayClient(ev) =>{relay_client::ev_dispatch(ev)}
                                BehaviourEvent::KeepAlive(_)=>{} }
                            },
                        SwarmEvent::ConnectionEstablished { .. } => println!("connection established"),
                        SwarmEvent::ConnectionClosed { .. } => println!("connection closed"),
                        SwarmEvent::IncomingConnection { send_back_addr,.. } => println!("incoming connection from {} on node {}",send_back_addr,ident.get_peer_id()),
                        SwarmEvent::IncomingConnectionError { send_back_addr,.. } => println!("incoming connection error for {} on node {}", send_back_addr,ident.get_peer_id()),
                        SwarmEvent::OutgoingConnectionError { .. } => println!("outgoing connection error"),
                        SwarmEvent::BannedPeer { .. } => println!("banned peer"),
                        SwarmEvent::ExpiredListenAddr { .. } => println!("expired listen address"),
                        SwarmEvent::ListenerClosed { ..} => println!("listener closed"),
                        SwarmEvent::ListenerError { ..} => println!("listener error"),
                        SwarmEvent::Dialing(_) => println!("dailing"),
                            }
                        }
                    }
            }
        });
        let out_bundle = OutEventBundle {
            #[cfg(feature = "messaging")]
            messaging_rx,
            #[cfg(feature = "tethering")]
            tethering_rx,
            relay_client_rx,
            relay_server_rx,
        };
        (Manager { sender: op_tx }, out_bundle)
    }
}

fn swarm_op_exec(swarm: &mut Swarm<Behaviour>, op: in_event::swarm::InEvent) {
    use in_event::swarm::InEvent;
    match op {
        InEvent::Dial { addr, callback_tx } => {
            let send_res = match swarm.dial(addr) {
                Ok(_) => callback_tx.send(OpResult::DialOk),
                Err(e) => callback_tx.send(OpResult::DialErr(e)),
            };
            match send_res {
                Ok(_) => {}
                Err(op_res) => println!("Failed to send callback {:?}", op_res),
            }
        }
        InEvent::Listen { addr, callback_tx } => {
            let send_res = match swarm.listen_on(addr) {
                Ok(_) => callback_tx.send(OpResult::ListenOk),
                Err(e) => callback_tx.send(OpResult::ListenErr(e)),
            };
            match send_res {
                Ok(_) => {}
                Err(op_res) => println!("Failed to send callback {:?}", op_res),
            }
        }
        InEvent::AddExternalAddress {
            addr,
            score,
            callback_tx,
        } => {
            let send_res = match swarm.add_external_address(addr, score) {
                AddAddressResult::Inserted{..}=>callback_tx.send(OpResult::AddExternalAddress("Inserted".into())),
                AddAddressResult::Updated{..} => callback_tx.send(OpResult::AddExternalAddress("Updated".into())),
            };
            match send_res {
                Ok(_) => {}
                Err(op_res) => println!("Failed to send callback {:?}", op_res),
            }
        }
    }
}
