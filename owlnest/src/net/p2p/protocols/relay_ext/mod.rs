use std::sync::{atomic::AtomicU64, Arc};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent},
    with_timeout,
};

pub mod behaviour;
pub mod cli;
mod handler;
pub mod in_event;

pub use behaviour::Behaviour;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutEvent {
    QueryAnswered {
        from: PeerId,
        list: Vec<PeerId>,
    },
    RemoteAdvertisementResult {
        from: PeerId,
        result: Result<(), ()>,
    },
    ProviderState(bool, u64),
    Error(Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    ConnectionClosed,
    VerifierMismatch,
    NotProviding,
    Timeout,
    UnrecognizedMessage(String), // Serialzied not available on the original type
    IO(String),                  // Serialize not available on the original type
    Channel,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            ConnectionClosed => f.write_str("Connection Closed"),
            VerifierMismatch => f.write_str("Message verifier mismatch"),
            Timeout => f.write_str("Message timed out"),
            NotProviding => f.write_str(&format!("Relay is not providing")),
            UnrecognizedMessage(msg) => f.write_str(msg),
            IO(msg) => f.write_str(msg),
            Channel => f.write_str("Callback channel closed unexpectedly"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/relay_ext/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}

#[derive(Debug)]
pub enum InEvent {
    SetProviderState(bool, u64),
    QueryProviderState(u64),
    QueryAdvertisedPeer(PeerId),
    SetAdvertisingSelf {
        remote: PeerId,
        state: bool,
        id: u64,
    },
}

macro_rules! event_op {
    ($listener:ident,$pattern:pat,{$($ops:tt)+}) => {
        async move{
        loop{
            let ev = crate::handle_listener_result!($listener);
            if let SwarmEvent::Behaviour(BehaviourEvent::RelayExt($pattern)) = ev.as_ref() {
                $($ops)+
            } else {
                continue;
            }
        }
    }
    };
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    counter: Arc<AtomicU64>,
}
impl Handle {
    pub fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
                counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    pub async fn query_advertised_peer(&self, relay: PeerId) -> Result<Vec<PeerId>, Error> {
        let mut listener = self.event_tx.subscribe();
        let fut = event_op!(listener, OutEvent::QueryAnswered { from, list }, {
            if *from == relay {
                return list.clone();
            }
        });

        let ev = InEvent::QueryAdvertisedPeer(relay);
        self.sender.send(ev).await.unwrap();
        match with_timeout!(fut, 10) {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::Timeout),
        }
    }
    pub async fn set_provider_state(&self, state: bool) -> bool {
        let op_id = self.get_id();
        let mut listener = self.event_tx.subscribe();
        let fut = event_op!(listener, OutEvent::ProviderState(state, id), {
            if *id != op_id {
                continue;
            }
            return *state;
        });
        let ev = InEvent::SetProviderState(state, op_id);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    pub async fn provider_state(&self) -> bool {
        let op_id = self.get_id();
        let mut listener = self.event_tx.subscribe();
        let fut = event_op!(listener, OutEvent::ProviderState(state, id), {
            if *id != op_id {
                continue;
            }
            return *state;
        });
        let ev = InEvent::QueryProviderState(op_id);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    pub async fn set_remote_advertisement(&self, remote: PeerId, state: bool) {
        let op_id = self.get_id();
        let ev = InEvent::SetAdvertisingSelf {
            remote,
            state,
            id: op_id,
        };
        self.sender.send(ev).await.expect("Send to succeed");
    }
    fn get_id(&self) -> u64 {
        use std::sync::atomic::Ordering;
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod test {
    use std::{thread, time::Duration};

    use libp2p::Multiaddr;

    use crate::net::p2p::setup_default;

    #[test]
    fn test() {
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        peer1_m
            .swarm()
            .listen(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
            .unwrap();
        let peer1_id = peer1_m.identity().get_peer_id();
        let peer2_id = peer2_m.identity().get_peer_id();
        peer2_m
            .swarm()
            .dial(&peer1_m.swarm().list_listeners_blocking()[0])
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.relay_ext().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        peer2_m
            .executor()
            .block_on(peer2_m.relay_ext().set_remote_advertisement(peer1_id, true));
        assert!(peer2_m.swarm().is_connected_blocking(peer1_id));
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m
            .executor()
            .block_on(peer2_m.relay_ext().query_advertised_peer(peer1_id))
            .unwrap()
            .contains(&peer2_id));
        assert!(!peer1_m.executor().block_on(peer1_m.relay_ext().set_provider_state(false)));
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m.executor().block_on(peer2_m.relay_ext().query_advertised_peer(peer1_id)).unwrap().len() == 0);
        peer2_m.executor().block_on(peer2_m.relay_ext().set_remote_advertisement(peer1_id, false));
        assert!(peer1_m.executor().block_on(peer1_m.relay_ext().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m.executor().block_on(peer2_m.relay_ext().query_advertised_peer(peer1_id)).unwrap().len() == 0);
    }
}
