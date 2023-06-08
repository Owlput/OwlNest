pub mod identity;
pub mod protocols;
pub mod swarm;

use self::identity::IdentityUnion;
use crate::net::p2p::protocols::*;

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
    pub use crate::net::p2p::swarm::op::behaviour::CallbackSender;
    pub use libp2p::swarm::{
        handler::{
            ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
        },
        ConnectionHandler, ConnectionHandlerEvent, KeepAlive, Stream, StreamUpgradeError,
        SubstreamProtocol,
    };
    pub use futures::{future::BoxFuture, FutureExt};
    pub use std::io;
    pub use std::task::Poll;
}
