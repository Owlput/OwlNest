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
    record_filtering: kad::StoreInserts,
    provider_record_ttl: Option<Duration>,
    provider_publication_interval: Option<Duration>,
    kbucket_inserts: kad::BucketInserts,
    caching: kad::Caching,
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
            .set_record_filtering(record_filtering)
            .set_record_ttl(record_ttl)
            .set_replication_factor(query_config.replication_factor);
        config
            .set_caching(caching)
            .set_kbucket_inserts(kbucket_inserts)
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
            record_filtering: kad::StoreInserts::Unfiltered,
            provider_publication_interval: Some(Duration::from_secs(12 * 60 * 60)),
            provider_record_ttl: Some(Duration::from_secs(48 * 60 * 60)),
            kbucket_inserts: kad::BucketInserts::OnConnected,
            caching: kad::Caching::Enabled { max_peers: 1 },
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
    event_tx: EventSender,
    tree_map: Arc<RwLock<BTreeMap<PeerId, kad::Addresses>>>,
}
impl Handle {
    pub(crate) fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        let tree_map = Arc::new(RwLock::new(BTreeMap::new()));
        let tree_map_clone = tree_map.clone();
        let mut listener = event_tx.subscribe();
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
                event_tx: event_tx.clone(),
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
        let mut listener = self.event_tx.subscribe();
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
        let mut listener = self.event_tx.subscribe();
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
    use crate::net::p2p::swarm;
    use swarm::manager::Manager;

    /// Top-level handler for `kad` command.
    pub fn handle_kad(manager: &Manager, command: Vec<&str>) {
        if command.len() < 2 {
            println!("Missing subcommands. Type \"kad help\" for more information");
            return;
        }
        match command[1] {
            "query" => kad_query(manager, command),
            "lookup" => kad_lookup(manager, command),
            "bootstrap" => kad_bootstrap(manager),
            "set-mode" => kad_setmode(manager, command),
            "help" => println!("{}", TOP_HELP_MESSAGE),
            "insert-default" => kad_insert_default(manager),
            _ => println!("Unrecoginzed subcommands. Type \"kad help\" for more information"),
        }
    }

    fn kad_insert_default(manager: &Manager) {
        let result = manager.executor().block_on(manager.kad().insert_node(
            PeerId::from_str("QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN").unwrap(),
            "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
        ));
        println!(
            "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN:{:?}",
            result
        );
        let result = manager.executor().block_on(manager.kad().insert_node(
            PeerId::from_str("QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa").unwrap(),
            "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
        ));
        println!(
            "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa:{:?}",
            result
        );
        let result = manager.executor().block_on(manager.kad().insert_node(
            PeerId::from_str("QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb").unwrap(),
            "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
        ));
        println!(
            "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb:{:?}",
            result
        );
        let result = manager.executor().block_on(manager.kad().insert_node(
            PeerId::from_str("QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt").unwrap(),
            "/dnsaddr/bootstrap.libp2p.io".parse::<Multiaddr>().unwrap(),
        ));
        println!(
            "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt:{:?}",
            result
        );
        let result = manager.executor().block_on(manager.kad().insert_node(
            PeerId::from_str("QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ").unwrap(),
            "/ip4/104.131.131.82/tcp/4001".parse::<Multiaddr>().unwrap(),
        ));
        println!(
            "QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ:{:?}",
            result
        );
    }

    /// Handler for `kad lookup` command.
    fn kad_query(manager: &Manager, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing required argument: <peer ID>");
            return;
        }
        let peer_id = match PeerId::from_str(command[2]) {
            Ok(peer_id) => peer_id,
            Err(e) => {
                println!("Error: Failed parsing peer ID `{}`: {}", command[1], e);
                return;
            }
        };
        let result = manager.executor().block_on(manager.kad().query(peer_id));
        println!("{:?}", result)
    }

    fn kad_lookup(manager: &Manager, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing required argument: <peer ID>");
            return;
        }
        let peer_id = match PeerId::from_str(command[2]) {
            Ok(peer_id) => peer_id,
            Err(e) => {
                println!("Error: Failed parsing peer ID `{}`: {}", command[1], e);
                return;
            }
        };
        let result = manager.executor().block_on(manager.kad().lookup(&peer_id));
        println!("{:?}", result)
    }

    fn kad_bootstrap(manager: &Manager) {
        let result = manager.executor().block_on(manager.kad().bootstrap());
        if result.is_err() {
            println!("No known peer in the DHT");
            return;
        }
        println!("Bootstrap started")
    }

    fn kad_setmode(manager: &Manager, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing required argument: <mode>. Syntax: `kad set-mode <mode>`");
            return;
        }
        let mode = match command[2] {
            "client" => Some(kad::Mode::Client),
            "server" => Some(kad::Mode::Server),
            "default" => None,
            _ => {
                println!("Invalid mode, possible modes: `client`, `server`, `default`");
                return;
            }
        };
        if manager
            .executor()
            .block_on(manager.kad().set_mode(mode))
            .is_err()
        {
            println!("Timeout reached for setting kad mode");
            return;
        }
        println!("mode for kad has been set to {}", command[2])
    }

    /// Top-level help message for `kad` command.
    const TOP_HELP_MESSAGE: &str = r#"
Protocol `/ipfs/kad/1.0.0`

Available Subcommands:
    query <peer ID>        
        Initiate a query for the given peer.
        This will notify peers in the network to lookup the peer.
    
    lookup <peer ID>
        Perform lookup on local address book.

    bootstrap
        Start traversing the DHT network to get latest information
        about all peers participating the network.

    set-mode <mode>
        Set the local DHT manager to the given <mode>.
        Available modes are:
            `client`: Don't share local DHT to others.
            `server`: Broadcast local DHT to others.
            `default`: Restore to default mode, which is
                      automatically determined by local node.
"#;
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
