pub mod identity;
pub mod protocols;
pub mod swarm;

use std::io::stdout;
use std::sync::Arc;

use crate::net::p2p::protocols::*;
use identity::IdentityUnion;
use swarm::manager::Manager;
use tokio::sync::Notify;

pub use libp2p::Multiaddr;
pub use libp2p::PeerId;

pub struct SwarmConfig {
    pub local_ident: IdentityUnion,
    #[cfg(feature = "libp2p-protocols")]
    pub kad: kad::Config,
    #[cfg(feature = "libp2p-protocols")]
    pub identify: identify::Config,
    #[cfg(feature = "libp2p-protocols")]
    pub mdns: mdns::Config,
    #[cfg(feature = "owlnest-protocols")]
    pub messaging: messaging::Config,
    #[cfg(feature = "libp2p-protocols")]
    pub relay_server: libp2p::relay::Config,
}
impl SwarmConfig {
    pub fn default_with_ident(ident: &IdentityUnion) -> Self {
        Self {
            local_ident: ident.clone(),
            #[cfg(feature = "libp2p-protocols")]
            kad: Default::default(),
            #[cfg(feature = "libp2p-protocols")]
            identify: identify::Config::new("/owlnest/0.0.1".into(), ident.get_pubkey()),
            #[cfg(feature = "libp2p-protocols")]
            mdns: Default::default(),
            #[cfg(feature = "owlnest-protocols")]
            messaging: Default::default(),
            #[cfg(feature = "libp2p-protocols")]
            relay_server: Default::default(),
        }
    }
    pub fn ident(&self) -> &IdentityUnion {
        &self.local_ident
    }
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

// async fn with_timeout<T>(mut future: impl Future<Output = T> + Unpin, timeout: u64) -> Result<T,()> {
//     let mut timer = futures_timer::Delay::new(std::time::Duration::from_secs(timeout));
//     tokio::select! {
//         _ = &mut timer =>{
//             Err(())
//         }
//         v = &mut future => {
//             return Ok(v);
//         }
//     }
// }

pub mod test_suit{
    use super::*;
    pub fn setup_default() -> (Manager, Arc<Notify>) {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let ident = IdentityUnion::generate();
        let guard = rt.enter();
        let swarm_config = SwarmConfig {
            local_ident: ident.clone(),
            #[cfg(feature = "libp2p-protocols")]
            kad: protocols::kad::Config::default(),
            #[cfg(feature = "libp2p-protocols")]
            identify: protocols::identify::Config::new("/owlnest/0.0.1".into(), ident.get_pubkey()),
            #[cfg(feature = "libp2p-protocols")]
            mdns: protocols::mdns::Config::default(),
            #[cfg(feature = "owlnest-protocols")]
            messaging: protocols::messaging::Config::default(),
            #[cfg(feature = "libp2p-protocols")]
            relay_server: protocols::relay_server::Config::default(),
        };
        let mgr = swarm::Builder::new(swarm_config).build(8, rt.handle().clone());
        drop(guard);
        let shutdown_notifier = std::sync::Arc::new(Notify::const_new());
        let notifier_clone = shutdown_notifier.clone();
        std::thread::spawn(move || {
            let rt = rt;
            let _ = rt.block_on(notifier_clone.notified());
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
            .with_target("owlnest", Level::DEBUG)
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