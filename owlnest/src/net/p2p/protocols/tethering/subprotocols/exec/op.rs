use crate::net::p2p::{protocols::*, swarm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Swarm(swarm::SwarmOp),
    Tethering(tethering::TetheringOp),
    
    Messaging(messaging::Op),
}
