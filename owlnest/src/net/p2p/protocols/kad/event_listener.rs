use super::PROTOCOL_NAME;
use crate::event_bus::{prelude::*, ToEventIdentifier};

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    OnOutboundQueryProgressed = 0,
}
impl ToEventIdentifier for Kind{
    fn event_identifier(&self) -> String {
        format!("{}:{:?}", PROTOCOL_NAME, self)
    }
}

impl Into<EventKind> for Kind {
    fn into(self) -> EventKind {
        EventKind::Behaviours(BehaviourEventKind::Kad(self))
    }
}
impl std::hash::Hash for Kind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{}:{:?}", PROTOCOL_NAME, self).hash(state)
    }
}
