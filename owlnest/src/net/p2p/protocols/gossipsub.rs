use std::sync::Arc;

use crate::net::p2p::swarm::EventSender;
pub use config::Config;
pub use libp2p::gossipsub::Behaviour;
pub use libp2p::gossipsub::Event as OutEvent;
use libp2p::gossipsub::{IdentTopic, TopicHash};
pub use libp2p::gossipsub::{MessageAuthenticity, MessageId};
pub use libp2p::gossipsub::{PublishError, SubscriptionError};
use libp2p::PeerId;
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use owlnest_macro::with_timeout;
use tokio::sync::{mpsc, oneshot};

type HashToTopicMap = Arc<dashmap::DashMap<TopicHash, String>>;

#[derive(Debug, Clone)]
pub struct ReadableTopic {
    inner: TopicHash,
    topic_string: Option<String>,
}
impl ReadableTopic {
    pub(crate) fn from_mapped(map: &HashToTopicMap, hash: TopicHash) -> Self {
        Self {
            topic_string: map.get(&hash).map(|r| r.value().clone()),
            inner: hash,
        }
    }
    pub fn from_topic_string(topic_string: impl Into<String>) -> Self {
        let topic_string: String = topic_string.into();
        Self {
            inner: IdentTopic::new(topic_string.clone()).hash(),
            topic_string: Some(topic_string),
        }
    }
    pub fn topic_hash(&self) -> &str {
        self.inner.as_str()
    }
    pub fn topic_string(&self) -> Option<&String> {
        self.topic_string.as_ref()
    }
}
impl From<String> for ReadableTopic {
    fn from(value: String) -> Self {
        Self::from_topic_string(value)
    }
}
impl From<&str> for ReadableTopic {
    fn from(value: &str) -> Self {
        Self::from_topic_string(value)
    }
}
impl std::fmt::Display for ReadableTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let None = self.topic_string {
            return write!(f, "Topic:{}", self.inner.as_str());
        }
        write!(
            f,
            "Topic:{}({})",
            self.inner.as_str(),
            self.topic_string.clone().expect("Already handled")
        )
    }
}

#[derive(Debug)]
pub enum InEvent {
    SubscribeTopic(IdentTopic, oneshot::Sender<Result<bool, SubscriptionError>>),
    UnsubscribeTopic(IdentTopic, oneshot::Sender<Result<bool, PublishError>>),
    PublishMessage(
        TopicHash,
        Box<[u8]>,
        oneshot::Sender<Result<MessageId, PublishError>>,
    ),
    BanPeer(PeerId),
    UnbanPeer(PeerId),
    MeshPeersOfTopic(TopicHash, oneshot::Sender<Box<[PeerId]>>),
    AllPeersWithTopic(oneshot::Sender<Box<[(PeerId, Box<[TopicHash]>)]>>),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Handle {
    swarm_event_source: EventSender,
    sender: mpsc::Sender<InEvent>,
    hash_to_topic: HashToTopicMap,
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
            hash_to_topic: Default::default(),
        };
        (handle, rx)
    }
    pub async fn subscribe_topic(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, SubscriptionError> {
        let topic = IdentTopic::new(topic_string);
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::SubscribeTopic(topic, tx);
        self.sender.send(ev).await.expect("Send to succeed");
        rx.await.unwrap()
    }
    pub async fn unsubscribe_topic(
        &self,
        topic_string: impl Into<String>,
    ) -> Result<bool, PublishError> {
        let topic = IdentTopic::new(topic_string);
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::UnsubscribeTopic(topic, tx);
        self.sender.send(ev).await.expect("Send to succeed");
        rx.await.unwrap()
    }
    pub async fn mesh_peers_of_topic(&self, topic: impl Into<String>) -> Box<[PeerId]> {
        let (tx, rx) = oneshot::channel();
        let topic = IdentTopic::new(topic.into());
        let ev = InEvent::MeshPeersOfTopic(topic.hash(), tx);
        self.sender
            .send(ev)
            .await
            .expect("Swarm receiver to stay alive the entire time");
        with_timeout!(rx, 10)
            .expect("future to complete within 10s")
            .expect("callback to succeed")
    }
    pub async fn all_peers_with_topic(&self) -> Box<[(PeerId, Box<[ReadableTopic]>)]> {
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
            .map(|(peer, topic_hash)| {
                (
                    peer,
                    topic_hash
                        .to_vec()
                        .into_iter()
                        .map(|hash| ReadableTopic::from_mapped(&self.hash_to_topic, hash))
                        .collect(),
                )
            })
            .collect()
    }
    pub fn topic_hash_to_string(&self, hash: &TopicHash) -> Option<String> {
        self.hash_to_topic.get(hash).map(|r| r.value().clone())
    }
    generate_handler_method!(
        PublishMessage:publish_message(topic_hash:TopicHash, message:Box<[u8]>)->Result<MessageId,PublishError>;
    );
    generate_handler_method!(
        BanPeer:ban_peer(peer:PeerId);
        UnbanPeer:unban_peer(peer:PeerId);
    );
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
    use libp2p::{
        gossipsub::{IdentTopic, TopicHash},
        PeerId,
    };
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
        /// Lookup the string repersentaion of the topic hash.  
        /// This command won't perfrom network operation and will
        /// lookup from local store.
        TopicHashLookup { topic_hash: String },
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
                                IdentTopic::new(topic.clone()).hash()
                            )
                        } else {
                            println!(
                                r#"Already subscribed to the topic "{}", topic hash: {}"#,
                                topic,
                                IdentTopic::new(topic.clone()).hash()
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
                                r#"Successfully unsubscribed the topic "{}", topic hash: {}"#,
                                topic,
                                IdentTopic::new(topic.clone()).hash()
                            )
                        } else {
                            println!(
                                r#"Not subscribing to the topic "{}", topic hash: {}"#,
                                topic,
                                IdentTopic::new(topic.clone()).hash()
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
                        IdentTopic::new(topic.clone()).hash(),
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
            }
            Gossipsub::TopicHashLookup { topic_hash } => {
                let hash = TopicHash::from_raw(topic_hash);
                match handle.topic_hash_to_string(&hash) {
                    Some(v) => println!("Topic:{}({})", hash.as_str(), v),
                    None => println!(
                        "Cannot find topic string associated with hash {}",
                        hash.as_str()
                    ),
                }
            }
        }
    }
}
