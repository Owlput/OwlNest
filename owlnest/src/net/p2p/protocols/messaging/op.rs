use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    SendMessage(PeerId, Message),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpResult {
    SuccessfulPost(Duration),
    Error(Error),
}