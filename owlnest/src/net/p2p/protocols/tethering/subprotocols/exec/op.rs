use crate::net::p2p::{swarm, protocols::{*, tethering::TetheringOp}};

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Swarm(swarm::Op),
    Tethering(TetheringOp),
    #[cfg(feature = "messaging")]
    Messaging(messaging::Op),
}
