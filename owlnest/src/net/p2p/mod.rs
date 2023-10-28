pub mod identity;
pub mod protocols;
pub mod swarm;

use identity::IdentityUnion;
use crate::net::p2p::protocols::*;
use futures::Future;

pub struct SwarmConfig {
    pub local_ident: IdentityUnion,
    pub kad: kad::Config,
    pub identify: identify::Config,
    pub mdns: mdns::Config,
    pub messaging: messaging::Config,
    pub tethering: tethering::Config,
    pub relay_server: libp2p::relay::Config,
}
impl SwarmConfig {
    pub fn ident(&self) -> &IdentityUnion {
        &self.local_ident
    }
}

mod handler_prelude {
    pub use futures::{future::BoxFuture, FutureExt};
    pub use libp2p::swarm::{
        handler::{
            ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
        },
        ConnectionHandler, ConnectionHandlerEvent, Stream, StreamUpgradeError,
        SubstreamProtocol,
    };
    pub use std::io;
    pub use std::task::Poll;
}

async fn with_timeout<T>(mut future: impl Future<Output = T> + Unpin, timeout: u64) -> Result<T,()> {
    let mut timer = futures_timer::Delay::new(std::time::Duration::from_secs(timeout));
    tokio::select! {
        _ = &mut timer =>{
            tracing::warn!("A timeout for an event listener related task reached");
            Err(())
        }
        v = &mut future => {
            return Ok(v);
        }
    }
}
