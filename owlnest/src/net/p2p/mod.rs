pub mod identity;
pub mod protocols;
pub mod swarm;
pub use protocols::*;

use self::identity::IdentityUnion;
use super::*;
#[cfg(feature = "messaging")]
use crate::net::p2p::protocols::messaging;
#[cfg(feature = "relay-client")]
use crate::net::p2p::protocols::relay_client;
#[cfg(feature = "relay-server")]
use crate::net::p2p::protocols::relay_server;
#[cfg(feature = "tethering")]
use crate::net::p2p::protocols::tethering;
use futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use tokio::select;
use tokio::sync::{mpsc, oneshot};

pub struct SwarmConfig {
    pub local_ident: IdentityUnion,
    #[cfg(feature = "messaging")]
    pub messaging: messaging::Config,
    #[cfg(feature = "tethering")]
    pub tethering: tethering::Config,
    #[cfg(feature = "relay-server")]
    pub relay_server: libp2p::relay::v2::relay::Config,
}
impl SwarmConfig {
    pub fn ident(&self) -> IdentityUnion {
        self.local_ident.clone()
    }
}
