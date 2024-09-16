use std::sync::Arc;

use crate::net::p2p::swarm::EventSender;
pub use config::Config;
pub use libp2p::gossipsub::Behaviour;
pub use libp2p::gossipsub::Event as OutEvent;
pub use libp2p::gossipsub::{Message, MessageAuthenticity, MessageId};
pub use libp2p::gossipsub::{PublishError, SubscriptionError};
pub use libp2p::gossipsub::{Sha256Topic, TopicHash};
use libp2p::PeerId;
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use owlnest_macro::with_timeout;
use serde::Deserialize;
use serde::Serialize;
use store::MemMessageStore;
use store::MemTopicStore;
use tokio::sync::{mpsc, oneshot};

type TopicStore = Box<dyn store::TopicStore + Send + Sync>;
type MessageStore = Box<dyn store::MessageStore + Send + Sync>;

#[derive(Debug)]
pub enum InEvent {
    /// Subscribe to the topic.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.
    SubscribeTopic(
        Sha256Topic,
        oneshot::Sender<Result<bool, SubscriptionError>>,
    ),
    /// Unsubscribe from the topic.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    UnsubscribeTopic(Sha256Topic, oneshot::Sender<Result<bool, PublishError>>),
    /// Publishes the message with the given topic to the network.  
    /// Duplicate messages will not be published, resulting in `PublishError::Duplicate`
    PublishMessage(
        TopicHash,
        Box<[u8]>,
        oneshot::Sender<Result<MessageId, PublishError>>,
    ),
    /// Reject all messages from this peer.
    BanPeer(PeerId),
    /// Remove the peer from ban list.
    UnbanPeer(PeerId),
    /// List all peers that interests in the topic.
    MeshPeersOfTopic(TopicHash, oneshot::Sender<Box<[PeerId]>>),
    /// List all peers with their topics of interests.
    AllPeersWithTopic(oneshot::Sender<Box<[(PeerId, Box<[TopicHash]>)]>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadableTopic {
    HashOnly(serde_types::TopicHash),
    StringOnly(String),
    Both {
        hash: serde_types::TopicHash,
        string: String,
    },
}
impl ReadableTopic {
    pub fn get_hash(&self) -> serde_types::TopicHash {
        match self {
            ReadableTopic::HashOnly(hash) => hash.clone(),
            ReadableTopic::StringOnly(string) => Sha256Topic::new(string).hash().into(),
            ReadableTopic::Both { hash, .. } => hash.clone(),
        }
    }
    pub fn get_string(&self) -> Option<&String> {
        match self {
            ReadableTopic::HashOnly(_) => None,
            ReadableTopic::StringOnly(string) => Some(string),
            ReadableTopic::Both { string, .. } => Some(string),
        }
    }
}
impl From<String> for ReadableTopic {
    fn from(value: String) -> Self {
        let topic = Sha256Topic::new(value.clone());
        Self::Both {
            hash: topic.hash().into(),
            string: value,
        }
    }
}
impl From<&str> for ReadableTopic {
    fn from(value: &str) -> Self {
        let topic = Sha256Topic::new(value);
        Self::Both {
            hash: topic.hash().into(),
            string: value.to_owned(),
        }
    }
}
impl From<TopicHash> for ReadableTopic {
    fn from(value: TopicHash) -> Self {
        Self::HashOnly(value.into())
    }
}

#[allow(unused)]
#[derive(Clone)]
pub struct Handle {
    swarm_event_source: EventSender,
    sender: mpsc::Sender<InEvent>,
    pub topic_store: Arc<TopicStore>,
    pub message_store: Arc<MessageStore>,
}
impl Handle {
    pub(crate) fn new(
        buffer: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        let handle = Self {
            swarm_event_source: swarm_event_source.clone(),
            sender: tx,
            topic_store: Arc::new(Box::new(MemTopicStore::default())),
            message_store: Arc::new(Box::new(MemMessageStore::default())),
        };
        (handle, rx)
    }
    /// Subscribe to the topic.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.  
    /// Note: You can only subscribe to a topic
    /// when you have string representation of the topic.
    /// This is intentional.
    pub async fn subscribe_topic(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, SubscriptionError> {
        let topic = Sha256Topic::new(topic_string);
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::SubscribeTopic(topic, tx);
        self.sender.send(ev).await.expect("Send to succeed");
        rx.await.unwrap()
    }
    /// Unsubscribe from the topic.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    pub async fn unsubscribe_topic(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, PublishError> {
        let topic = Sha256Topic::new(topic_string);
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::UnsubscribeTopic(topic, tx);
        self.sender.send(ev).await.expect("Send to succeed");
        rx.await.unwrap()
    }
    /// List all peers that interests in the topic.
    pub async fn mesh_peers_of_topic(&self, topic: impl Into<String>) -> Box<[PeerId]> {
        let (tx, rx) = oneshot::channel();
        let topic = Sha256Topic::new(topic.into());
        let ev = InEvent::MeshPeersOfTopic(topic.hash(), tx);
        self.sender
            .send(ev)
            .await
            .expect("Swarm receiver to stay alive the entire time");
        with_timeout!(rx, 10)
            .expect("future to complete within 10s")
            .expect("callback to succeed")
    }
    /// List all peers with their topics of interests.
    pub async fn all_peers_with_topic(&self) -> Box<[(PeerId, Box<[TopicHash]>)]> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::AllPeersWithTopic(tx);
        self.sender
            .send(ev)
            .await
            .expect("Swarm receiver to stay alive the entire time");
        with_timeout!(rx, 10)
            .expect("future to complete within 10s")
            .expect("callback to succeed")
            .to_vec()
            .into_iter()
            .map(|(peer, topic_hash)| (peer, topic_hash))
            .collect()
    }
    generate_handler_method!(
        /// Publishes the message with the given topic to the network.
        /// Duplicate messages will not be published, resulting in `PublishError::Duplicate`
        PublishMessage:publish_message(topic_hash:TopicHash, message:Box<[u8]>)->Result<MessageId,PublishError>;
    );
    generate_handler_method!(
        /// Reject all messages from this peer.
        BanPeer:ban_peer(peer:PeerId);
        /// Remove the peer from ban list.
        UnbanPeer:unban_peer(peer:PeerId);
    );
}
impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("gossipsub::Handle")
    }
}

pub(crate) fn map_in_event(behav: &mut Behaviour, ev: InEvent) {
    use InEvent::*;
    match ev {
        SubscribeTopic(topic_hash, callback) => {
            let result = behav.subscribe(&topic_hash);
            handle_callback_sender!(result => callback);
        }
        UnsubscribeTopic(topic, callback) => {
            let result = behav.unsubscribe(&topic);
            handle_callback_sender!(result => callback)
        }
        MeshPeersOfTopic(topic_hash, callback) => {
            let list = behav.mesh_peers(&topic_hash).copied().collect();
            handle_callback_sender!(list => callback);
        }
        AllPeersWithTopic(callback) => {
            let list = behav
                .all_peers()
                .map(|(peer, topics)| (*peer, topics.into_iter().cloned().collect()))
                .collect();
            handle_callback_sender!(list => callback);
        }
        PublishMessage(topic, message, callback) => {
            let result = behav.publish(topic, message);
            handle_callback_sender!(result => callback);
        }
        BanPeer(peer) => behav.blacklist_peer(&peer),
        UnbanPeer(peer) => behav.remove_blacklisted_peer(&peer),
    }
}

mod config {
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    /// Configuration parameters that define the performance of the gossipsub network.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        validation_mode: ValidationMode,
        max_transmit_size: usize,
        history_length: usize,
        history_gossip: usize,
        mesh_n: usize,
        mesh_n_low: usize,
        mesh_n_high: usize,
        retain_scores: usize,
        gossip_lazy: usize,
        gossip_factor: f64,
        heartbeat_interval: Duration,
        duplicate_cache_time: Duration,
        allow_self_origin: bool,
        gossip_retransimission: u32,
        max_messages_per_rpc: Option<usize>,
        max_ihave_length: usize,
        max_ihave_messages: usize,
    }
    impl Default for Config {
        fn default() -> Self {
            Self {
                validation_mode: ValidationMode::Strict,
                max_transmit_size: 65536,
                history_length: 5,
                history_gossip: 3,
                mesh_n: 6,
                mesh_n_low: 5,
                mesh_n_high: 12,
                retain_scores: 4,
                gossip_lazy: 6,
                gossip_factor: 0.25,
                heartbeat_interval: Duration::from_secs(1),
                duplicate_cache_time: Duration::from_secs(60),
                allow_self_origin: false,
                gossip_retransimission: 3,
                max_messages_per_rpc: None,
                max_ihave_length: 5000,
                max_ihave_messages: 10,
            }
        }
    }
    impl From<Config> for libp2p::gossipsub::Config {
        fn from(value: Config) -> Self {
            let Config {
                validation_mode,
                max_transmit_size,
                history_length,
                history_gossip,
                mesh_n,
                mesh_n_low,
                mesh_n_high,
                retain_scores,
                gossip_lazy,
                gossip_factor,
                heartbeat_interval,
                duplicate_cache_time,
                allow_self_origin,
                gossip_retransimission,
                max_messages_per_rpc,
                max_ihave_length,
                max_ihave_messages,
            } = value;
            let mut builder = libp2p::gossipsub::ConfigBuilder::default();
            builder
                .history_length(history_length)
                .history_gossip(history_gossip)
                .mesh_n(mesh_n)
                .mesh_n_low(mesh_n_low)
                .mesh_n_high(mesh_n_high)
                .retain_scores(retain_scores)
                .gossip_lazy(gossip_lazy)
                .gossip_factor(gossip_factor)
                .heartbeat_interval(heartbeat_interval)
                .duplicate_cache_time(duplicate_cache_time)
                .allow_self_origin(allow_self_origin)
                .gossip_retransimission(gossip_retransimission)
                .max_messages_per_rpc(max_messages_per_rpc)
                .max_ihave_length(max_ihave_length)
                .max_ihave_messages(max_ihave_messages)
                .validation_mode(validation_mode.into())
                .max_transmit_size(max_transmit_size);
            builder.build().expect("Correct paramaters")
        }
    }

    /// The types of message validation that can be employed by gossipsub.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ValidationMode {
        /// This is the default setting. This requires the message author to be a valid [`PeerId`] and to
        /// be present as well as the sequence number. All messages must have valid signatures.
        ///
        /// NOTE: This setting will reject messages from nodes using
        /// [`crate::behaviour::MessageAuthenticity::Anonymous`] and all messages that do not have
        /// signatures.
        Strict,
        /// This setting permits messages that have no author, sequence number or signature. If any of
        /// these fields exist in the message these are validated.
        Permissive,
        /// This setting requires the author, sequence number and signature fields of a message to be
        /// empty. Any message that contains these fields is considered invalid.
        Anonymous,
        /// This setting does not check the author, sequence number or signature fields of incoming
        /// messages. If these fields contain data, they are simply ignored.
        ///
        /// NOTE: This setting will consider messages with invalid signatures as valid messages.
        None,
    }
    impl From<ValidationMode> for libp2p::gossipsub::ValidationMode {
        fn from(value: ValidationMode) -> Self {
            use libp2p::gossipsub::ValidationMode::*;
            match value {
                ValidationMode::Strict => Strict,
                ValidationMode::Permissive => Permissive,
                ValidationMode::Anonymous => Anonymous,
                ValidationMode::None => None,
            }
        }
    }
}

pub(crate) mod cli {
    use super::Handle;
    use clap::Subcommand;
    use libp2p::{gossipsub::Sha256Topic, PeerId};
    use prettytable::{row, Table};
    use printable::iter::PrintableIter;

    /// Subcommand for interacting with `libp2p-gossipsub`.
    ///
    /// Gossipsub works by randomly talking to peers that support this protocol
    /// about topics of interest and form a virtual network of peers whose interested
    /// topic is the same.  
    /// Through the network, peers can publish and propagate messages to other peers.
    /// However, messages are delivered with best-effort, there is no guarantee
    /// to receive all messages associated with a certain topic.  
    /// The string repersentation of the topic must be known to subscribe to it.
    #[derive(Debug, Subcommand)]
    pub enum Gossipsub {
        /// Subscribe to the given topic.
        Subscribe {
            /// String representation of the topic to subscribe to.
            topic: String,
        },
        /// Unsubscribe to the given topic
        Unsubscribe {
            /// String representation of the topic to unsubscribe.
            topic: String,
        },
        /// Publish a message on the given topic.  
        /// The message can be arbitrary bytes but here we perfer string for convenience.  
        /// The message will be signed using local identity by default.  
        Publish {
            /// The topic to publish message on.
            topic: String,
            /// The message to publish.
            message: String,
        },
        /// Ban a peer, e.g. not interacting with that peer in any way.
        Ban {
            /// The peer to ban.
            peer: PeerId,
        },
        /// Unban a peer.
        Unban {
            /// The peer to unban.
            peer: PeerId,
        },
        /// Get all peers that is connected and supports this protocol,
        /// along with all known topic that the peer is subscribing.
        AllPeersWithTopic,
        /// Get all known peers that is subscribing the topic.
        MeshPeersOfTopic {
            /// The topic to lookup from.
            topic: String,
        },
    }

    pub async fn handle_gossipsub(handle: &Handle, command: Gossipsub) {
        match command {
            Gossipsub::Subscribe { topic } => {
                let result = handle.subscribe_topic(topic.clone()).await;
                match result {
                    Ok(v) => {
                        if v {
                            println!(
                                r#"Successfully subscribed to the topic "{}", topic hash: {}"#,
                                topic,
                                Sha256Topic::new(topic.clone()).hash()
                            )
                        } else {
                            println!(
                                r#"Already subscribed to the topic "{}", topic hash: {}"#,
                                topic,
                                Sha256Topic::new(topic.clone()).hash()
                            )
                        }
                    }
                    Err(e) => {
                        println!("Unable to subscribe to the topic: {:?}", e)
                    }
                }
            }
            Gossipsub::Unsubscribe { topic } => {
                let result = handle.unsubscribe_topic(topic.clone()).await;
                match result {
                    Ok(v) => {
                        if v {
                            println!(
                                r#"Successfully unsubscribed from the topic "{}", topic hash: {}"#,
                                topic,
                                Sha256Topic::new(topic.clone()).hash()
                            )
                        } else {
                            println!(
                                r#"Topic "{}" not subscribed previously, topic hash: {}"#,
                                topic,
                                Sha256Topic::new(topic.clone()).hash()
                            )
                        }
                    }
                    Err(e) => {
                        println!("Unable to unsubscribe to the topic: {:?}", e)
                    }
                }
            }
            Gossipsub::Publish { topic, message } => {
                let result = handle
                    .publish_message(
                        Sha256Topic::new(topic.clone()).hash(),
                        message.into_bytes().into(),
                    )
                    .await;
                match result {
                    Ok(v) => {
                        println!("Message published with Id {}", v)
                    }
                    Err(e) => {
                        println!("Unable to publish to the topic: {:?}", e)
                    }
                }
            }
            Gossipsub::Ban { peer } => {
                handle.ban_peer(peer).await;
                println!("OK")
            }
            Gossipsub::Unban { peer } => {
                handle.unban_peer(peer).await;
                println!("OK")
            }
            Gossipsub::AllPeersWithTopic => {
                let list = handle.all_peers_with_topic().await;
                let mut table = prettytable::Table::new();
                table.add_row(row!["Peer ID", "Subscribed Topics"]);
                list.to_vec().into_iter().for_each(|(peer, topics)| {
                    table.add_row(row![peer, topics.iter().printable()]);
                });
                table.printstd();
            }
            Gossipsub::MeshPeersOfTopic { topic } => {
                let list = handle.mesh_peers_of_topic(topic.clone()).await;
                let mut table = Table::new();
                table.add_row(row![
                    format!("Peers associated with topic\n{}", topic),
                    format!("{}", list.iter().printable())
                ]);
                table.printstd()
            }
        }
    }
}

pub mod store {
    use dashmap::{DashMap, DashSet};
    use libp2p::{
        gossipsub::{Message, Sha256Topic, TopicHash},
        PeerId,
    };

    use super::ReadableTopic;

    pub trait TopicStore {
        fn insert_string(&self, topic_string: String);
        fn insert_hash(&self, topic_hash: TopicHash) -> bool;
        fn try_map(&self, topic_hash: &TopicHash) -> Option<String>;
        fn participants(&self, topic_hash: &TopicHash) -> Option<Box<[PeerId]>>;
        fn join_topic(&self, peer: &PeerId, topic_hash: &TopicHash) -> bool;
        fn leave_topic(&self, peer: &PeerId, topic_hash: &TopicHash) -> bool;
        fn subscribe_topic(&self, topic: ReadableTopic) -> bool;
        fn unsubscribe_topic(&self, topic: &TopicHash) -> bool;
        fn hash_topics(&self) -> Box<[TopicHash]>;
        fn readable_topics(&self) -> Box<[(TopicHash, String)]>;
        fn subscribed_topics(&self) -> Box<[ReadableTopic]>;
    }
    pub trait MessageStore {
        fn get_messages(&self, topic_hash: &TopicHash) -> Option<Box<[Message]>>;
        fn insert_message(&self, message: Message) -> Result<(), ()>;
        fn clear_message(&self, topic: Option<&TopicHash>);
    }

    #[derive(Debug, Default)]
    pub struct MemTopicStore {
        populated: DashMap<TopicHash, (String, DashSet<PeerId>)>,
        vacant: DashMap<TopicHash, DashSet<PeerId>>,
        subscribed: DashSet<TopicHash>,
    }
    impl TopicStore for MemTopicStore {
        fn insert_string(&self, topic_string: String) {
            let hash = Sha256Topic::new(topic_string.clone()).hash();
            if let Some((_, list)) = self.vacant.remove(&hash) {
                self.populated.insert(hash, (topic_string, list));
                return; // move the entry from vacant to populated if present.
            };
            if self.populated.get_mut(&hash).is_none() {
                // Only insert when not present.
                self.populated
                    .insert(hash, (topic_string, Default::default()));
            }
        }
        /// Returns true when the topic is not present anywhere(false when already present or readable).
        fn insert_hash(&self, topic_hash: TopicHash) -> bool {
            if self.populated.get(&topic_hash).is_some() {
                return false; // The topic is present
            }
            if self.vacant.get(&topic_hash).is_some() {
                return false; // The topic is present
            }
            self.vacant.insert(topic_hash.clone(), DashSet::default());
            true // The topic was not presnet
        }
        fn try_map(&self, topic_hash: &TopicHash) -> Option<String> {
            self.populated
                .get(topic_hash)
                .map(|entry| entry.value().0.clone())
        }
        fn readable_topics(&self) -> Box<[(TopicHash, String)]> {
            self.populated
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().0.clone()))
                .collect()
        }
        fn hash_topics(&self) -> Box<[TopicHash]> {
            self.vacant
                .iter()
                .map(|entry| entry.key().clone())
                .collect()
        }
        fn participants(&self, topic_hash: &TopicHash) -> Option<Box<[PeerId]>> {
            self.populated
                .get(topic_hash)
                .map(|entry| entry.value().1.iter().map(|entry| *entry.key()).collect())
                .or(self
                    .vacant
                    .get(topic_hash)
                    .map(|entry| entry.value().iter().map(|entry| *entry.key()).collect()))
        }
        fn join_topic(&self, peer: &PeerId, topic_hash: &TopicHash) -> bool {
            self.insert_hash(topic_hash.clone());
            if let Some(mut entry) = self.vacant.get_mut(topic_hash) {
                return entry.value_mut().insert(*peer);
            }
            if let Some(mut entry) = self.populated.get_mut(topic_hash) {
                return entry.value_mut().1.insert(*peer);
            }
            unreachable!()
        }
        fn leave_topic(&self, peer: &PeerId, topic_hash: &TopicHash) -> bool {
            if let Some(mut entry) = self.vacant.get_mut(topic_hash) {
                return entry.value_mut().remove(peer).is_some();
            }
            if let Some(mut entry) = self.populated.get_mut(topic_hash) {
                return entry.value_mut().1.remove(peer).is_some();
            }
            false
        }
        /// Returns `true` when we have not subscribed before.  
        /// This will add to existing store if not presnet.
        fn subscribe_topic(&self, topic: ReadableTopic) -> bool {
            let hash: TopicHash = topic.get_hash().into();
            let is_subscribed = self.subscribed.insert(hash.clone());
            if let ReadableTopic::HashOnly(_) = topic {
                if self.vacant.get(&hash).is_none() {
                    // insert if not present
                    self.vacant.insert(hash, Default::default());
                }
                return is_subscribed;
            }
            if self.populated.get(&hash).is_none() {
                // insert if not present
                self.populated.insert(
                    hash,
                    (topic.get_string().unwrap().to_owned(), Default::default()),
                );
            }
            is_subscribed
        }
        fn unsubscribe_topic(&self, topic_hash: &TopicHash) -> bool {
            self.subscribed.remove(&topic_hash).is_some()
        }
        fn subscribed_topics(&self) -> Box<[ReadableTopic]> {
            self.subscribed
                .iter()
                .map(|entry| {
                    let hash = entry.key().clone();
                    if let Some(string) = self
                        .populated
                        .get(&hash)
                        .map(|entry| entry.value().0.clone())
                    {
                        return ReadableTopic::Both {
                            hash: hash.into(),
                            string,
                        };
                    };
                    ReadableTopic::HashOnly(hash.into())
                })
                .collect()
        }
    }
    #[derive(Debug, Clone, Default)]
    pub struct MemMessageStore {
        inner: DashMap<TopicHash, Vec<super::Message>>,
    }
    impl MessageStore for MemMessageStore {
        fn get_messages(&self, topic_hash: &TopicHash) -> Option<Box<[Message]>> {
            self.inner
                .get(topic_hash)
                .map(|entry| entry.value().clone().into_boxed_slice())
        }

        fn insert_message(&self, message: Message) -> Result<(), ()> {
            match self.inner.get_mut(&message.topic) {
                Some(mut entry) => entry.value_mut().push(message),
                None => {
                    self.inner.insert(message.topic.clone(), vec![message]);
                }
            }
            Ok(())
        }
        fn clear_message(&self, topic: Option<&TopicHash>) {
            if topic.is_none() {
                return self.inner.clear();
            }
            self.inner.remove(topic.unwrap());
        }
    }
}

pub mod serde_types {
    use libp2p::gossipsub;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
    pub struct TopicHash {
        /// The topic hash. Stored as a string to align with the protobuf API.
        hash: String,
    }
    impl TopicHash {
        pub fn from_raw(hash: impl Into<String>) -> TopicHash {
            TopicHash { hash: hash.into() }
        }

        pub fn into_string(self) -> String {
            self.hash
        }

        pub fn as_str(&self) -> &str {
            &self.hash
        }
    }
    impl From<gossipsub::TopicHash> for TopicHash {
        fn from(value: gossipsub::TopicHash) -> Self {
            Self {
                hash: value.into_string(),
            }
        }
    }
    impl From<TopicHash> for gossipsub::TopicHash {
        fn from(value: TopicHash) -> Self {
            gossipsub::TopicHash::from_raw(value.hash)
        }
    }
}
