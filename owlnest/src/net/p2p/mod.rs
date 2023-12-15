pub mod identity;
pub mod protocols;
pub mod swarm;

use std::sync::Arc;

use crate::net::p2p::protocols::*;
use identity::IdentityUnion;
use tokio::sync::Notify;
use swarm::manager::Manager;

pub use libp2p::PeerId;
pub use libp2p::Multiaddr;

pub struct SwarmConfig {
    pub local_ident: IdentityUnion,
    pub kad: kad::Config,
    pub identify: identify::Config,
    pub mdns: mdns::Config,
    pub messaging: messaging::Config,
    #[cfg(feature = "tethering")]
    pub tethering: tethering::Config,
    pub relay_server: libp2p::relay::Config,
}
impl SwarmConfig {
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

pub fn setup_default() -> (Manager,Arc<Notify>) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let ident = IdentityUnion::generate();
    let guard = rt.enter();
    let swarm_config = SwarmConfig {
        local_ident: ident.clone(),
        kad: protocols::kad::Config::default(),
        identify: protocols::identify::Config::new("/owlnest/0.0.1".into(), ident.get_pubkey()),
        mdns: protocols::mdns::Config::default(),
        messaging: protocols::messaging::Config::default(),
        #[cfg(feature="tethering")]
        tethering: protocols::tethering::Config,
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
    (mgr,shutdown_notifier)
}

#[macro_export]
macro_rules! with_timeout {
    ($future:ident,$timeout:literal) => {{
        let timer = futures_timer::Delay::new(std::time::Duration::from_secs($timeout));
        tokio::select! {
            _ = timer =>{
                Err(())
            }
            v = $future => {
                Ok(v)
            }
        }
    }};
}
