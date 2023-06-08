use super::*;
use crate::net::p2p::swarm::op::behaviour;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    SendMessage(PeerId, Message),
}
impl Into<behaviour::Op> for Op{
    fn into(self) -> behaviour::Op {
        behaviour::Op::Messaging(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpResult {
    SuccessfulPost(Duration),
    Error(Error),
}
impl Into<behaviour::OpResult> for OpResult{
    fn into(self) -> behaviour::OpResult {
        behaviour::OpResult::Messaging(self)
    }
}
