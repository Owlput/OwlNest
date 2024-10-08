pub mod identity;
pub mod protocols;
pub mod swarm;

use std::io::stdout;
use std::sync::Arc;

use crate::net::p2p::protocols::*;
use identity::IdentityUnion;
use serde::Deserialize;
use serde::Serialize;
use swarm::manager::Manager;
use tokio::sync::Notify;

pub use libp2p::Multiaddr;
pub use libp2p::PeerId;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub swarm: swarm::Config,
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
    pub kad: kad::Config,
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
    pub identify: identify::Config,
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
    pub mdns: mdns::Config,
    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
    pub messaging: messaging::Config,
    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-blob"))]
    pub blob: blob::config::Config,
    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
    pub advertise: advertise::config::Config,
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
    pub relay_server: relay_server::Config,
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"))]
    pub gossipsub: gossipsub::Config,
}

#[allow(unused)]
mod handler_prelude {
    pub use futures::{future::BoxFuture, FutureExt};
    pub use libp2p::swarm::{
        handler::{
            ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
        },
        ConnectionHandler, ConnectionHandlerEvent, Stream, StreamUpgradeError, SubstreamProtocol,
    };
    pub use std::io;
    pub use std::task::Poll;
}

pub mod test_suit {

    use super::*;
    pub fn setup_default() -> (Manager, Arc<Notify>) {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let ident = IdentityUnion::generate();
        let guard = rt.enter();
        let swarm_config = SwarmConfig {
            swarm: Default::default(),
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

    #[allow(unused)]
    // tests only
    pub fn setup_logging() {
        use std::sync::Mutex;
        use tracing::level_filters::LevelFilter;
        use tracing::Level;
        use tracing_log::LogTracer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Layer;
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("owlnest", Level::TRACE)
            .with_target("rustyline", LevelFilter::ERROR)
            .with_target("libp2p_noise", Level::WARN)
            .with_target("libp2p_mdns", Level::DEBUG)
            .with_target("hickory_proto", Level::WARN)
            .with_target("", Level::DEBUG);
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_ansi(false)
            .with_writer(Mutex::new(stdout()))
            .with_filter(filter);
        let reg = tracing_subscriber::registry().with(layer);
        tracing::subscriber::set_global_default(reg).expect("you can only set global default once");
        LogTracer::init().unwrap()
    }
}
