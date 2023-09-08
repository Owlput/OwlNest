use super::Error;
use std::{any::Any, sync::Arc};
use tokio::sync::{broadcast, oneshot};

#[derive(Clone)]
pub struct ListenedEvent(String, Arc<dyn Any + Send + Sync + 'static>);
impl ListenedEvent {
    pub fn new(ident: String, event: impl Any + Send + Sync) -> Self {
        Self(ident, Arc::new(event))
    }
    pub fn kind(&self) -> String {
        self.0.clone()
    }
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.1.downcast_ref::<T>()
    }
}
impl std::fmt::Debug for ListenedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ListenedEvent").field(&self.0).finish()
    }
}

#[derive(Debug)]
pub(crate) enum EventListenerOp {
    Add(
        String,
        oneshot::Sender<Result<broadcast::Receiver<ListenedEvent>, Error>>,
    ),
}

pub trait Listenable: Sized + Send + Sync + 'static {
    fn into_listened(self) -> ListenedEvent {
        ListenedEvent::new(Self::as_event_identifier(), self)
    }
    fn as_event_identifier() -> String;
}
