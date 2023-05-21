
use crate::event_bus::prelude::*;
use super::protocol::PROTOCOL_NAME;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Kind {
    IncomingMessage = 0,
}
impl std::hash::Hash for Kind{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{}:{:?}",PROTOCOL_NAME,self).hash(state);
    }
}
impl Into<EventKind> for Kind {
    fn into(self) -> EventKind {
        EventKind::Behaviours(BehaviourEventKind::Messaging(self))
    }
}
