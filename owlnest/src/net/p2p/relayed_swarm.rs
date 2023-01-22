use super::*;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    identity::PublicKey,
    relay::v2::client::transport::ClientTransport,
    swarm::{AddressScore, DialError},
    TransportError,
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = RelayedOutEvent)]
#[cfg(feature = "relay-client")]
pub struct Behaviour {
    #[cfg(feature = "messaging")]
    pub messaging: messaging::Behaviour,
    #[cfg(feature = "tethering")]
    pub tethering: tethering::Behaviour,
    #[cfg(feature = "relay-server")]
    pub relay_server: libp2p::relay::v2::relay::Relay,
    pub relay_client: libp2p::relay::v2::client::Client,
    pub keep_alive: libp2p::swarm::keep_alive::Behaviour,
}
impl Behaviour {
    pub fn from_config(config: SwarmConfig) -> (Self, ClientTransport) {
        let (transport, relay_client_behaviour) =
            libp2p::relay::v2::client::Client::new_transport_and_behaviour(
                config.local_ident.get_peer_id(),
            );
        let behaviour = Self {
            #[cfg(feature = "messaging")]
            messaging: messaging::Behaviour::new(config.messaging.clone()),
            #[cfg(feature = "tethering")]
            tethering: tethering::Behaviour::new(config.tethering.clone()),
            #[cfg(feature = "relay-server")]
            relay_server: libp2p::relay::v2::relay::Relay::new(
                config.local_ident.get_peer_id(),
                config.relay_server,
            ),
            relay_client: relay_client_behaviour,
            keep_alive: libp2p::swarm::keep_alive::Behaviour::default(),
        };
        (behaviour, transport)
    }
    #[cfg(feature = "messaging")]
    pub fn message_op_exec(&mut self, ev: messaging::InEvent) {
        self.messaging.push_event(ev)
    }
    #[cfg(feature = "tethering")]
    pub fn tether_op_exec(&mut self, ev: tethering::InEvent) {
        self.tethering.push_op(ev)
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

pub struct OutEventBundle {
    #[cfg(feature = "messaging")]
    pub message_rx: mpsc::Receiver<messaging::OutEvent>,
    #[cfg(feature = "tethering")]
    pub tether_rx: mpsc::Receiver<tethering::OutEvent>,
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

pub struct Builder {
    config: SwarmConfig,
}
impl Builder {
    pub fn new(config: SwarmConfig) -> Self {
        Self { config }
    }
    pub fn build_relayed(self, buffer_size: usize) -> (Manager, OutEventBundle) {
        use libp2p::core::transport::OrTransport;
        let ident = self.config.local_ident.clone();
        let (behaviour, client_transport) = Behaviour::from_config(self.config);
        let tcp_transport = libp2p::tcp::tokio::Transport::new(libp2p::tcp::Config::default());
        let transport = upgrade_transport(
            OrTransport::new(client_transport, tcp_transport).boxed(),
            ident.get_pubkey(),
        );
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
                            if let Err(e) = send_result {println!("Cannot send out listen error {:?}", e);
                            }
                        }
                        Op::Dial { addr, callback_tx } => {
                            if let Err(e) = swarm.dial(addr) {
                                println!("{}", e);
                                if let Err(e) = callback_tx.send(OpResult::DialErr(e)) {
                                println!("Cannot send out dial error {:?}", e)
                                };
                            } else {callback_tx.send(OpResult::DialOk).unwrap();};
                        }
                        #[cfg(feature = "messaging")]
                        Op::Messaging(ev) => swarm.behaviour_mut().message_op_exec(ev),
                        #[cfg(feature="relay-server")]
                        Op::RelayServer(ev)=>{ if let relay_server::InEvent::AddExternalAddress(addr) = ev{
                            swarm.add_external_address(addr, AddressScore::Infinite);
                        }
                            }
                    }
                }
                Some(swarm_event) = swarm.next() =>{
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
                                #[cfg(feature="relay-client")]
                                BehaviourEvent::RelayClient(ev)=>{
                                    relay_client::ev_dispatch(ev).await;
                                }
                                BehaviourEvent::KeepAlive(_)=>{} }
                            },
                        SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, concurrent_dial_errors } => println!("connection to peer {} established. list of established endpoint: {:?}. totol number of connection to that peer: {}. error(s)?: {:?}",peer_id,endpoint,num_established,concurrent_dial_errors),
                        SwarmEvent::ConnectionClosed { peer_id, endpoint, num_established, cause } => println!("connection to peer {} closed on endpoint(s) {:?}, totol number of connection to that peer: {}, cause(s): {:?}",peer_id,endpoint,num_established,cause),
                        SwarmEvent::IncomingConnection { local_addr, send_back_addr } => println!("incoming connection from {} on node {}",send_back_addr,local_addr),
                        SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error } => println!("incoming connection error for {} on node {}: {:#?}", send_back_addr,local_addr,error),
                        SwarmEvent::OutgoingConnectionError { peer_id, error } => println!("outgoing connection to {:?} error: {:#?}",peer_id,error),
                        SwarmEvent::BannedPeer { peer_id, endpoint } => println!("banned peer {} on endpoint {:?}",peer_id,endpoint),
                        SwarmEvent::ExpiredListenAddr { address,.. } => println!("expired listen address: {}",address),
                        SwarmEvent::ListenerClosed { addresses, reason, .. } => println!("listener closed on address: {:?}, reason: {:?}",addresses,reason),
                        SwarmEvent::ListenerError { listener_id, error } => println!("listener {:?} error: {}",listener_id,error),
                        SwarmEvent::Dialing(peer_id) => println!("dailing {}", peer_id),
                            }
                        }
                    }
            }
        });
        let out_bundle = OutEventBundle {
            #[cfg(feature = "messaging")]
            message_rx,
            #[cfg(feature = "tethering")]
            tether_rx,
        };
        (Manager { sender: op_tx }, out_bundle)
    }
}

use futures::{AsyncRead, AsyncWrite};
use libp2p::{core::upgrade, plaintext::PlainText2Config, Transport};

fn upgrade_transport<StreamSink>(
    transport: Boxed<StreamSink>,
    local_public_key: PublicKey,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    transport
        .upgrade(upgrade::Version::V1)
        .authenticate(PlainText2Config { local_public_key })
        .multiplex(libp2p::yamux::YamuxConfig::default())
        .boxed()
}

async fn poll_swarm<S: futures::Stream>(swarm: S) {}
