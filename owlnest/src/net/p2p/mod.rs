pub mod identity;
pub mod protocols;
pub mod swarm;
#[cfg(feature = "relay-client")]
pub mod relayed_swarm;

pub use protocols::*;

use super::*;
use self::identity::IdentityUnion;
use tokio::sync::{oneshot,mpsc};
use tokio::select;
use libp2p::swarm::{SwarmEvent,NetworkBehaviour};
use futures::StreamExt;

pub struct SwarmConfig {
    local_ident: IdentityUnion,
    #[cfg(feature = "messaging")]
    messaging: messaging::Config,
    #[cfg(feature = "tethering")]
    tethering: tethering::Config,
    #[cfg(feature = "relay-server")]
    relay_server: libp2p::relay::v2::relay::Config,
}
impl SwarmConfig{
    pub fn ident(&self)->IdentityUnion{
        self.local_ident.clone()
    }
}
impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            local_ident: IdentityUnion::generate(),
            #[cfg(feature = "messaging")]
            messaging: messaging::Config::default(),
            #[cfg(feature = "tethering")]
            tethering: tethering::Config::default(),
            #[cfg(feature = "relay-server")]
            relay_server: libp2p::relay::v2::relay::Config::default(),
        }
    }
}
