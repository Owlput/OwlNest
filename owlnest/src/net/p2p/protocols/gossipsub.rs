use std::str::FromStr;
use std::sync::Arc;

use crate::net::p2p::swarm::EventSender;
use clap::ValueEnum;
pub use config::Config;
pub use libp2p::gossipsub::Behaviour;
pub use libp2p::gossipsub::Event as OutEvent;
pub use libp2p::gossipsub::Topic;
pub use libp2p::gossipsub::{Hasher, IdentityHash, Sha256Hash, TopicHash};
pub use libp2p::gossipsub::{Message, MessageAuthenticity, MessageId};
pub use libp2p::gossipsub::{PublishError, SubscriptionError};
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
    SubscribeTopic {
        topic_hash: TopicHash,
        callback: oneshot::Sender<Result<bool, SubscriptionError>>,
    },
    /// Unsubscribe from the topic.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    UnsubscribeTopic {
        topic_hash: TopicHash,
        callback: oneshot::Sender<Result<bool, PublishError>>,
    },
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

#[derive(Debug, Copy, Clone, ValueEnum, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashType {
    Sha256,
    Identity,
}
impl HashType {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadableTopic {
    HashOnly {
        hash: serde_types::TopicHash,
        hash_type: Option<HashType>,
    },
    StringOnly {
        topic_string: String,
    },
    Both {
        topic_hash: serde_types::TopicHash,
        hash_type: Option<HashType>,
        topic_string: String,
    },
}
impl ReadableTopic {
    pub fn from_string(topic_string: String, hash_type: HashType) -> Self {
        Self::Both {
            topic_hash: hash_type.hash(topic_string.clone()).into(),
            hash_type: Some(hash_type),
            topic_string,
        }
    }
    pub fn get_hash<FallbackHasher: Hasher>(&self) -> serde_types::TopicHash {
        match self {
            ReadableTopic::HashOnly { hash, .. } => hash.clone(),
            ReadableTopic::StringOnly { topic_string } => {
                FallbackHasher::hash(topic_string.clone()).into()
            }
            ReadableTopic::Both {
                topic_hash: hash, ..
            } => hash.clone(),
        }
    }
    pub fn get_string(&self) -> Option<&String> {
        match self {
            ReadableTopic::HashOnly { .. } => None,
            ReadableTopic::StringOnly { topic_string } => Some(topic_string),
            ReadableTopic::Both {
                topic_string: string,
                ..
            } => Some(string),
        }
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
    /// Use a topic string to subscribe to a topic.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.  
    pub async fn subscribe_topic<H: Hasher>(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, SubscriptionError> {
        self.subscribe_topic_hash(H::hash(topic_string.into()))
            .await
    }
    /// Use a topic hash to subscribe to a topic. Any type of hash can be used here.  
    /// Returns `Ok(true)` if the subscription worked. Returns `Ok(false)` if we were already
    /// subscribed.  
    pub async fn subscribe_topic_hash(
        &self,
        topic_hash: TopicHash,
    ) -> Result<bool, SubscriptionError> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::SubscribeTopic {
            topic_hash,
            callback: tx,
        };
        self.sender.send(ev).await.expect("Send to succeed");
        rx.await.unwrap()
    }
    /// Use a topic string to unsubscribe from a topic.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    pub async fn unsubscribe_topic<H: Hasher>(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, PublishError> {
        self.unsubscribe_topic_hash(H::hash(topic_string.into()))
            .await
    }
    /// Use a topic hash to unsubscribe from a topic. Any type of hash can be used here.  
    /// Returns [`Ok(true)`] if we were subscribed to this topic.
    pub async fn unsubscribe_topic_hash(
        &self,
        topic_hash: TopicHash,
    ) -> Result<bool, PublishError> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::UnsubscribeTopic {
            topic_hash,
            callback: tx,
        };
        self.sender.send(ev).await.expect("Send to succeed");
        rx.await.unwrap()
    }
    /// List all peers that interests in the topic. Any type of hash can be used here.  
    pub async fn mesh_peers_of_topic(&self, topic_hash: TopicHash) -> Box<[PeerId]> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::MeshPeersOfTopic(topic_hash, tx);
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
        SubscribeTopic {
            topic_hash,
            callback,
        } => {
            let result = behav.subscribe(&topic_hash);
            handle_callback_sender!(result => callback);
        }
        UnsubscribeTopic {
            topic_hash,
            callback,
        } => {
            let result = behav.unsubscribe(&topic_hash);
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
    use super::{Handle, HashType};
    use clap::Subcommand;
    use libp2p::{
        gossipsub::{Hasher, IdentityHash, Sha256Hash, Sha256Topic, TopicHash},
        PeerId,
    };
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
            #[arg(long, require_equals = true, value_enum)]
            hash_type: HashType,
        },
    }
    impl SubUnsubCommand {
        pub(crate) fn inner(&self) -> &String {
            match self {
                Self::ByHash { topic_hash, .. } => topic_hash,
                Self::ByString { topic_string, .. } => topic_string,
            }
        }
        pub(crate) fn get_hash_type(&self) -> &HashType {
            match self {
                Self::ByHash { hash_type, .. } => hash_type,
                Self::ByString { hash_type, .. } => hash_type,
            }
        }
        pub(crate) fn get_hash(&self) -> TopicHash {
            if let Self::ByHash { topic_hash, .. } = &self {
                return TopicHash::from_raw(topic_hash);
            }
            match self.get_hash_type() {
                HashType::Identity => IdentityHash::hash(self.inner().clone()),
                HashType::Sha256 => Sha256Hash::hash(self.inner().clone()),
            }
        }
    }

    pub async fn handle_gossipsub(handle: &Handle, command: Gossipsub) {
        match command {
            Gossipsub::Subscribe(command) => {
                let result = match &command {
                    SubUnsubCommand::ByHash {
                        topic_hash,
                        hash_type: HashType::Identity,
                    } => {
                        handle
                            .subscribe_topic_hash(TopicHash::from_raw(topic_hash))
                            .await
                    }
                    SubUnsubCommand::ByHash {
                        topic_hash,
                        hash_type: HashType::Sha256,
                    } => {
                        handle
                            .subscribe_topic_hash(TopicHash::from_raw(topic_hash))
                            .await
                    }
                    SubUnsubCommand::ByString {
                        topic_string,
                        hash_type: HashType::Identity,
                    } => handle.subscribe_topic::<IdentityHash>(topic_string).await,
                    SubUnsubCommand::ByString {
                        topic_string,
                        hash_type: HashType::Sha256,
                    } => handle.subscribe_topic::<Sha256Hash>(topic_string).await,
                };
                match result {
                    Ok(v) => {
                        if v {
                            println!(
                                r#"Successfully subscribed to the topic "{}""#,
                                command.inner(),
                            )
                        } else {
                            println!(r#"Already subscribed to the topic "{}""#, command.inner(),)
                        }
                    }
                    Err(e) => {
                        println!("Unable to subscribe to the topic: {:?}", e)
                    }
                }
            }
            Gossipsub::Unsubscribe(command) => {
                let result = match &command {
                    SubUnsubCommand::ByHash {
                        topic_hash,
                        hash_type: HashType::Identity,
                    } => {
                        handle
                            .unsubscribe_topic_hash(TopicHash::from_raw(topic_hash))
                            .await
                    }
                    SubUnsubCommand::ByHash {
                        topic_hash,
                        hash_type: HashType::Sha256,
                    } => {
                        handle
                            .unsubscribe_topic_hash(TopicHash::from_raw(topic_hash))
                            .await
                    }
                    SubUnsubCommand::ByString {
                        topic_string,
                        hash_type: HashType::Identity,
                    } => handle.unsubscribe_topic::<IdentityHash>(topic_string).await,
                    SubUnsubCommand::ByString {
                        topic_string,
                        hash_type: HashType::Sha256,
                    } => handle.unsubscribe_topic::<Sha256Hash>(topic_string).await,
                };
                match result {
                    Ok(v) => {
                        if v {
                            println!(
                                r#"Successfully unsubscribed from the topic "{}""#,
                                command.inner(),
                            )
                        } else {
                            println!(r#"Topic "{}" not subscribed previously."#, command.inner())
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
                list.iter().for_each(|(peer, topics)| {
                    table.add_row(row![peer, topics.iter().printable()]);
                });
                table.printstd();
            }
            Gossipsub::MeshPeersOfTopic(topic) => {
                let list = handle.mesh_peers_of_topic(topic.get_hash()).await;
                let mut table = Table::new();
                table.add_row(row![
                    format!("Peers associated with topic\n{}", topic.get_hash()),
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
        gossipsub::{IdentityHash, Message, Sha256Topic, TopicHash},
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
            let hash: TopicHash = topic.get_hash::<IdentityHash>().into();
            let is_subscribed = self.subscribed.insert(hash.clone());
            if let ReadableTopic::HashOnly { .. } = topic {
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
            self.subscribed.remove(topic_hash).is_some()
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
                            topic_hash: hash.into(),
                            topic_string: string,
                            hash_type: None,
                        };
                    };
                    ReadableTopic::HashOnly {
                        hash: hash.into(),
                        hash_type: None,
                    }
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
