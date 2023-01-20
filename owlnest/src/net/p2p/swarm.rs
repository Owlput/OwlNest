use std::fmt::Debug;
use std::marker::PhantomData;
use ::futures::StreamExt;
use libp2p::swarm::{DialError, SwarmEvent, NetworkBehaviour};
pub use libp2p::Multiaddr;
use libp2p::TransportError;
use tokio::select;
use tokio::sync::*;

use super::identity::IdentityUnion;
use super::protocols::*;

#[derive(Default)]
pub struct SwarmConfig{
    messaging:messaging::Config,
    tethering:tethering::Config,
}
impl Behaviour{
    #[inline]
    pub fn from_config(config:SwarmConfig)->Self{
        Self{
            messaging:messaging::Behaviour::new(config.messaging),
            tethering:tethering::Behaviour::new(config.tethering)
        }
    }
}

#[derive(Debug)]
pub enum Libp2pSwarmOp {
    Dial {
        addr: Multiaddr,
        callback_tx: oneshot::Sender<Libp2pSwarmOpResult>,
    },
    Listen{
        addr:Multiaddr,
        callback_tx:oneshot::Sender<Libp2pSwarmOpResult>
    },
}

#[derive(Debug)]
pub enum Libp2pSwarmOpResult {
    DialOk,
    DialErr(DialError),
    ListenOk,
    ListenErr(TransportError<std::io::Error>),
}

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug,Clone)]
pub struct Libp2pSwarmManager {
    sender: mpsc::Sender<Libp2pSwarmOp>,

}
impl Libp2pSwarmManager
{
    pub async fn execute(&self, op: Libp2pSwarmOp) {
        self.sender.send(op).await.unwrap()
    }
}

#[derive(Default)]
pub struct Libp2pSwarmBuilder {
    behaviour: PhantomData<Behaviour>,
    config:SwarmConfig
}

impl Libp2pSwarmBuilder
{
    pub fn build(
        self,
        transport: libp2p::core::transport::Boxed<(
            libp2p::PeerId,
            libp2p::core::muxing::StreamMuxerBox,
        )>,
        identity: IdentityUnion,
        buffer_size: usize,
    ) -> (Libp2pSwarmManager,mpsc::Receiver<<Behaviour as NetworkBehaviour>::OutEvent>) {
        let (op_tx, mut op_rx) = mpsc::channel(buffer_size);
        let (event_tx, event_rx) = mpsc::channel(8);
        tokio::spawn(async move {
            let mut swarm =
                libp2p::Swarm::with_tokio_executor(transport, Behaviour::from_config(self.config), identity.get_peer_id());
            loop {
                select! {
                Some(manager_op) = op_rx.recv() =>{
                    match manager_op{
                        Libp2pSwarmOp::Listen{addr,callback_tx}=>{
                            let send_result = match swarm.listen_on(addr){
                                Ok(_)=>{callback_tx.send(Libp2pSwarmOpResult::ListenOk)},
                                Err(e)=>{callback_tx.send(Libp2pSwarmOpResult::ListenErr(e))}
                            };
                            match send_result{
                                Ok(_)=>{},
                                Err(e)=>println!("Cannot send out listen error {:?}",e)
                            }
                        }
                        Libp2pSwarmOp::Dial{addr,callback_tx}=>{
                            if let Err(e) = swarm.dial(addr){
                                println!("{}",e);
                                match callback_tx.send(Libp2pSwarmOpResult::DialErr(e)){
                                    Ok(())=>{},
                                    Err(e)=>println!("Cannot send out dial error {:?}",e)
                                };
                            };
                        }
                    }
                }
                swarm_event = swarm.select_next_some() =>{
                    match swarm_event{
                        SwarmEvent::NewListenAddr{address, ..}=>println!("Listening on {:?}",address),
                        SwarmEvent::Behaviour(event)=>{let _ = event_tx.send(event).await;},
                        SwarmEvent::ConnectionEstablished { .. } => println!("connection established"),
                        SwarmEvent::ConnectionClosed { .. } => println!("connection closed"),
                        SwarmEvent::IncomingConnection { send_back_addr,.. } => println!("incoming connection from {} on node {}",send_back_addr,identity.get_peer_id()),
                        SwarmEvent::IncomingConnectionError { send_back_addr,.. } => println!("incoming connection error for {} on node {}", send_back_addr,identity.get_peer_id()),
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
        (Libp2pSwarmManager {sender: op_tx,},event_rx)
    }
}
