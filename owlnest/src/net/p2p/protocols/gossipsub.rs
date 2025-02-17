use super::*;
use clap::ValueEnum;
pub use config::Config;
pub use libp2p::gossipsub::Behaviour;
pub use libp2p::gossipsub::Event as OutEvent;
pub use libp2p::gossipsub::Topic;
pub use libp2p::gossipsub::{Hasher, IdentityHash, Sha256Hash, TopicHash};
pub use libp2p::gossipsub::{Message, MessageAuthenticity, MessageId};
pub use libp2p::gossipsub::{PublishError, SubscriptionError};
use std::str::FromStr;
use std::sync::Arc;

type TopicStore = Box<dyn store::TopicStore + Send + Sync>;
type MessageStore = Box<dyn store::MessageStore + Send + Sync>;

#[derive(Debug)]
pub(crate) enum InEvent {
    /// Subscribe to the topic.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.
    SubscribeTopic {
        topic: TopicHash,
        callback: Callback<Result<bool, SubscriptionError>>,
    },
    /// Unsubscribe from the topic.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    UnsubscribeTopic {
        topic: TopicHash,
        callback: Callback<bool>,
    },
    /// Publishes the message with the given topic to the network.  
    /// Duplicate messages will not be published, resulting in `PublishError::Duplicate`
    PublishMessage {
        topic: TopicHash,
        message: Box<[u8]>,
        callback: Callback<Result<MessageId, PublishError>>,
    },
    /// List all peers that interests in the topic.
    MeshPeersOfTopic {
        topic: TopicHash,
        callback: Callback<Box<[PeerId]>>,
    },
    /// List all peers with their topics of interests.
    AllPeersWithTopic {
        callback: Callback<Box<[(PeerId, Box<[TopicHash]>)]>>,
    },
    /// Reject all messages from this peer.
    BanPeer { peer: PeerId },
    /// Remove the peer from ban list.
    UnbanPeer { peer: PeerId },
}

/// Types of different supported hashers for the topic.
#[derive(Debug, Copy, Clone, ValueEnum, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashType {
    /// SHA256 algorithm.
    Sha256,
    /// Equivalent to using no hasher.
    Identity,
}
impl HashType {
    /// Hash the given topic string.
    pub fn hash(&self, topic_string: String) -> TopicHash {
        match self {
            Self::Sha256 => Sha256Hash::hash(topic_string),
            Self::Identity => IdentityHash::hash(topic_string),
        }
    }
}
impl FromStr for HashType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(Self::Sha256),
            "identity" => Ok(Self::Identity),
            _ => Err(()),
        }
    }
}
impl std::fmt::Display for HashType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

/// A handle that can communicate with the behaviour within the swarm.
#[allow(unused)]
#[derive(Clone)]
pub struct Handle {
    swarm_event_source: EventSender,
    sender: mpsc::Sender<InEvent>,
    topic_store: Arc<TopicStore>,
    message_store: Arc<MessageStore>,
}
impl Handle {
    pub(crate) fn new(
        config: &Config,
        buffer_size: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let (topic_store, message_store) = match config.store {
            config::Store::Volatile => (
                Arc::new(Box::new(mem_store::MemTopicStore::default()) as TopicStore),
                Arc::new(Box::new(mem_store::MemMessageStore::default()) as MessageStore),
            ),
        };
        let handle = Self {
            swarm_event_source: swarm_event_source.clone(),
            sender: tx,
            topic_store,
            message_store,
        };
        (handle, rx)
    }
    /// Use a topic string to subscribe to a topic.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.  
    pub async fn subscribe_topic<H: Hasher>(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, SubscriptionError> {
        let topic_string = topic_string.into();
        let topic_hash = H::hash(topic_string.clone());
        let result = self.subscribe_topic_hash(&topic_hash).await;
        if let Ok(true) = result {
            self.topic_store
                .subscribe_topic(&topic_hash, Some(topic_string));
        }
        result
    }
    /// Use a topic hash to subscribe to a topic. Any type of hash can be used here.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.  
    pub async fn subscribe_topic_hash(
        &self,
        topic_hash: &TopicHash,
    ) -> Result<bool, SubscriptionError> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::SubscribeTopic {
            topic: topic_hash.clone(),
            callback: tx,
        };
        send_swarm!(self.sender, ev);
        let result = handle_callback!(rx);
        if let Ok(true) = result {
            self.topic_store.subscribe_topic(&topic_hash, None);
        }
        result
    }
    /// Use a topic string to unsubscribe from a topic.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    pub async fn unsubscribe_topic<H: Hasher>(&self, topic_string: impl Into<String>) -> bool {
        self.unsubscribe_topic_hash(&H::hash(topic_string.into()))
            .await
    }
    /// Use a topic hash to unsubscribe from a topic. Any type of hash can be used here.  
    /// Returns [`true`] if we were subscribed to this topic.
    pub async fn unsubscribe_topic_hash(&self, topic_hash: &TopicHash) -> bool {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::UnsubscribeTopic {
            topic: topic_hash.clone(),
            callback: tx,
        };
        send_swarm!(self.sender, ev);
        let result = handle_callback!(rx);
        if result {
            self.topic_store.unsubscribe_topic(&topic_hash);
        }
        result
    }
    /// Get a reference to the internal message store.
    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }
    /// Get a reference to the internal topic store.
    pub fn topic_store(&self) -> &TopicStore {
        &self.topic_store
    }
    /// Publishes the message with the given topic to the network.
    /// Duplicate messages will not be published, resulting in `PublishError::Duplicate`
    pub async fn publish_message(
        &self,
        topic_hash: &TopicHash,
        message: Box<[u8]>,
    ) -> Result<MessageId, PublishError> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::PublishMessage {
            topic: topic_hash.clone(),
            message: message.clone(),
            callback: tx,
        };
        send_swarm!(self.sender, ev);
        let result = handle_callback!(rx);
        if result.is_ok() {
            self.message_store
                .insert_message(topic_hash, store::MessageRecord::Local(message));
        }
        result
    }
    generate_handler_method!(
        /// Reject all messages from this peer.
        BanPeer:ban_peer(peer:&PeerId);
        /// Remove the peer from ban list.
        UnbanPeer:unban_peer(peer:&PeerId);
        /// List all peers that interests in the topic. Any type of hash can be used here.
        MeshPeersOfTopic:mesh_peers_of_topic(topic: <&TopicHash>) -> Box<[PeerId]>;
        /// List all peers with their topics of interests.
        AllPeersWithTopic:all_peers_with_topic() -> Box<[(PeerId, Box<[TopicHash]>)]>;
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
        SubscribeTopic {
            topic: topic_hash,
            callback,
        } => {
            let result = behav.subscribe(&topic_hash);
            handle_callback_sender!(result => callback);
        }
        UnsubscribeTopic {
            topic: topic_hash,
            callback,
        } => {
            let result = behav.unsubscribe(&topic_hash);
            handle_callback_sender!(result => callback)
        }
        MeshPeersOfTopic { topic, callback } => {
            let list = behav.mesh_peers(&topic).copied().collect();
            handle_callback_sender!(list => callback);
        }
        AllPeersWithTopic { callback } => {
            let list = behav
                .all_peers()
                .map(|(peer, topics)| (*peer, topics.into_iter().cloned().collect()))
                .collect();
            handle_callback_sender!(list => callback);
        }
        PublishMessage {
            topic,
            message,
            callback,
        } => {
            let result = behav.publish(topic, message);
            handle_callback_sender!(result => callback);
        }
        BanPeer { peer } => behav.blacklist_peer(&peer),
        UnbanPeer { peer } => behav.remove_blacklisted_peer(&peer),
    }
}

mod config {
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    /// Configuration parameters that define the performance of the gossipsub network.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        /// The types of message validation that can be employed by gossipsub.
        /// for the available types. The default is ValidationMode::Strict.
        pub validation_mode: ValidationMode,
        /// The maximum amount of bytes a message can carry without being rejected.
        pub max_transmit_size: usize,
        /// Number of heartbeats to keep in the `memcache` (default to 5).
        pub history_length: usize,
        /// Number of past heartbeats to gossip about (default to 3).
        pub history_gossip: usize,
        /// Target number of peers for the mesh network (D in the spec, default to 6).
        pub mesh_n: usize,
        /// Minimum number of peers in mesh network before adding more (D_lo in the spec, default to 4).
        pub mesh_n_low: usize,
        /// Maximum number of peers in mesh network before removing some (D_high in the spec, default
        /// to 12).
        pub mesh_n_high: usize,
        /// Affects how peers are selected when pruning a mesh due to over subscription.
        ///
        /// At least [`Self::retain_scores`] of the retained peers will be high-scoring, while the remainder are
        /// chosen randomly (D_score in the spec, default to 4).
        pub retain_scores: usize,
        /// Minimum number of peers to emit gossip to during a heartbeat (D_lazy in the spec,
        /// default to 6).
        pub gossip_lazy: usize,
        /// Affects how many peers we will emit gossip to at each heartbeat.
        ///
        /// We will send gossip to `gossip_factor * (total number of non-mesh peers)`, or
        /// `gossip_lazy`, whichever is greater. The default is 0.25.
        pub gossip_factor: f64,
        /// Time between each heartbeat (default to 1000 ms).
        pub heartbeat_interval_ms: u64,
        /// Duplicates are prevented by storing message id's of known messages in an LRU time cache.
        /// This settings sets the time period that messages are stored in the cache. Duplicates can be
        /// received if duplicate messages are sent at a time greater than this setting apart. The
        /// default is 1 minute.
        pub duplicate_cache_time_ms: u64,
        /// By default, gossipsub will reject messages that are sent to us that have the same message
        /// source as we have specified locally. Enabling this, allows these messages and prevents
        /// penalizing the peer that sent us the message. Default is false.
        pub allow_self_origin: bool,
        /// Times we will allow a peer to request the same message id through IWANT
        /// gossip before we start ignoring them. This is designed to prevent peers from spamming us
        /// with requests and wasting our resources. The default is 3.
        pub gossip_retransimission: u32,
        /// The maximum number of messages we will process in a given RPC. If this is unset, there is
        /// no limit. The default is None.
        pub max_messages_per_rpc: Option<usize>,
        /// The maximum number of messages to include in an IHAVE message.
        /// Also controls the maximum number of IHAVE ids we will accept and request with IWANT from a
        /// peer within a heartbeat, to protect from IHAVE floods. You should adjust this value from the
        /// default if your system is pushing more than 5000 messages in GossipSubHistoryGossip
        /// heartbeats; with the defaults this is 1666 messages/s. The default is 5000.
        pub max_ihave_length: usize,
        /// GossipSubMaxIHaveMessages is the maximum number of IHAVE messages to accept from a peer
        /// within a heartbeat.
        pub max_ihave_messages: usize,
        /// External stores used for messages and topics.
        pub store: Store,
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
                heartbeat_interval_ms: 1 * 1000,
                duplicate_cache_time_ms: 60 * 1000,
                allow_self_origin: false,
                gossip_retransimission: 3,
                max_messages_per_rpc: None,
                max_ihave_length: 5000,
                max_ihave_messages: 10,
                store: Store::Volatile,
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
                heartbeat_interval_ms,
                duplicate_cache_time_ms,
                allow_self_origin,
                gossip_retransimission,
                max_messages_per_rpc,
                max_ihave_length,
                max_ihave_messages,
                ..
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
                .heartbeat_interval(Duration::from_millis(heartbeat_interval_ms))
                .duplicate_cache_time(Duration::from_millis(duplicate_cache_time_ms))
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Store {
        Volatile,
    }
}

pub mod cli {
    use super::{Handle, HashType};
    use clap::Subcommand;
    use libp2p::{gossipsub::TopicHash, PeerId};
    use prettytable::{row, Table};
    use printable::iter::PrintableIter;

    /// Subcommand for interacting with `libp2p-gossipsub`.
    ///
    /// Gossipsub works by randomly talking to peers that support this protocol
    /// about topics of interest and historical messages, forming a virtual network of peers
    /// whose interested topic is the same.  
    /// Through the network, peers can publish and propagate messages to other peers.
    /// However, messages are delivered with best-effort, there is no guarantee
    /// to receive all messages associated with a certain topic.  
    /// The string repersentation of the topic must be known to subscribe to it.
    #[derive(Debug, Subcommand)]
    pub enum Gossipsub {
        /// Subscribe to the given topic.
        #[command(subcommand)]
        Subscribe(SubUnsubCommand),
        /// Unsubscribe to the given topic
        #[command(subcommand)]
        Unsubscribe(SubUnsubCommand),
        /// Publish a message on the given topic.  
        /// The message can be arbitrary bytes but here we perfer string for convenience.  
        /// The message will be signed using local identity by default.  
        #[command(subcommand)]
        Publish(PublishCommand),
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
        #[command(subcommand)]
        MeshPeersOfTopic(SubUnsubCommand),
    }

    #[derive(Debug, Clone, Subcommand)]
    pub enum SubUnsubCommand {
        ByString {
            /// String representation of the topic to subscribe/unsubscribe to.
            topic_string: String,
            /// The type of hasher to use.
            #[arg(long, require_equals = true, value_enum)]
            hash_type: HashType,
        },
        ByHash {
            /// Hash representation of the topic to subscribe/unsubscribe to.
            topic_hash: String,
        },
    }
    impl SubUnsubCommand {
        pub(crate) fn destruct(self) -> TopicHash {
            match self {
                Self::ByHash { topic_hash } => TopicHash::from_raw(topic_hash),
                Self::ByString {
                    topic_string,
                    hash_type,
                } => hash_type.hash(topic_string),
            }
        }
    }

    #[derive(Debug, Clone, Subcommand)]
    pub enum PublishCommand {
        ByString {
            /// String representation of the topic to subscribe/unsubscribe to.
            topic_string: String,
            /// The type of hasher to use.
            #[arg(long, require_equals = true, value_enum)]
            hash_type: HashType,
            message: String,
        },
        ByHash {
            /// Hash representation of the topic to subscribe/unsubscribe to.
            topic_hash: String,
            message: String,
        },
    }
    impl PublishCommand {
        pub(crate) fn destruct(self) -> (TopicHash, Box<[u8]>) {
            match self {
                Self::ByHash {
                    topic_hash,
                    message,
                } => (
                    TopicHash::from_raw(topic_hash),
                    message.into_bytes().into_boxed_slice(),
                ),
                Self::ByString {
                    topic_string,
                    hash_type,
                    message,
                } => (
                    hash_type.hash(topic_string),
                    message.into_bytes().into_boxed_slice(),
                ),
            }
        }
    }

    pub async fn handle_gossipsub(handle: &Handle, command: Gossipsub) {
        match command {
            Gossipsub::Subscribe(command) => {
                let topic_hash = command.destruct();
                let result = handle.subscribe_topic_hash(&topic_hash).await;
                match result {
                    Ok(v) => {
                        if v {
                            println!(r#"Successfully subscribed to the topic "{}""#, topic_hash,)
                        } else {
                            println!(r#"Already subscribed to the topic "{}""#, topic_hash,)
                        }
                    }
                    Err(e) => {
                        println!("Unable to subscribe to the topic: {:?}", e)
                    }
                }
            }
            Gossipsub::Unsubscribe(command) => {
                let topic_hash = command.destruct();
                let result = handle.unsubscribe_topic_hash(&topic_hash).await;

                if result {
                    println!(
                        r#"Successfully unsubscribed from the topic "{}""#,
                        topic_hash,
                    )
                } else {
                    println!(r#"Topic "{}" not subscribed previously."#, topic_hash)
                }
            }
            Gossipsub::Publish(command) => {
                let (topic_hash, message) = command.destruct();
                let result = handle.publish_message(&topic_hash, message).await;
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
                handle.ban_peer(&peer).await;
                println!("OK")
            }
            Gossipsub::Unban { peer } => {
                handle.unban_peer(&peer).await;
                println!("OK")
            }
            Gossipsub::AllPeersWithTopic => {
                let list = handle.all_peers_with_topic().await;
                let mut table = prettytable::Table::new();
                table.add_row(row!["Peer ID", "Subscribed Topics"]);
                list.iter().for_each(|(peer, topics)| {
                    table.add_row(row![peer, topics.iter().printable()]);
                });
                table.printstd();
            }
            Gossipsub::MeshPeersOfTopic(command) => {
                let topic_hash = command.destruct();
                let list = handle.mesh_peers_of_topic(&topic_hash).await;
                let mut table = Table::new();
                table.add_row(row![
                    format!("Peers associated with topic\n{}", topic_hash),
                    format!("{}", list.iter().printable())
                ]);
                table.printstd()
            }
        }
    }
}

/// Traits and types related to storing string-hash map and messages.
pub mod store {
    use super::*;

    /// Trait for topic stores.
    pub trait TopicStore {
        /// Insert a topic record with only its string representation.
        /// Returns `true` when the topic is new, or `false` otherwise.
        fn insert_topic(&self, topic_string: String, topic_hash: &TopicHash) -> bool;
        /// Insert a topic record with only its string representation.
        /// Returns `true` when the topic is new, or `false` otherwise.
        fn insert_hash(&self, topic_hash: &TopicHash) -> bool;
        /// Try to get the string representation of the topic.
        fn try_map(&self, topic_hash: &TopicHash) -> Option<String>;
        /// Get all participans of the topic.  
        /// Will return `None` if the topic is not known.
        fn participants(&self, topic_hash: &TopicHash) -> Option<Box<[PeerId]>>;
        /// Called when a new peer is subscribed to the topic.
        fn join_topic(&self, peer: &PeerId, topic_hash: &TopicHash) -> bool;
        /// Called when a peer is unsubscribed from the topic.
        fn leave_topic(&self, peer: &PeerId, topic_hash: &TopicHash) -> bool;
        /// Called when the local peer is subscribing to the topic.
        fn subscribe_topic(&self, topic: &TopicHash, topic_string: Option<String>) -> bool;
        /// Called when the local peer is unsubscribing from the topic.
        fn unsubscribe_topic(&self, topic: &TopicHash) -> bool;
        /// Get all topics that only has known hash
        fn hash_topics(&self) -> Box<[TopicHash]>;
        /// Get all topics whose string repr and hash are both known.
        fn readable_topics(&self) -> Box<[(TopicHash, String)]>;
        /// Get all subscribed topics.
        fn subscribed_topics(&self) -> Box<[TopicHash]>;
    }

    /// The kind of message.
    #[derive(Debug, Clone)]
    pub enum MessageRecord {
        /// Message published by a remote node.
        Remote(super::Message),
        /// Message published by local node.
        Local(Box<[u8]>),
    }

    /// Trait for message stores.
    pub trait MessageStore {
        /// Get all messages related to the topic.
        fn get_messages(&self, topic_hash: &TopicHash) -> Option<Box<[MessageRecord]>>;
        /// Called when a new message is received on the topic.
        fn insert_message(&self, topic_hash: &TopicHash, message: MessageRecord);
        /// Clear the store of the topic.  
        /// Will clear everything if not supplied with a topic hash.
        fn clear_message(&self, topic: Option<&TopicHash>);
    }
}

/// Alternative types that support `serde`.
pub mod serde_types {
    use libp2p::gossipsub;
    use serde::{Deserialize, Serialize};

    /// Equivalent of `libp2p::gossipsub::TopicHash`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
    pub struct TopicHash {
        /// The topic hash. Stored as a string to align with the protobuf API.
        hash: String,
    }
    impl TopicHash {
        /// Build a `TopicHash` from raw string.
        pub fn from_raw(hash: impl Into<String>) -> TopicHash {
            TopicHash { hash: hash.into() }
        }
        /// Get the internal representation of the `TopicHash`.
        pub fn into_string(self) -> String {
            self.hash
        }
        /// Get a refrence to the internal.
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
    impl AsRef<String> for TopicHash {
        fn as_ref(&self) -> &String {
            &self.hash
        }
    }
}

/// Volatile in-memory stores
pub mod mem_store {
    use dashmap::{DashMap, DashSet};
    use store::MessageRecord;

    use super::store::{MessageStore, TopicStore};
    use super::*;
    /// An in-memory message store.
    #[derive(Debug, Default)]
    pub struct MemTopicStore {
        populated: DashMap<TopicHash, (String, DashSet<PeerId>)>,
        vacant: DashMap<TopicHash, DashSet<PeerId>>,
        subscribed: DashSet<TopicHash>,
    }
    impl TopicStore for MemTopicStore {
        fn insert_topic(&self, topic_string: String, topic_hash: &TopicHash) -> bool {
            if let Some((hash, list)) = self.vacant.remove(&topic_hash) {
                self.populated.insert(hash, (topic_string, list));
                return false; // move the entry from vacant to populated if present.
            };
            if self.populated.get_mut(topic_hash).is_none() {
                // Only insert when not present.
                self.populated
                    .insert(topic_hash.clone(), (topic_string, Default::default()));
                return true;
            }
            false
        }
        /// Returns true when the topic is not present anywhere(false when already present or readable).
        fn insert_hash(&self, topic_hash: &TopicHash) -> bool {
            if self.vacant.get(topic_hash).is_some() || self.populated.get(topic_hash).is_some() {
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
            self.insert_hash(topic_hash);
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
        fn subscribe_topic(&self, topic: &TopicHash, topic_string: Option<String>) -> bool {
            if let Some(topic_string) = topic_string {
                self.insert_topic(topic_string, &topic);
            } else {
                self.insert_hash(&topic);
            }
            if !self.subscribed.contains(topic) {
                self.subscribed.insert(topic.clone());
                return true;
            }
            false
        }
        fn unsubscribe_topic(&self, topic_hash: &TopicHash) -> bool {
            self.subscribed.remove(topic_hash).is_some()
        }
        fn subscribed_topics(&self) -> Box<[TopicHash]> {
            self.subscribed.iter().map(|entry| entry.clone()).collect()
        }
    }

    /// An in-memory message store.
    #[derive(Clone, Default)]
    pub struct MemMessageStore {
        inner: DashMap<TopicHash, Vec<super::store::MessageRecord>>,
    }
    impl MessageStore for MemMessageStore {
        fn get_messages(&self, topic_hash: &TopicHash) -> Option<Box<[MessageRecord]>> {
            self.inner
                .get(topic_hash)
                .map(|entry| entry.value().clone().into_boxed_slice())
        }

        fn insert_message(&self, topic: &TopicHash, message: MessageRecord) {
            match self.inner.get_mut(topic) {
                Some(mut entry) => entry.value_mut().push(message),
                None => {
                    self.inner.insert(topic.clone(), vec![message]);
                }
            }
        }
        fn clear_message(&self, topic: Option<&TopicHash>) {
            if topic.is_none() {
                return self.inner.clear();
            }
            self.inner.remove(topic.unwrap());
        }
    }
    #[cfg(test)]
    mod test {
        use libp2p::gossipsub::TopicHash;

        use super::{store::TopicStore, MemTopicStore};

        #[test]
        fn fallthrough_and_promote_when_string_is_available() {
            let store = MemTopicStore::default();
            // test promotion when using insert
            let string: String = "this is a topic hash".into();
            let hash = TopicHash::from_raw(string.clone());
            assert!(store.insert_hash(&hash));
            assert!(store.populated.get(&hash).is_none() && store.vacant.get(&hash).is_some());
            assert!(!store.insert_topic(string, &hash));
            assert!(store.populated.get(&hash).is_some() && store.vacant.get(&hash).is_none());
            // don't demote
            assert!(!store.insert_hash(&hash));
            assert!(store.populated.get(&hash).is_some() && store.vacant.get(&hash).is_none());

            // test promotion when using subscribe
            let string: String = "this is also a topic hash".into();
            let hash = TopicHash::from_raw(string.clone());
            assert!(store.subscribe_topic(&hash, None));
            assert!(store.populated.get(&hash).is_none() && store.vacant.get(&hash).is_some());
            assert!(!store.subscribe_topic(&hash, Some(string)));
            assert!(store.populated.get(&hash).is_some() && store.vacant.get(&hash).is_none());
            // don't demote
            assert!(!store.subscribe_topic(&hash, None));
            assert!(store.populated.get(&hash).is_some() && store.vacant.get(&hash).is_none());
        }
    }
}
