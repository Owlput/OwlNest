use std::sync::Arc;

use crate::net::p2p::swarm::EventSender;
pub use libp2p::gossipsub::Behaviour;
pub use libp2p::gossipsub::Config;
pub use libp2p::gossipsub::Event as OutEvent;
pub use libp2p::gossipsub::MessageAuthenticity;
pub use libp2p::gossipsub::MessageId;
use libp2p::gossipsub::{IdentTopic, TopicHash};
pub use libp2p::gossipsub::{PublishError, SubscriptionError};
use libp2p::PeerId;
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use owlnest_macro::with_timeout;
use tokio::sync::{mpsc, oneshot};

type HashToTopicMap = Arc<dashmap::DashMap<TopicHash, String>>;

#[derive(Debug)]
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
    #[derive(Clone)]
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
