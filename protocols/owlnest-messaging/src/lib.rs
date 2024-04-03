use owlnest_prelude::lib_prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::trace;

mod behaviour;
mod config;
pub mod error;
mod handler;
pub mod message;
mod op;

pub use behaviour::Behaviour;
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
    SendResult(Result<Duration, error::SendError>, u64),
    Error(Error),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
    Unsupported(PeerId),
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/messaging/0.0.1";
    pub use owlnest_prelude::utils::protocol::universal::*;
}

#[cfg(feature = "disabled")]
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
