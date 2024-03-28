use crate::net::p2p::protocols::*;
use serde::{Deserialize, Serialize};

/// Operations that can be executed remotely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Tethering(tethering::TetheringOp),
}
