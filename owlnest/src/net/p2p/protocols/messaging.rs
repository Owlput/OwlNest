use super::*;
use crate::net::p2p::swarm::{BehaviourEvent, SwarmEvent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use store::MemMessageStore;

pub use owlnest_messaging::*;

type MessageStore = Box<dyn store::MessageStore + 'static + Send + Sync>;

/// A handle that can communicate with the behaviour within the swarm.
#[derive(Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    #[allow(unused)]
    swarm_event_source: EventSender,
    message_store: Arc<MessageStore>,
    #[allow(unused)]
    counter: Arc<AtomicU64>,
}
impl Handle {
    pub(crate) fn new(
        _config: &Config,
        buffer_size: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let message_store = Arc::new(Box::new(MemMessageStore::default())
            as Box<dyn store::MessageStore + 'static + Send + Sync>);
        let mut listener = swarm_event_source.subscribe();
        let store = message_store.clone();
        tokio::spawn(async move {
            while let Ok(ev) = listener.recv().await {
                if let SwarmEvent::Behaviour(BehaviourEvent::Messaging(ev)) = ev.as_ref() {
                    match ev {
                        OutEvent::IncomingMessage { from, msg } => {
                            store::MessageStore::push_message(
                                store.as_ref().as_ref(),
                                from,
                                msg.clone(),
                            );
                        }
                        _ => {}
                    }
                }
            }
        });
        (
            Self {
                sender: tx,
                swarm_event_source: swarm_event_source.clone(),
                message_store,
                counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    /// Send a message to the target peer.  
    /// Will return the time taken between sending and acknowledgement.
    /// If the peer isn't connected, an error will be returned.
    pub async fn send_message(
        &self,
        peer_id: PeerId,
        message: Message,
    ) -> Result<Duration, error::SendError> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::SendMessage {
            peer: peer_id,
            message,
            callback: tx,
        };
        send_swarm!(self.sender, ev);
        handle_callback!(rx)
    }
    generate_handler_method!(
        /// List all peers that is connected and supports this protocol.
        ListConnected:list_connected()->Box<[PeerId]>;
    );
    /// Get a reference to the internal message store.
    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }
    #[allow(unused)]
    fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}

pub mod cli {
    use clap::Subcommand;
    use libp2p::PeerId;

    use super::{Handle, Message, OutEvent};
    use crate::net::p2p::identity::IdentityUnion;
    use crate::net::p2p::swarm;
    use crate::net::p2p::swarm::behaviour::BehaviourEvent;
    use crate::net::p2p::swarm::manager::Manager;

    /// Subcommand for interacting with `owlnest-messaging` protocol.  
    /// You can use this protocol to send real-time text messages(IM)
    /// to another peer that supports this protocol and you have a connection to.
    #[derive(Debug, Subcommand)]
    pub enum Messaging {
        /// Send the text message to the given peer.
        Send {
            /// The peer to send the message to.
            #[arg(required = true)]
            peer_id: PeerId,
            /// Your text message to send.
            #[arg(required = true)]
            message: String,
        },
    }

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

    pub async fn handle_messaging(handle: &Handle, ident: &IdentityUnion, command: Messaging) {
        use Messaging::*;
        match command {
            Send { peer_id, message } => {
                let msg = Message::new(ident.get_peer_id(), peer_id, message);
                let result = handle.send_message(peer_id, msg).await;
                match result {
                    Ok(_) => println!("Message has been successfully sent"),
                    Err(e) => println!("Error occurred when sending message: {}", e),
                }
            }
        }
    }
}

/// Adapter trait for message store(on-disk or volatile)
pub mod store {
    use dashmap::DashMap;
    use libp2p::PeerId;
    use owlnest_messaging::Message;

    /// The trait a message store need to implement.
    /// Currently the trait is modeled after volatile store.
    pub trait MessageStore {
        /// Insert an empty record of a newly connected peer.
        fn insert_empty_record(&self, peer_id: &PeerId);
        /// Get all message history of a peer.
        fn get_messages(&self, peer_id: &PeerId) -> Option<Box<[Message]>>;
        /// Append a message to the history of the given peer.
        fn push_message(&self, remote: &PeerId, message: Message);
        /// Get all peers that has a record in the store
        fn list_all_peers(&self) -> Box<[PeerId]>;
        /// Clear the message history of the given peer permanently,
        /// but peer records will be retained.
        /// Will clear all history if not supplied with a peer ID.
        fn clear_message(&self, peer_id: Option<&PeerId>);
        /// Empty the message store, including peer records
        fn empty_store(&self);
    }

    /// In-memory volatile message store.
    /// All records will be lost permanently once the store is dropped
    /// e.g. the peer is shutdown or sudden power loss.
    #[derive(Debug, Clone, Default)]
    pub struct MemMessageStore {
        store: DashMap<PeerId, Vec<Message>>,
    }
    impl MessageStore for MemMessageStore {
        fn insert_empty_record(&self, peer_id: &PeerId) {
            if self.store.get(peer_id).is_none() {
                self.store.insert(*peer_id, vec![]);
            }
        }
        fn get_messages(&self, peer_id: &PeerId) -> Option<Box<[Message]>> {
            self.store
                .get(peer_id)
                .map(|v| v.value().clone().into_boxed_slice())
        }
        fn push_message(&self, remote: &PeerId, message: Message) {
            if self.store.get(remote).is_none() {
                self.store.insert(*remote, vec![message]);
                return;
            }
            if let Some(mut entry) = self.store.get_mut(remote) {
                entry.value_mut().push(message)
            };
        }
        fn list_all_peers(&self) -> Box<[PeerId]> {
            self.store.iter().map(|entry| *entry.key()).collect()
        }
        fn clear_message(&self, peer_id: Option<&PeerId>) {
            if peer_id.is_none() {
                self.store
                    .iter_mut()
                    .map(|mut entry| {
                        entry.value_mut().clear();
                        entry.value_mut().shrink_to_fit()
                    })
                    .count();
                return;
            }
            if let Some(mut entry) = self.store.get_mut(peer_id.unwrap()) {
                entry.value_mut().clear();
                entry.value_mut().shrink_to_fit()
            };
        }
        fn empty_store(&self) {
            self.store.clear();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::net::p2p::{swarm::Manager, test_suit::setup_default};
    use libp2p::Multiaddr;
    use owlnest_macro::listen_event;
    use serial_test::serial;
    use std::{io::stdout, thread::sleep};

    #[test]
    #[serial]
    fn test_sigle_send_recv() -> anyhow::Result<()> {
        let (peer1, _) = setup_default();
        let (peer2, _) = setup_default();
        peer1
            .swarm()
            .listen_blocking(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>()?)?;
        let mut peer1_message_watcher = spawn_watcher(&peer1);
        let mut peer2_message_watcher = spawn_watcher(&peer2);
        sleep(Duration::from_millis(100));
        peer2
            .swarm()
            .dial_blocking(&peer1.swarm().list_listeners_blocking()[0])?;
        let peer1_id = peer1.identity().get_peer_id();
        let peer2_id = peer2.identity().get_peer_id();
        sleep(Duration::from_millis(1000));
        assert!(
            peer2.swarm().is_connected_blocking(&peer1_id)
                && peer1.swarm().is_connected_blocking(&peer2_id)
        );
        single_send_recv(&peer1, &peer2, &mut peer2_message_watcher);
        single_send_recv(&peer2, &peer1, &mut peer1_message_watcher);
        sleep(Duration::from_millis(500));
        Ok(())
    }

    fn eq_message(lhs: &Message, rhs: &Message) -> bool {
        lhs.from == rhs.from && lhs.to == rhs.to && lhs.msg == rhs.msg
    }
    fn spawn_watcher(manager: &Manager) -> mpsc::Receiver<(PeerId, Message)> {
        manager.executor().block_on(async {
            let mut listener = manager.event_subscriber().subscribe();
            let (tx, rx) = mpsc::channel(8);

            tokio::spawn(
                listen_event!(listener for Messaging, OutEvent::IncomingMessage { from, msg }=>{
                    tx.send((*from, msg.clone())).await.unwrap();
                }),
            );
            rx
        })
    }
    fn single_send_recv(
        from: &Manager,
        to: &Manager,
        watcher: &mut mpsc::Receiver<(PeerId, Message)>,
    ) {
        let from_peer_id = from.identity().get_peer_id();
        let to_peer_id = to.identity().get_peer_id();
        from.executor()
            .block_on(from.messaging().send_message(
                to_peer_id,
                Message::new(from_peer_id, to_peer_id, "Test MESSAGE 测试信息。"),
            ))
            .unwrap();
        let message_received = watcher.blocking_recv().unwrap();
        assert!(
            message_received.0 == from_peer_id
                && eq_message(
                    &message_received.1,
                    &Message::new(from_peer_id, to_peer_id, "Test MESSAGE 测试信息。")
                )
        );
    }

    // Attach when necessary
    #[allow(unused)]
    fn setup_logging() {
        use owlnest_core::expect::GLOBAL_DEFAULT_SINGLETON;
        use std::sync::Mutex;
        use tracing::Level;
        use tracing_log::LogTracer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Layer;
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("owlnest_messaging", Level::TRACE)
            .with_target("owlnest_blob", Level::INFO)
            .with_target("owlnest::net::p2p::swarm", Level::INFO)
            .with_target("owlnest", Level::TRACE)
            .with_target("multistream_select", Level::WARN)
            .with_target("libp2p_core::transport::choice", Level::WARN)
            .with_target("", Level::DEBUG);
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_ansi(false)
            .with_writer(Mutex::new(stdout()))
            .with_filter(filter);
        let reg = tracing_subscriber::registry().with(layer);
        tracing::subscriber::set_global_default(reg).expect(GLOBAL_DEFAULT_SINGLETON);
        LogTracer::init().unwrap()
    }
}
