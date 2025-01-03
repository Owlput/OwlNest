use super::*;
use crate::net::p2p::swarm::{behaviour::BehaviourEvent, SwarmEvent};
use libp2p::StreamProtocol;
use owlnest_core::error::OperationError;
use std::str::FromStr;
use tracing::{debug, info, trace};

pub use libp2p::kad;
/// An alias to the behaviour with in-memory record store.
pub type Behaviour = kad::Behaviour<kad::store::MemoryStore>;
/// An ailas to `libp2p::kad::Event` for unified naming.
pub type OutEvent = kad::Event;
pub use libp2p::kad::PROTOCOL_NAME;

type PeerTreeMap = std::collections::BTreeMap<PeerId, kad::Addresses>;

/// Equivalent to `libp2p::kad::Config` that supports `serde`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    max_packet_size: usize,
    query_config: config::QueryConfig,
    record_ttl_sec: Option<u64>,
    record_replication_interval_sec: Option<u64>,
    record_publication_interval_sec: Option<u64>,
    record_filtering: config::StoreInserts,
    provider_record_ttl_sec: Option<u64>,
    provider_publication_interval_sec: Option<u64>,
    kbucket_inserts: config::BucketInserts,
    caching: config::Caching,
    periodic_bootstrap_interval_sec: Option<u64>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_packet_size: 16 * 1024,
            query_config: config::QueryConfig::default(),
            record_ttl_sec: Some(48 * 60 * 60), // 2 days
            record_replication_interval_sec: Some(60 * 60), // 1 hr
            record_publication_interval_sec: Some(22 * 60 * 60), // 22 hr
            record_filtering: config::StoreInserts::Unfiltered,
            provider_publication_interval_sec: Some(12 * 60 * 60), // 12 hr
            provider_record_ttl_sec: Some(48 * 60 * 60),           // 2 days
            kbucket_inserts: config::BucketInserts::OnConnected,
            caching: config::Caching::Enabled { max_peers: 1 },
            periodic_bootstrap_interval_sec: Some(5 * 60), // 5 min
        }
    }
}
impl Config {
    /// Convert to `libp2p::kad::Config` with a ptotocol string.  
    /// ### Protocol string  
    /// The protocol string is used to distinguish between different
    /// kadelima networks.
    /// ### Prevent unnecessary network merging
    /// If two kadelima networks are different enough
    /// they should use different protocol string to prevent merging.  
    /// Merging different kadlima networks will reduce effeciency of both
    /// networks due to extra peers that aren't of help.
    pub fn into_config(self, protocol: String) -> kad::Config {
        use std::time::Duration;
        let Config {
            max_packet_size,
            query_config,
            record_ttl_sec,
            record_replication_interval_sec,
            record_publication_interval_sec,
            record_filtering,
            provider_record_ttl_sec,
            provider_publication_interval_sec,
            kbucket_inserts,
            caching,
            periodic_bootstrap_interval_sec,
        } = self;
        let mut config = kad::Config::new(
            StreamProtocol::try_from_owned(protocol)
                .expect("protocol string start with a slash('/')"),
        );
        config
            .disjoint_query_paths(query_config.disjoint_query_paths)
            .set_parallelism(query_config.parallelism)
            .set_periodic_bootstrap_interval(
                periodic_bootstrap_interval_sec.map(|sec| Duration::from_secs(sec)),
            )
            .set_provider_record_ttl(provider_record_ttl_sec.map(|sec| Duration::from_secs(sec)))
            .set_query_timeout(query_config.timeout)
            .set_record_filtering(record_filtering.into())
            .set_record_ttl(record_ttl_sec.map(|sec| Duration::from_secs(sec)))
            .set_replication_factor(query_config.replication_factor)
            .set_caching(caching.into())
            .set_kbucket_inserts(kbucket_inserts.into())
            .set_max_packet_size(max_packet_size)
            .set_provider_publication_interval(
                provider_publication_interval_sec.map(|sec| Duration::from_secs(sec)),
            )
            .set_replication_interval(
                record_replication_interval_sec.map(|sec| Duration::from_secs(sec)),
            )
            .set_publication_interval(
                record_publication_interval_sec.map(|sec| Duration::from_secs(sec)),
            );
        config
    }
}

/// Equivalent of `libp2p::kad::config` module with types that support `serde`.
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
        Enabled {
            /// Maximum amount of peers to store in the cache.
            max_peers: u16,
        },
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
    PeerLookup {
        peer_id: PeerId,
        callback: Callback<kad::QueryId>,
    },
    BootStrap(Callback<Result<kad::QueryId, kad::NoKnownPeers>>),
    InsertNode {
        peer_id: PeerId,
        address: Multiaddr,
        callback: Callback<kad::RoutingUpdate>,
    },
    SetMode(Option<kad::Mode>),
}

/// A handle that can communicate with the behaviour within the swarm.
#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    swarm_event_source: EventSender,
    tree_map: std::sync::Arc<std::sync::RwLock<PeerTreeMap>>,
}
impl Handle {
    pub(crate) fn new(
        _config: &Config,
        buffer_size: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let tree_map: std::sync::Arc<std::sync::RwLock<PeerTreeMap>> = Default::default();
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
    /// Start a query that goes through the entire network.
    pub async fn query(&self, peer_id: PeerId) -> Vec<kad::QueryResult> {
        let mut listener = self.swarm_event_source.subscribe();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(InEvent::PeerLookup {
                peer_id,
                callback: tx,
            })
            .await
            .expect("sending event to succeed");
        let query_id = rx.await.expect("callback to succeed");
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
    /// Perform a lookup on local peer store.
    pub async fn lookup(&self, peer_id: &PeerId) -> Option<kad::Addresses> {
        self.tree_map
            .read()
            .expect("Lock not poisoned.")
            .get(peer_id)
            .cloned()
    }
    /// Get all records stored on local node.
    pub async fn all_records(&self) -> PeerTreeMap {
        self.tree_map.read().expect("Lock not poisoned.").clone()
    }
    /// Set the current kadelima behaviour mode.
    /// ### Client mode
    /// Only listen on the network for updates without
    /// sharing the local peer store.
    /// ### Server mode
    /// Actively publish records in the local store.
    /// ### Routing table
    /// Only peers in `server` mode will be added to routing tables.
    pub async fn set_mode(&self, mode: Option<kad::Mode>) -> Result<kad::Mode, OperationError> {
        let ev = InEvent::SetMode(mode);
        let mut listener = self.swarm_event_source.subscribe();
        self.sender.send(ev).await.expect("send to succeed");
        let fut = listen_event!(listener for Kad, OutEvent::ModeChanged { new_mode }=>{
            return *new_mode;
        });
        future_timeout!(fut, 1000)
    }
    generate_handler_method!(
        /// Start bootstrapping.
        /// ## Bootstrapping in kadlima network
        /// Bootstrapping means perform a walk-through of the network to gain
        /// more information of the network. Between walk-throughs(bootstrappings),
        /// some peer may join the network while others may leave, so regular
        /// bootstrapping will help maintain a healthy network with up-to-date
        /// information.
        /// ## Bootstrapping on a newly started peer
        /// For a newly started peer, its kadlima record store is empty, which means
        /// the new peer have no idea of the network, hence unable to query for
        /// information of the network(there is no one to talk to). So you should
        /// manually insert some record(at least one known and reachable peer) for
        /// bootstrapping.
        /// ### Hijaking
        /// If the node used for initial bootstrapping is malicious, the new peer
        /// is vulnerable to [sybil attack](https://ssg.lancs.ac.uk/wp-content/uploads/ndss_preprint.pdf),
        /// which means higher chances of encounting other malicious peers that
        /// may breach trusts and consensus.
        /// So it is VERY important to choose bootstrapping nodes carefully and
        /// only use those peers you trust rather than a random node.
        BootStrap:bootstrap()->Result<kad::QueryId,kad::NoKnownPeers>;
    );
    generate_handler_method!(
        /// Mannually insert a record to the store.
        InsertNode:insert_node{peer_id:PeerId, address:Multiaddr}->kad::RoutingUpdate;
    );
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    use InEvent::*;

    match ev {
        PeerLookup { peer_id, callback } => {
            let query_id = behav.get_record(kad::RecordKey::new(&peer_id.to_bytes()));
            handle_callback_sender!(query_id=>callback);
        }
        BootStrap(callback) => {
            let result = behav.bootstrap();
            handle_callback_sender!(result=>callback);
        }
        SetMode(mode) => behav.set_mode(mode),
        InsertNode {
            peer_id,
            address,
            callback,
        } => {
            let result = behav.add_address(&peer_id, address);
            handle_callback_sender!(result=>callback);
        }
    }
}

pub(crate) fn ev_dispatch(ev: &OutEvent) {
    use kad::Event::*;
    match ev{
        InboundRequest { request } => info!("Incoming request: {:?}",request),
        OutboundQueryProgressed { id, result, stats, step } => debug!("Outbound query {:?} progressed, stats: {:?}, step: {:?}, result: {:?}",id,stats,step,result),
        RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => trace!("Peer {} updated the table, is new peer: {}, addresses: {:?}, bucket range: {:?}, old peer?: {:?}",peer, is_new_peer,addresses,bucket_range,old_peer),
        ModeChanged { new_mode } => info!("The mode of this peer has been changed to {}",new_mode),
        _=>{}
    }
}

pub mod cli {
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
        ///   without publishing record or answering queries.
        /// - `Server`: Actively publish records and answer queries.
        /// - `Default`: Automatically determin the mode according to
        ///   public reachability of local peer. If local peer is publicly reachable,
        ///   the mode will be set to `Server`, or `Client` otherwise.
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

    /// The mode kadelima is currently working at.
    #[derive(Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
    pub enum KadMode {
        /// Local node only listens to other record providers and don't
        /// publish records it has.
        Client,
        /// Local node will actively provide records to other peers.
        /// Only peers in `Server` mode will be put into the routing table.
        Server,
        /// The mode will be determined automatically:
        /// - `Client` when local node is not publicly reachable.
        /// - `Server` when local node is publicly reachable.
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
                        PeerId::from_str("QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN")
                            .expect("parsing to succeed"),
                        "/dnsaddr/bootstrap.libp2p.io"
                            .parse::<Multiaddr>()
                            .expect("parsing to succeed"),
                    )
                    .await;
                println!(
                    "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa")
                            .expect("parsing to succeed"),
                        "/dnsaddr/bootstrap.libp2p.io"
                            .parse::<Multiaddr>()
                            .expect("parsing to succeed"),
                    )
                    .await;
                println!(
                    "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb")
                            .expect("parsing to succeed"),
                        "/dnsaddr/bootstrap.libp2p.io"
                            .parse::<Multiaddr>()
                            .expect("parsing to succeed"),
                    )
                    .await;
                println!(
                    "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt")
                            .expect("parsing to succeed"),
                        "/dnsaddr/bootstrap.libp2p.io"
                            .parse::<Multiaddr>()
                            .expect("parsing to succeed"),
                    )
                    .await;
                println!(
                    "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt:{:?}",
                    result
                );
                let result = handle
                    .insert_node(
                        PeerId::from_str("QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ")
                            .expect("parsing to succeed"),
                        "/ip4/104.131.131.82/tcp/4001"
                            .parse::<Multiaddr>()
                            .expect("parsing to succeed"),
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
