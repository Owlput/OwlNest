use std::time::Duration;

use crate::net::p2p::swarm::EventSender;
use crate::net::p2p::swarm::Swarm;

pub use libp2p::mdns::tokio::Behaviour;
pub use libp2p::mdns::Event as OutEvent;
use libp2p::PeerId;
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot::*};

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
impl Into<libp2p::mdns::Config> for Config {
    fn into(self) -> libp2p::mdns::Config {
        let Config {
            ttl,
            query_interval,
            enable_ipv6,
        } = self;
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
    ListDiscoveredNodes(Sender<Box<[PeerId]>>),
    HasNode(PeerId, Sender<bool>),
}

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    #[allow(unused)]
    event_tx: EventSender,
}
impl Handle {
    pub(crate) fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
            },
            rx,
        )
    }
    generate_handler_method! {
        ListDiscoveredNodes:list_discovered_node()->Box<[PeerId]>;
        HasNode:has_node(peer_id:PeerId)->bool;
    }
}

pub(crate) fn map_in_event(ev: InEvent, behav: &mut Behaviour) {
    use InEvent::*;
    match ev {
        ListDiscoveredNodes(callback) => {
            let node_list = behav.discovered_nodes().copied().collect();
            handle_callback_sender!(node_list => callback)
        }
        HasNode(peer_id, callback) => {
            let has_node = behav.discovered_nodes().any(|peer| *peer == peer_id);
            handle_callback_sender!(has_node => callback)
        }
    }
}

pub fn ev_dispatch(ev: &OutEvent, swarm: &mut Swarm) {
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

pub(crate) mod cli {
    use crate::net::p2p::swarm::manager::Manager;
    use clap::Subcommand;
    use libp2p::PeerId;

    #[derive(Debug, Subcommand)]
    pub enum Mdns {
        ListDiscovered,
        HasNode { peer_id: PeerId },
    }

    pub fn handle_mdns(manager: &Manager, command: Mdns) {
        use Mdns::*;
        match command {
            ListDiscovered => println!(
                "{:?}",
                manager
                    .executor()
                    .block_on(manager.mdns().list_discovered_node())
            ),
            HasNode { peer_id } => {
                let result = manager
                    .executor()
                    .block_on(manager.mdns().has_node(peer_id));
                println!("Peer {} discovered? {}", peer_id, result);
            }
        }
    }
}
