/// Code for importing and exporting identities(keypairs).
pub mod identity;
/// All protocols supported by OwlNest.
pub mod protocols;
/// The libp2p swarm that manages all connections.
pub mod swarm;

use crate::net::p2p::protocols::*;
use serde::{Deserialize, Serialize};
use swarm::manager::Manager;
use tokio::sync::Notify;

pub use libp2p::Multiaddr;
pub use libp2p::PeerId;

/// Config struct for the libp2p swarm that can be read from or write into a file.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// General config for the swarm. Please refer to the inner struct for more information.
    pub swarm: swarm::Config,
    /// Config for `libp2p-autonat`.
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-autonat"))]
    pub autonat: autonat::Config,
    /// Config for `libp2p-kad`.
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
    pub kad: kad::Config,
    /// Config for `libp2p-identify`.
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
    pub identify: identify::Config,
    /// Config for `libp2p-mdns`.
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
    pub mdns: mdns::Config,
    /// Config for `owlnest-messaging`.
    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
    pub messaging: messaging::Config,
    /// Config for `owlnest-blob`.
    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-blob"))]
    pub blob: blob::config::Config,
    /// Config for `owlnest-advertise`
    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
    pub advertise: advertise::config::Config,
    /// Config for server part of `libp2p-relay`
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
    pub relay_server: relay_server::Config,
    /// Config for client part of `libp2p-relay`
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"))]
    pub gossipsub: gossipsub::Config,
}

/// Some utility functions for setting up tests
pub mod test_suit {

    use super::*;
    /// Set up a swarm with default config and random identity
    /// on a dedicated `tokio` runtime.
    pub fn setup_default() -> (Manager, std::sync::Arc<Notify>) {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Tokio runtime to be created successfully");
        let ident = identity::IdentityUnion::generate();
        let guard = rt.enter();
        let swarm_config = SwarmConfig {
            swarm: Default::default(),
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-autonat"))]
            autonat: protocols::autonat::Config::default(),
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
            kad: protocols::kad::Config::default(),
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
            identify: protocols::identify::Config::default(),
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
            mdns: protocols::mdns::Config::default(),
            #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
            messaging: protocols::messaging::Config::default(),
            #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-blob"))]
            blob: blob::config::Config::default(),
            #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
            advertise: advertise::config::Config::default(),
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
            relay_server: protocols::relay_server::Config::default(),
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"))]
            gossipsub: gossipsub::Config::default(),
        };
        let mgr = swarm::Builder::new(swarm_config).build(ident, rt.handle().clone());
        drop(guard);
        let shutdown_notifier = std::sync::Arc::new(Notify::const_new());
        let notifier_clone = shutdown_notifier.clone();
        std::thread::spawn(move || {
            rt.block_on(notifier_clone.notified());
        });
        (mgr, shutdown_notifier)
    }
}
