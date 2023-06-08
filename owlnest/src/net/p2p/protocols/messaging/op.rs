use super::*;
use crate::net::p2p::swarm::op::behaviour;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    SendMessage(PeerId, Message),
}
impl From<Op> for behaviour::Op {
    fn from(value: Op) -> Self {
        Self::Messaging(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpResult {
    SuccessfulPost(Duration),
    Error(Error),
}
impl From<OpResult> for behaviour::OpResult {
    fn from(value: OpResult) -> Self {
        Self::Messaging(value)
    }
}
