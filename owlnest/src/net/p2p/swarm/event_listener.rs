use std::collections::HashMap;
use super::SwarmEvent;
use futures::channel::oneshot;
use tokio::{select, sync::mpsc};

type Error = crate::event_bus::Error;

#[derive(Debug)]
pub enum Op {
    Add(
        Kind,
        mpsc::Sender<SwarmEvent>,
        oneshot::Sender<Result<u64, Error>>,
    ),
    Remove(Kind, u64, oneshot::Sender<Result<(), Error>>),
}

#[repr(u32)]
#[derive(Debug)]
pub enum Kind {
    OnConnectionEstablished = 0,
    OnIncomingConnection = 1,
    OnIncomingConnectionError = 2,
    OnOutgoingConnectionError = 3,
    OnBannedPeer = 4,
    OnNewListenAddr = 5,
    OnExpiredListenAddr = 6,
    OnListenerClosed = 7,
    OnListenerError = 8,
    OnDialing = 9,
}

type Listener = mpsc::Sender<SwarmEvent>;

pub fn setup_event_listener(mut op_rx: mpsc::Receiver<Op>, mut ev_rx: mpsc::Receiver<SwarmEvent>) {
    tokio::spawn(async move {
        let listener_store: &mut [HashMap<u64, Listener>] = &mut [HashMap::new(), HashMap::new()];
        select! {
                Some(op) = op_rx.recv() => {
                    match op{
                        Op::Add(kind,ev_tx, callback) => {
                            match kind{
                                Kind::OnConnectionEstablished => {
                                    let id = rand::random::<u64>();
                                    if listener_store[0].try_insert(id,ev_tx).is_ok(){
                                        callback.send(Ok(id)).unwrap();
                                        return;
                                    };
                                    callback.send(Err(Error::QueueFull)).unwrap();
                                }
                                Kind::OnDialing => {
                                    let id = rand::random::<u64>();
                                    if listener_store[1].try_insert(id,ev_tx).is_ok(){
                                        callback.send(Ok(id)).unwrap();
                                        return;
                                    };
                                    callback.send(Err(Error::QueueFull)).unwrap();
                                },
                                _=>{}
                            }
                        },
                        Op::Remove(kind,id,callback) => {
                            match kind{
                                Kind::OnConnectionEstablished => {
                                    if listener_store[0].remove(&id).is_some(){
                                        callback.send(Ok(())).unwrap();
                                    } else{
                                        callback.send(Err(Error::NotFound)).unwrap()
                                    }
                                },
                                Kind::OnDialing => todo!(),
                                _=>{}
                            }
                        },
                    }
                        }
                        Some(ev) = ev_rx.recv() =>{
                            match ev{
                                libp2p::swarm::SwarmEvent::Behaviour(_) => todo!(),
                                libp2p::swarm::SwarmEvent::ConnectionEstablished {..} => {

                                },
                                libp2p::swarm::SwarmEvent::ConnectionClosed { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::IncomingConnection { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::IncomingConnectionError { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::OutgoingConnectionError { .. } => todo!(),
                                #[allow(deprecated)]
                                libp2p::swarm::SwarmEvent::BannedPeer { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::NewListenAddr { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::ExpiredListenAddr { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::ListenerClosed { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::ListenerError { .. } => todo!(),
                                libp2p::swarm::SwarmEvent::Dialing(_) => todo!(),
        }
                        }
                    }
    });
}
