use crate::net::p2p::swarm;

use super::*;
#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Swarm(swarm::Op),
    Tethering(TetheringOp),
    #[cfg(feature = "messaging")]
    Messaging(messaging::Op),
}
#[derive(Debug, Serialize, Deserialize)]
pub enum TetheringOp {
    Trust(PeerId),
    Untrust(PeerId),
}