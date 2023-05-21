use crate::event_bus::{listened_event::*, ConversionError};

#[repr(i8)]
#[derive(Debug)]
pub enum Kind {
    OnOutboundQueryProgressed = 0,
}
impl Into<EventKind> for Kind {
    fn into(self) -> EventKind {
        EventKind::Behaviours(BehaviourEventKind::Kad(self))
    }
}

impl Into<ListenedEvent> for super::OutEvent{
    fn into(self) -> ListenedEvent {
        ListenedEvent::Behaviours(BehaviourEvent::Kad(self))
    }
}
impl TryFrom<ListenedEvent> for super::OutEvent{
    type Error = ConversionError;

    fn try_from(value: ListenedEvent) -> Result<Self, Self::Error> {
        match value{
            ListenedEvent::Behaviours(BehaviourEvent::Kad(ev)) => Ok(ev),
            _=>Err(ConversionError::Mismatch)
        }
    }
}


