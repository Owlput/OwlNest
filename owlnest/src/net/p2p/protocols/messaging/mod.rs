use crate::net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{trace, warn};

mod behaviour;
pub(crate) mod cli;
mod config;
mod error;
mod handler;
mod message;
mod op;

pub use behaviour::Behaviour;
pub(crate) use cli::handle_messaging;
pub use config::Config;
pub use error::Error;
pub use message::Message;
pub use protocol::PROTOCOL_NAME;

#[derive(Debug)]
pub enum InEvent {
    SendMessage(PeerId, Message, u64),
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingMessage { from: PeerId, msg: Message },
    SendResult(Result<Duration, SendError>, u64),
    Error(Error),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
    Unsupported(PeerId),
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/messaging/0.0.1";
    pub use crate::net::p2p::protocols::universal::protocol::{recv, send};
}

use tokio::sync::mpsc;

use self::error::SendError;
use crate::with_timeout;
use std::sync::atomic::{AtomicU64, Ordering};
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
    pub async fn send_message(
        &self,
        peer_id: PeerId,
        message: Message,
    ) -> Result<Duration, SendError> {
        let op_id = self.next_id();
        let ev = InEvent::SendMessage(peer_id, message, op_id);
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Messaging, OutEvent::SendResult(result, id)=>{
            if *id != op_id {
                continue;
            }
            return result.clone();
        });
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(fut, 10) {
            Ok(v) => v,
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(SendError::Timeout)
            }
        }
    }
    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::net::p2p::{setup_default, setup_logging, swarm::Manager};
    use libp2p::Multiaddr;
    use std::thread;

    #[test]
    fn test_sigle_send_recv() {
        setup_logging();
        let (peer1, _) = setup_default();
        let (peer2, _) = setup_default();
        peer1
            .swarm()
            .listen_blocking(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
            .unwrap();
        let mut peer1_message_watcher = spawn_watcher(&peer1);
        let mut peer2_message_watcher = spawn_watcher(&peer2);
        thread::sleep(Duration::from_millis(100));
        peer2
            .swarm()
            .dial_blocking(&peer1.swarm().list_listeners_blocking()[0])
            .unwrap();
        let peer1_id = peer1.identity().get_peer_id();
        let peer2_id = peer2.identity().get_peer_id();
        thread::sleep(Duration::from_millis(100));
        assert!(
            peer1.swarm().is_connected_blocking(peer2_id)
                && peer2.swarm().is_connected_blocking(peer1_id)
        );
        single_send_recv(&peer1, &peer2, &mut peer2_message_watcher, 1);
        single_send_recv(&peer2, &peer1, &mut peer1_message_watcher, 2);
        thread::sleep(Duration::from_millis(100000));
    }

    fn eq_message(lhs: &Message, rhs: &Message) -> bool {
        lhs.time == rhs.time && lhs.from == rhs.from && lhs.to == rhs.to && lhs.msg == rhs.msg
    }
    fn spawn_watcher(manager: &Manager) -> mpsc::Receiver<(PeerId, Message)> {
        manager.executor().block_on(async {
            let mut listener = manager.event_subscriber().subscribe();
            let (tx, rx) = mpsc::channel(8);
            tokio::spawn(async move {
                while let Ok(ev) = listener.recv().await {
                    if let SwarmEvent::Behaviour(BehaviourEvent::Messaging(
                        OutEvent::IncomingMessage { from, msg },
                    )) = ev.as_ref()
                    {
                        tx.send((*from, msg.clone())).await.unwrap();
                    }
                }
            });
            rx
        })
    }
    fn single_send_recv(
        from: &Manager,
        to: &Manager,
        watcher: &mut mpsc::Receiver<(PeerId, Message)>,
        order: u32,
    ) {
        let from_peer_id = from.identity().get_peer_id();
        let to_peer_id = to.identity().get_peer_id();
        from.executor()
            .block_on(from.messaging().send_message(
                to_peer_id,
                Message::new_ordered(from_peer_id, to_peer_id, order),
            ))
            .unwrap();
        let message_received = watcher.blocking_recv().unwrap();
        assert!(
            message_received.0 == from_peer_id
                && eq_message(
                    &message_received.1,
                    &Message::new_ordered(from_peer_id, to_peer_id, order)
                )
        );
    }
}
