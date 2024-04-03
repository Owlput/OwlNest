use crate::net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent};
use libp2p::PeerId;
use owlnest_macro::{listen_event, with_timeout};
pub use owlnest_messaging::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

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
    ) -> Result<Duration, error::SendError> {
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
                Err(error::SendError::Timeout)
            }
        }
    }
    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}

pub(crate) mod cli {
    use super::{Message, OutEvent, PROTOCOL_NAME};
    use crate::net::p2p::identity::IdentityUnion;
    use crate::net::p2p::swarm;
    use crate::net::p2p::swarm::behaviour::BehaviourEvent;
    use crate::net::p2p::swarm::manager::Manager;

    pub fn setup(manager: &Manager) {
        let mut listener = manager.event_subscriber().subscribe();
        manager.executor().spawn(async move {
            while let Ok(ev) = listener.recv().await {
                if let swarm::SwarmEvent::Behaviour(BehaviourEvent::Messaging(
                    OutEvent::IncomingMessage { from, msg },
                )) = ev.as_ref()
                {
                    println!("Incoming message from {}: {}", from, msg.msg)
                }
            }
        });
    }

    pub fn handle_messaging(manager: &Manager, ident: &IdentityUnion, command: Vec<&str>) {
        if command.len() < 2 {
            println!("Failed to execute: missing subcommands.");
            println!("{}", TOP_HELP_MESSAGE);
            return;
        }
        match command[1] {
            "send" => handle_message_send(manager, ident, command),
            "help" => println!("Protocol {}/n{}", PROTOCOL_NAME, TOP_HELP_MESSAGE),
            _ => println!(
                "Failed to execute: unrecognized subcommand.\n{}",
                TOP_HELP_MESSAGE
            ),
        }
    }

    fn handle_message_send(manager: &Manager, ident: &IdentityUnion, command: Vec<&str>) {
        if command.len() < 4 {
            println!(
                "Error: Missing required argument(s), syntax: `messaging send <peer id> <message>`"
            );
            return;
        }
        let target_peer: libp2p::PeerId = match std::str::FromStr::from_str(command[2]) {
            Ok(addr) => addr,
            Err(e) => {
                println!("Error: Failed parsing peer ID `{}`: {}", command[2], e);
                return;
            }
        };
        let msg = Message::new(
            ident.get_peer_id(),
            target_peer,
            command.split_at(3).1.join(" "),
        );
        let result = manager
            .executor()
            .block_on(manager.messaging().send_message(target_peer, msg));
        match result {
            Ok(_) => println!("Message has been successfully sent"),
            Err(e) => println!("Error occurred when sending message: {}", e),
        }
    }

    const TOP_HELP_MESSAGE: &str = r#"
Sending messages to the given peer.

Available subcommands:
    send <peer ID> <message>
                Send the message to the given peer.

    help
                Show this help message.
"#;
}
