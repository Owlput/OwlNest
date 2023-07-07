use crate::net::p2p::protocols::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Tethering(tethering::TetheringOp),

    Messaging(messaging::Op),
}
