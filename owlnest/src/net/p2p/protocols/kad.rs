use crate::net::p2p::swarm::{behaviour::BehaviourEvent, EventSender, SwarmEvent};
use libp2p::{Multiaddr, PeerId, StreamProtocol};
use owlnest_macro::{generate_handler_method, handle_callback_sender, listen_event, with_timeout};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, trace};

pub use libp2p::kad;
pub type Behaviour = kad::Behaviour<kad::store::MemoryStore>;
pub type OutEvent = kad::Event;
pub use libp2p::kad::PROTOCOL_NAME;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    max_packet_size: usize,
    query_config: config::QueryConfig,
    record_ttl: Option<Duration>,
    record_replication_interval: Option<Duration>,
    record_publication_interval: Option<Duration>,
    record_filtering: config::StoreInserts,
    provider_record_ttl: Option<Duration>,
    provider_publication_interval: Option<Duration>,
    kbucket_inserts: config::BucketInserts,
    caching: config::Caching,
    periodic_bootstrap_interval: Option<Duration>,
    automatic_bootstrap_throttle: Option<Duration>,
}
impl Config {
    pub fn into_config(self, protocol: String) -> kad::Config {
        let Config {
            max_packet_size,
            query_config,
            record_ttl,
            record_replication_interval,
            record_publication_interval,
            record_filtering,
            provider_record_ttl,
            provider_publication_interval,
            kbucket_inserts,
            caching,
            periodic_bootstrap_interval,
            automatic_bootstrap_throttle,
        } = self;
        let mut config = kad::Config::new(StreamProtocol::try_from_owned(protocol).unwrap())
            .disjoint_query_paths(query_config.disjoint_query_paths)
            .set_automatic_bootstrap_throttle(automatic_bootstrap_throttle)
            .set_parallelism(query_config.parallelism)
            .set_periodic_bootstrap_interval(periodic_bootstrap_interval)
            .set_provider_record_ttl(provider_record_ttl)
            .set_query_timeout(query_config.timeout)
            .set_record_filtering(record_filtering.into())
            .set_record_ttl(record_ttl);
        config
            .set_replication_factor(query_config.replication_factor)
            .set_caching(caching.into())
            .set_kbucket_inserts(kbucket_inserts.into())
            .set_max_packet_size(max_packet_size)
            .set_provider_publication_interval(provider_publication_interval)
            .set_replication_interval(record_replication_interval)
            .set_publication_interval(record_publication_interval);
        config
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_packet_size: 16 * 1024,
            query_config: config::QueryConfig::default(),
            record_ttl: Some(Duration::from_secs(48 * 60 * 60)),
            record_replication_interval: Some(Duration::from_secs(60 * 60)),
            record_publication_interval: Some(Duration::from_secs(22 * 60 * 60)),
            record_filtering: config::StoreInserts::Unfiltered,
            provider_publication_interval: Some(Duration::from_secs(12 * 60 * 60)),
            provider_record_ttl: Some(Duration::from_secs(48 * 60 * 60)),
            kbucket_inserts: config::BucketInserts::OnConnected,
            caching: config::Caching::Enabled { max_peers: 1 },
            periodic_bootstrap_interval: Some(Duration::from_secs(5 * 60)),
            automatic_bootstrap_throttle: Some(Duration::from_secs(10)),
        }
    }
}

pub mod config {
    /// The `k` parameter of the Kademlia specification.
    ///
    /// This parameter determines:
    ///
    ///   1) The (fixed) maximum number of nodes in a bucket.
    ///   2) The (default) replication factor, which in turn determines:
    ///       a) The number of closer peers returned in response to a request.
    ///       b) The number of closest peers to a key to search for in an iterative query.
    ///
    /// The choice of (1) is fixed to this constant. The replication factor is configurable
    /// but should generally be no greater than `K_VALUE`. All nodes in a Kademlia
    /// DHT should agree on the choices made for (1) and (2).
    ///
    /// The current value is `20`.
    pub const K_VALUE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(20) };

    /// The `α` parameter of the Kademlia specification.
    ///
    /// This parameter determines the default parallelism for iterative queries,
    /// i.e. the allowed number of in-flight requests that an iterative query is
    /// waiting for at a particular time while it continues to make progress towards
    /// locating the closest peers to a key.
    ///
    /// The current value is `3`.
    pub const ALPHA_VALUE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(3) };

    use std::{num::NonZeroUsize, time::Duration};

    use serde::{Deserialize, Serialize};
    /// The configuration for queries in a `QueryPool`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct QueryConfig {
        /// Timeout of a single query.
        ///
        /// See [`crate::behaviour::Config::set_query_timeout`] for details.
        pub(crate) timeout: Duration,
        /// The replication factor to use.
        ///
        /// See [`crate::behaviour::Config::set_replication_factor`] for details.
        pub(crate) replication_factor: NonZeroUsize,
        /// Allowed level of parallelism for iterative queries.
        ///
        /// See [`crate::behaviour::Config::set_parallelism`] for details.
        pub(crate) parallelism: NonZeroUsize,
        /// Whether to use disjoint paths on iterative lookups.
        ///
        /// See [`crate::behaviour::Config::disjoint_query_paths`] for details.
        pub(crate) disjoint_query_paths: bool,
    }
    impl Default for QueryConfig {
        fn default() -> Self {
            QueryConfig {
                timeout: Duration::from_secs(60),
                replication_factor: NonZeroUsize::new(K_VALUE.get()).expect("K_VALUE > 0"),
                parallelism: ALPHA_VALUE,
                disjoint_query_paths: false,
            }
        }
    }
    /// The configurable filtering strategies for the acceptance of
    /// incoming records.
    ///
    /// This can be used for e.g. signature verification or validating
    /// the accompanying [`Key`].
    ///
    /// [`Key`]: crate::record::Key
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum StoreInserts {
        /// Whenever a (provider) record is received,
        /// the record is forwarded immediately to the [`RecordStore`].
        Unfiltered,
        /// Whenever a (provider) record is received, an event is emitted.
        /// Provider records generate a [`InboundRequest::AddProvider`] under [`Event::InboundRequest`],
        /// normal records generate a [`InboundRequest::PutRecord`] under [`Event::InboundRequest`].
        ///
        /// When deemed valid, a (provider) record needs to be explicitly stored in
        /// the [`RecordStore`] via [`RecordStore::put`] or [`RecordStore::add_provider`],
        /// whichever is applicable. A mutable reference to the [`RecordStore`] can
        /// be retrieved via [`Behaviour::store_mut`].
        FilterBoth,
    }
    impl From<StoreInserts> for super::kad::StoreInserts {
        fn from(value: StoreInserts) -> Self {
            match value {
                StoreInserts::Unfiltered => Self::Unfiltered,
                StoreInserts::FilterBoth => Self::FilterBoth,
            }
        }
    }

    /// The configurable strategies for the insertion of peers
    /// and their addresses into the k-buckets of the Kademlia
    /// routing table.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum BucketInserts {
        /// Whenever a connection to a peer is established as a
        /// result of a dialing attempt and that peer is not yet
        /// in the routing table, it is inserted as long as there
        /// is a free slot in the corresponding k-bucket. If the
        /// k-bucket is full but still has a free pending slot,
        /// it may be inserted into the routing table at a later time if an unresponsive
        /// disconnected peer is evicted from the bucket.
        OnConnected,
        /// New peers and addresses are only added to the routing table via
        /// explicit calls to [`Behaviour::add_address`].
        ///
        /// > **Note**: Even though peers can only get into the
        /// > routing table as a result of [`Behaviour::add_address`],
        /// > routing table entries are still updated as peers
        /// > connect and disconnect (i.e. the order of the entries
        /// > as well as the network addresses).
        Manual,
    }
    impl From<BucketInserts> for super::kad::BucketInserts {
        fn from(value: BucketInserts) -> Self {
            match value {
                BucketInserts::OnConnected => Self::OnConnected,
                BucketInserts::Manual => Self::Manual,
            }
        }
    }

    /// The configuration for Kademlia "write-back" caching after successful
    /// lookups via [`Behaviour::get_record`].
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum Caching {
        /// Caching is disabled and the peers closest to records being looked up
        /// that do not return a record are not tracked, i.e.
        /// [`GetRecordOk::FinishedWithNoAdditionalRecord`] is always empty.
        Disabled,
        /// Up to `max_peers` peers not returning a record that are closest to the key
        /// being looked up are tracked and returned in [`GetRecordOk::FinishedWithNoAdditionalRecord`].
        /// The write-back operation must be performed explicitly, if
        /// desired and after choosing a record from the results, via [`Behaviour::put_record_to`].
        Enabled { max_peers: u16 },
    }
    impl From<Caching> for super::kad::Caching {
        fn from(value: Caching) -> Self {
            match value {
                Caching::Disabled => Self::Disabled,
                Caching::Enabled { max_peers } => Self::Enabled { max_peers },
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum InEvent {
    PeerLookup(PeerId, oneshot::Sender<kad::QueryId>),
    BootStrap(oneshot::Sender<Result<kad::QueryId, kad::NoKnownPeers>>),
    InsertNode(PeerId, Multiaddr, oneshot::Sender<kad::RoutingUpdate>),
    SetMode(Option<kad::Mode>),
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    swarm_event_source: EventSender,
    tree_map: Arc<RwLock<BTreeMap<PeerId, kad::Addresses>>>,
}
impl Handle {
    pub(crate) fn new(
        buffer: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        let tree_map = Arc::new(RwLock::new(BTreeMap::new()));
        let tree_map_clone = tree_map.clone();
        let mut listener = swarm_event_source.subscribe();
        tokio::spawn(async move {
            while let Ok(ev) = listener.recv().await {
                if let SwarmEvent::Behaviour(BehaviourEvent::Kad(OutEvent::RoutingUpdated {
                    peer,
                    addresses,
                    ..
                })) = ev.as_ref()
                {
                    tree_map
                        .write()
                        .expect("Lock not poisoned")
                        .insert(*peer, addresses.clone());
                }
            }
        });
        (
            Self {
                sender: tx,
                swarm_event_source: swarm_event_source.clone(),
                tree_map: tree_map_clone,
            },
            rx,
        )
    }
    generate_handler_method!(
        InsertNode:insert_node(peer_id:PeerId, address:Multiaddr)->kad::RoutingUpdate;
        BootStrap:bootstrap()->Result<kad::QueryId,kad::NoKnownPeers>;
    );
    pub async fn query(&self, peer_id: PeerId) -> Vec<kad::QueryResult> {
        let mut listener = self.swarm_event_source.subscribe();
        let (callback_tx, callback_rx) = oneshot::channel();

        self.sender
            .send(InEvent::PeerLookup(peer_id, callback_tx))
            .await
            .expect("sending event to succeed");
        let query_id = callback_rx.await.expect("callback to succeed");
        let mut results = Vec::new();
        let handle = tokio::spawn(listen_event!(
            listener for Kad,
            OutEvent::OutboundQueryProgressed {
                id,
                result,
                step,
                ..
            }=>
            {
                if query_id != *id {
                    continue;
                }
                results.push(result.clone());
                if step.last {
                    drop(listener);
                    return results;
                }
            }
        ));
        handle.await.unwrap()
    }
    pub async fn lookup(&self, peer_id: &PeerId) -> Option<kad::Addresses> {
        self.tree_map
            .read()
            .expect("Lock not poisoned.")
            .get(peer_id)
            .cloned()
    }
    pub async fn all_records(&self) -> BTreeMap<PeerId, kad::Addresses> {
        self.tree_map.read().expect("Lock not poisoned.").clone()
    }
    pub async fn set_mode(&self, mode: Option<kad::Mode>) -> Result<kad::Mode, ()> {
        let ev = InEvent::SetMode(mode);
        let mut listener = self.swarm_event_source.subscribe();
        self.sender.send(ev).await.expect("send to succeed");
        let fut = listen_event!(listener for Kad, OutEvent::ModeChanged { new_mode }=>{
            return *new_mode;
        });
        match with_timeout!(fut, 10) {
            Ok(result) => Ok(result),
            Err(_) => Err(()),
        }
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    use InEvent::*;

    match ev {
        PeerLookup(peer_id, callback) => {
            let query_id = behav.get_record(kad::RecordKey::new(&peer_id.to_bytes()));
            handle_callback_sender!(query_id=>callback);
        }
        BootStrap(callback) => {
            let result = behav.bootstrap();
            handle_callback_sender!(result=>callback);
        }
        SetMode(mode) => behav.set_mode(mode),
        InsertNode(peer, address, callback) => {
            let result = behav.add_address(&peer, address);
            handle_callback_sender!(result=>callback);
        }
    }
}

pub fn ev_dispatch(ev: &OutEvent) {
    use kad::Event::*;
    match ev{
        InboundRequest { request } => info!("Incoming request: {:?}",request),
        OutboundQueryProgressed { id, result, stats, step } => debug!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => trace!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        ModeChanged { new_mode } => info!("The mode of this peer has been changed to {}",new_mode),
        _=>{}
    }
}

pub(crate) mod cli {
    use super::*;
    use clap::{Subcommand, ValueEnum};

    /// Subcommand for interacting with `libp2p-kad` protocol.  
    /// Kadelima protocol is an effecient routing algorithm
    /// for reaching peers in a distributed environment, while
    /// also being an excellent way of discovering more peers.    
    /// Peers can exchange their views of the network to help
    /// stitching a more complete view of the network and discover
    /// more peers.
    #[derive(Debug, Subcommand)]
    pub enum Kad {
        /// Initate a query for the given peer across the entire network,
        /// e.g all peers participating the network will be notified to look for the peer.  
        /// Logically closest peers will be returned if the given peer is not found.  
        Query {
            /// The peer to query for.
            #[arg(required = true)]
            peer_id: PeerId,
        },
        /// Try to look up the given peer on local routing table.
        /// Will return None when the peer is not found.  
        /// No request will be sent to the network.
        Lookup {
            /// The peer to look for.
            #[arg(required = true)]
            peer_id: PeerId,
        },
        /// Start bootstraping the network, e.g. contact all peers participating the network
        /// to get their view of the network in order to update the view on local peer.  
        /// New peers will be added to local routing table and contacted to get their view,
        /// so this can be a resource-intensive(CPU time, memory, network) operation, but
        /// it is essential to maintain a healthy routing table.  
        /// Bootstrapping therefore cannot be started when there is no sufficient nodes in the table.
        /// So it is recommended to issue `kad insert-default` before the first bootstrap.
        /// Bootstrapping will be automatically scheduled every 2 minutes regardless of this command.
        /// You can configure the interval in `owlnest_config.toml` if you find it too frequent.
        Bootstrap,
        /// Set current mode of local DHT provider:
        /// - `Client`: Only passively listen to the network
        /// without publishing record or answering queries.
        /// - `Server`: Actively publish records and answer queries.
        /// - `Default`: Automatically determin the mode according to
        /// public reachability of local peer. If local peer is publicly reachable,
        /// the mode will be set to `Server`, or `Client` otherwise.
        SetMode {
            /// The mode to set: `client`, `server` or `default`
            #[arg(required = true)]
            mode: KadMode,
        },
        /// Manually insert the record into local routing table.
        Insert {
            /// The peer ID to insert.
            #[arg(required = true)]
            peer_id: PeerId,
            /// The address associated with the given peer.
            /// The address will be removed if later determined unreachable.
            #[arg(required = true)]
            address: Multiaddr,
        },
        /// Insert the default nodes of the network to local routing table.
        /// Currently those peers are from official IPFS nodes. Visit
        /// https://docs.ipfs.tech/how-to/modify-bootstrap-list/ for more
        /// information about the nodes and their importance.
        InsertDefault,
    }

    #[derive(Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
    pub enum KadMode {
        Client,
        Server,
        Default,
    }
    impl From<KadMode> for Option<kad::Mode> {
        fn from(value: KadMode) -> Self {
            match value {
                KadMode::Client => Some(kad::Mode::Client),
                KadMode::Server => Some(kad::Mode::Server),
                KadMode::Default => None,
            }
        }
    }
    impl std::fmt::Display for KadMode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                KadMode::Client => write!(f, "Client"),
                KadMode::Server => write!(f, "Server"),
                KadMode::Default => write!(f, "Default"),
            }
        }
    }

    /// Top-level handler for `kad` command.
    pub async fn handle_kad(handle: &Handle, command: Kad) {
        use Kad::*;
        match command {
            Query { peer_id } => {
                let result = handle.query(peer_id).await;
                println!("{:?}", result)
            }
            Lookup { peer_id } => {
                let result = handle.lookup(&peer_id).await;
                println!("{:?}", result)
            }
            Bootstrap => {
                let result = handle.bootstrap().await;
                if result.is_err() {
                    println!("No known peer in the DHT");
                    return;
                }
                println!("Bootstrap started")
            }
            SetMode { mode } => {
                if handle.set_mode(mode.into()).await.is_err() {
                    println!("Timeout reached for setting kad mode");
                    return;
                }
                println!("Mode for kad has been set to {}", mode)
            }
            Insert { .. } => {}
            InsertDefault => {
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN").unwrap(),
                        "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
                    )
                    .await;
                println!(
                    "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa").unwrap(),
                        "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
                    )
                    .await;
                println!(
                    "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb").unwrap(),
                        "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
                    )
                    .await;
                println!(
                    "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt").unwrap(),
                        "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
                    )
                    .await;
                println!(
                    "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ").unwrap(),
                        "/ip4/104.131.131.82/tcp/4001".parse::<Multiaddr>().unwrap(),
                    )
                    .await;
                println!(
                    "QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ:{:?}",
                    result
                );
            }
        }
    }
}

pub(crate) mod swarm_hooks {
    use crate::net::p2p::swarm::Swarm;
    use libp2p::{core::ConnectedPoint, PeerId};

    #[inline]
    pub fn kad_add(swarm: &mut Swarm, peer_id: PeerId, endpoint: ConnectedPoint) {
        match endpoint {
            libp2p::core::ConnectedPoint::Dialer { address, .. } => {
                swarm.behaviour_mut().kad.add_address(&peer_id, address);
            }
            libp2p::core::ConnectedPoint::Listener { send_back_addr, .. } => {
                swarm
                    .behaviour_mut()
                    .kad
                    .add_address(&peer_id, send_back_addr);
            }
        }
    }

    #[inline]
    pub fn kad_remove(swarm: &mut Swarm, peer_id: PeerId, endpoint: ConnectedPoint) {
        match endpoint {
            libp2p::core::ConnectedPoint::Dialer { address, .. } => {
                swarm.behaviour_mut().kad.remove_address(&peer_id, &address);
            }
            libp2p::core::ConnectedPoint::Listener { send_back_addr, .. } => {
                swarm
                    .behaviour_mut()
                    .kad
                    .remove_address(&peer_id, &send_back_addr);
            }
        }
    }
}
