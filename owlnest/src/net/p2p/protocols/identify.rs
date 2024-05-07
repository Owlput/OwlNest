use std::time::Duration;

pub use libp2p::identify::Behaviour;
pub use libp2p::identify::Event as OutEvent;
pub use libp2p::identify::PROTOCOL_NAME;
use serde::Deserialize;
use serde::Serialize;
use tracing::{debug, trace, warn};

use crate::net::p2p::identity::IdentityUnion;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Application-specific version of the protocol family used by the peer,
    /// e.g. `ipfs/1.0.0` or `polkadot/1.0.0`.
    pub protocol_version: String,
    /// Name and version of the local peer implementation, similar to the
    /// `User-Agent` header in the HTTP protocol.
    ///
    /// Defaults to `rust-libp2p/<libp2p-identify-version>`.
    pub agent_version: String,
    /// The interval at which identification requests are sent to
    /// the remote on established connections after the first request,
    /// i.e. the delay between identification requests.
    ///
    /// Defaults to 5 minutes.
    pub interval: u64,

    /// Whether new or expired listen addresses of the local node should
    /// trigger an active push of an identify message to all connected peers.
    ///
    /// Enabling this option can result in connected peers being informed
    /// earlier about new or expired listen addresses of the local node,
    /// i.e. before the next periodic identify request with each peer.
    ///
    /// Disabled by default.
    pub push_listen_addr_updates: bool,

    /// How many entries of discovered peers to keep before we discard
    /// the least-recently used one.
    ///
    /// Disabled by default.
    pub cache_size: usize,
}
impl Config {
    pub fn into_config(self, ident: &IdentityUnion) -> libp2p::identify::Config {
        let Config {
            protocol_version,
            agent_version,
            interval,
            push_listen_addr_updates,
            cache_size,
        } = self;
        libp2p::identify::Config::new(protocol_version, ident.get_pubkey())
            .with_agent_version(agent_version)
            .with_cache_size(cache_size)
            .with_interval(Duration::from_secs(interval))
            .with_push_listen_addr_updates(push_listen_addr_updates)
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            protocol_version: format!("owlnest/{}", env!("CARGO_PKG_VERSION")),
            agent_version: format!("rust-libp2p/owlnest/{}", env!("CARGO_PKG_VERSION")),
            interval: 5 * 60, // 5 minutes
            push_listen_addr_updates: false,
            cache_size: 100,
        }
    }
}

pub(crate) fn ev_dispatch(ev: &OutEvent) {
    use OutEvent::*;
    match ev {
        Received { peer_id, info } => {
            debug!("Identified peer {} : {}", peer_id, info.protocol_version)
        }
        Sent { peer_id } => trace!(
            "Identification information has been sent to peer {} as response",
            peer_id
        ),
        Pushed { peer_id, info } => trace!(
            "Information {:?} has been sent to peer {} for identification",
            info,
            peer_id
        ),
        Error { peer_id, error } => {
            warn!("Error when identifying peer {} with {}", peer_id, error)
        }
    }
}
