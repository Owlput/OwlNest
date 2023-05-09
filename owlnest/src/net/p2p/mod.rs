pub mod identity;
pub mod protocols;
pub mod swarm;

pub use protocols::*;

use self::identity::IdentityUnion;
use super::*;

use crate::net::p2p::protocols::messaging;

use crate::net::p2p::protocols::relay_client;

use crate::net::p2p::protocols::relay_server;

use crate::net::p2p::protocols::tethering;
use futures::StreamExt;
use tokio::select;
use tokio::sync::{mpsc, oneshot};

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
    pub fn ident(&self) -> IdentityUnion {
        self.local_ident.clone()
    }
}
