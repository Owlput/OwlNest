use super::*;
use libp2p::swarm::{DialError, AddressScore};
use libp2p::TransportError;
use std::fmt::Debug;

#[cfg(feature = "messaging")]
use super::protocols::messaging;
#[cfg(feature = "tethering")]
use super::protocols::tethering;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = OutEvent)]
pub struct Behaviour {
    #[cfg(feature = "messaging")]
    pub messaging: messaging::Behaviour,
    #[cfg(feature = "tethering")]
    pub tethering: tethering::Behaviour,
    #[cfg(feature = "relay-server")]
    pub relay_server: libp2p::relay::v2::relay::Relay,
    pub keep_alive: libp2p::swarm::keep_alive::Behaviour,
}
impl Behaviour {
    #[cfg(feature = "messaging")]
    pub fn message_op_exec(&mut self, op: messaging::InEvent) {
        self.messaging.push_event(op)
    }
    #[cfg(feature = "tethering")]
    pub fn tether_op_exec(&mut self, op: tethering::InEvent) {
        self.tethering.push_op(op)
    }
}

#[derive(Debug)]
pub enum OutEvent {
    #[cfg(feature = "messaging")]
    Messaging(messaging::OutEvent),
    #[cfg(feature = "tethering")]
    Tethering(tethering::OutEvent),
    #[cfg(feature = "relay-client")]
    RelayClient(libp2p::relay::v2::client::Event),
    #[cfg(feature = "relay-server")]
    RelayServer(libp2p::relay::v2::relay::Event),
}

impl Behaviour {
    pub fn from_config(config: SwarmConfig) -> Self {
        Self {
            #[cfg(feature = "messaging")]
            messaging: messaging::Behaviour::new(config.messaging),
            #[cfg(feature = "tethering")]
            tethering: tethering::Behaviour::new(config.tethering),
            #[cfg(feature = "relay-server")]
            relay_server: libp2p::relay::v2::relay::Relay::new(
                config.local_ident.get_peer_id(),
                config.relay_server,
            ),
            keep_alive: libp2p::swarm::keep_alive::Behaviour::default(),
        }
    }
}

#[derive(Debug)]
pub enum Op {
    Dial {
        addr: Multiaddr,
        callback_tx: oneshot::Sender<OpResult>,
    },
    Listen {
        addr: Multiaddr,
        callback_tx: oneshot::Sender<OpResult>,
    },
    #[cfg(feature = "messaging")]
    Messaging(messaging::InEvent),
    #[cfg(feature = "relay-server")]
    RelayServer(relay_server::InEvent),
}

#[derive(Debug)]
pub enum OpResult {
    DialOk,
    DialErr(DialError),
    ListenOk,
    ListenErr(TransportError<std::io::Error>),
    EmitOk,
    EmitErr(mpsc::error::SendError<Op>),
    CallbackRecvErr(oneshot::error::RecvError),
}

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    sender: mpsc::Sender<Op>,
}
impl Manager {
    pub async fn execute(&self, op: Op) -> OpResult {
        match self.sender.send(op).await {
            Ok(_) => OpResult::EmitOk,
            Err(e) => OpResult::EmitErr(e),
        }
    }
    pub async fn dial(&self, addr: Multiaddr) -> OpResult {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = Op::Dial { addr, callback_tx };
        self.sender.send(op).await.unwrap();
        match callback_rx.await {
            Ok(res) => res,
            Err(e) => {
                println!("{}", e);
                OpResult::CallbackRecvErr(e)
            }
        }
    }
    pub async fn listen(&self, addr: Multiaddr) -> OpResult {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = Op::Listen { addr, callback_tx };
        self.sender.send(op).await.unwrap();
        callback_rx.await.unwrap()
    }
}

pub struct OutBundle {
    #[cfg(feature = "messaging")]
    pub message_rx: mpsc::Receiver<messaging::OutEvent>,
    #[cfg(feature = "tethering")]
    pub tether_rx: mpsc::Receiver<tethering::OutEvent>,
}
#[derive(Default)]
pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build(
        self,
        transport: libp2p::core::transport::Boxed<(
            libp2p::PeerId,
            libp2p::core::muxing::StreamMuxerBox,
        )>,
        buffer_size: usize,
    ) -> (Manager, OutBundle) {
        let ident = self.config.local_ident.clone();
        let behaviour = Behaviour::from_config(self.config);
        let (op_tx, mut op_rx) = mpsc::channel(buffer_size);
        #[cfg(feature = "messaging")]
        let (message_dispatch, message_rx) = mpsc::channel(8);
        #[cfg(feature = "tethering")]
        let (tether_dispatch, tether_rx) = mpsc::channel(8);
        tokio::spawn(async move {
            let mut swarm =
                libp2p::Swarm::with_tokio_executor(transport, behaviour, ident.get_peer_id());
            loop {
                select! {
                Some(op) = op_rx.recv() =>{
                    match op {
                        Op::Listen { addr, callback_tx } => {
                            let send_result = match swarm.listen_on(addr) {
                                Ok(_) => callback_tx.send(OpResult::ListenOk),
                                Err(e) => callback_tx.send(OpResult::ListenErr(e)),
                            };
                            match send_result {
                                Ok(_) => {}
                                Err(e) => println!("Cannot send out listen error {:?}", e),
                            }
                        }
                        Op::Dial { addr, callback_tx } => {
                            if let Err(e) = swarm.dial(addr) {
                                println!("{}", e);
                                match callback_tx.send(OpResult::DialErr(e)) {
                                    Ok(()) => {}
                                    Err(e) => println!("Cannot send out dial error {:?}", e),
                                };
                            };
                        }
                        #[cfg(feature = "messaging")]
                        Op::Messaging(op) => swarm.behaviour_mut().message_op_exec(op),
                        #[cfg(feature="relay-server")]
                        Op::RelayServer(ev)=>{ if let relay_server::InEvent::AddExternalAddress(addr) = ev{
                            swarm.add_external_address(addr, AddressScore::Infinite);
                        }}
                    }
                }
                swarm_event = swarm.select_next_some() =>{
                    match swarm_event{
                        SwarmEvent::NewListenAddr{address, ..}=>println!("Listening on {:?}",address),
                        SwarmEvent::Behaviour(event)=>{
                            match event{
                                #[cfg(feature="messaging")]
                                BehaviourEvent::Messaging(ev)=>{messaging::ev_dispatch(ev,&message_dispatch).await;}
                                #[cfg(feature="tethering")]
                                BehaviourEvent::Tethering(ev)=>{tethering::ev_dispatch(ev, &tether_dispatch).await;},
                                #[cfg(feature="relay-server")]
                                BehaviourEvent::RelayServer(_) => {},
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
        let out_bundle = OutBundle {
            #[cfg(feature = "messaging")]
            message_rx,
            #[cfg(feature = "tethering")]
            tether_rx,
        };
        (Manager { sender: op_tx }, out_bundle)
    }
}

#[cfg(feature = "tethering")]
impl From<tethering::OutEvent> for OutEvent {
    fn from(value: tethering::OutEvent) -> Self {
        super::swarm::OutEvent::Tethering(value)
    }
}

#[cfg(feature = "messaging")]
impl From<messaging::OutEvent> for OutEvent {
    fn from(value: messaging::OutEvent) -> Self {
        super::swarm::OutEvent::Messaging(value)
    }
}
