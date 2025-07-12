use super::*;
use crate::net::p2p::swarm::Swarm;
use std::time::Duration;

pub use libp2p::mdns::tokio::Behaviour;
pub use libp2p::mdns::Event as OutEvent;

/// Configuration for mDNS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// TTL to use for mdns records.
    pub ttl: Duration,
    /// Interval at which to poll the network for new peers. This isn't
    /// necessary during normal operation but avoids the case that an
    /// initial packet was lost and not discovering any peers until a new
    /// peer joins the network. Receiving an mdns packet resets the timer
    /// preventing unnecessary traffic.
    pub query_interval: Duration,
    /// Use IPv6 instead of IPv4.
    pub enable_ipv6: bool,
}
impl From<Config> for libp2p::mdns::Config {
    fn from(value: Config) -> Self {
        let Config {
            ttl,
            query_interval,
            enable_ipv6,
        } = value;
        libp2p::mdns::Config {
            ttl,
            query_interval,
            enable_ipv6,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(6 * 60),
            query_interval: Duration::from_secs(5 * 60),
            enable_ipv6: false,
        }
    }
}

#[derive(Debug)]
pub(crate) enum InEvent {
    ListDiscoveredNodes {
        callback: Callback<Box<[PeerId]>>,
    },
    HasNode {
        peer: PeerId,
        callback: Callback<bool>,
    },
}

/// A handle that can communicate with the behaviour within the swarm.
#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    #[allow(unused)]
    swarm_event_source: EventSender,
}
impl Handle {
    pub(crate) fn new(
        _config: &Config,
        buffer_size: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        (
            Self {
                sender: tx,
                swarm_event_source: swarm_event_source.clone(),
            },
            rx,
        )
    }
    generate_handler_method! {
        /// List all discovered nodes from mDNS.
        ListDiscoveredNodes:list_discovered_node()->Box<[PeerId]>;
        /// Check if the peer can be discovered through mDNS
        /// e.g. accessible from your local network without internet.
        HasNode:has_node(peer:&PeerId)->bool;
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    use InEvent::*;
    match ev {
        ListDiscoveredNodes { callback } => {
            let node_list = behav.discovered_nodes().copied().collect();
            handle_callback_sender!(node_list => callback)
        }
        HasNode { peer, callback } => {
            let has_node = behav
                .discovered_nodes()
                .any(|discovered| *discovered == peer);
            handle_callback_sender!(has_node => callback)
        }
    }
}

pub(crate) fn ev_dispatch(ev: &OutEvent, swarm: &mut Swarm) {
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
    match ev.clone() {
        libp2p::mdns::Event::Discovered(list) => {
            list.into_iter()
                .map(|(peer, addr)| swarm.behaviour_mut().kad.add_address(&peer, addr))
                .count();
        }
        libp2p::mdns::Event::Expired(list) => {
            list.into_iter()
                .map(|(peer, addr)| swarm.behaviour_mut().kad.remove_address(&peer, &addr))
                .count();
        }
    }
}

pub mod cli {
    use super::*;
    use clap::Subcommand;

    /// Subcommand for interacting with `libp2p-mdns` protocol.  
    /// This protocol constantly listens on the local network
    /// to see if anyone broadcasts messages to find some hosts,
    /// or with information about itself.
    #[derive(Debug, Subcommand)]
    pub enum Mdns {
        /// List all discovered peers through mDNS.
        ListDiscovered,
        /// Check if the given peer can be observed though mDNS,
        /// e.g on your local network.
        HasNode {
            /// The peer ID to check.
            #[arg(required = true)]
            peer_id: PeerId,
        },
    }

    pub async fn handle_mdns(handle: &Handle, command: Mdns) {
        use Mdns::*;
        match command {
            ListDiscovered => println!("{:?}", handle.list_discovered_node().await),
            HasNode { peer_id } => {
                let result = handle.has_node(&peer_id).await;
                println!("Is peer {peer_id} discovered through mDNS: {result}");
            }
        }
    }
}
