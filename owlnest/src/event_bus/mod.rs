use self::listened_event::EventListenerOp;
use std::{any::Any, fmt::Debug, sync::Arc};
use tokio::sync::{broadcast, mpsc, oneshot};

pub mod bus;
pub mod listened_event;

pub trait ListenableEvent {}

pub trait ToEventIdentifier {
    fn event_identifier(&self) -> String;
}

#[derive(Clone)]
pub struct ListenedEvent(String, Arc<Box<dyn Any + Send + Sync + 'static>>);
impl ListenedEvent {
    pub fn new(ident:String,event: impl Any + Send + Sync) -> Self {
        Self(ident, Arc::new(Box::new(event)))
    }
    pub fn kind(&self) -> String {
        self.0.clone()
    }
    pub fn downcast_ref<T:'static>(&self) -> Option<&T> {
        self.1.downcast_ref::<T>()
    }
}
impl Debug for ListenedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ListenedEvent").field(&self.0).finish()
    }
}

pub fn init() {}

/// A handle used for communicating with the task that handles
/// listener registration and removal.
#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<EventListenerOp>,
}
impl Handle {
    pub(crate) fn new(sender: mpsc::Sender<EventListenerOp>) -> Self {
        Self { sender }
    }
    pub fn add(&self, ident: String) -> Result<Listener, Error> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = EventListenerOp::Add(ident, callback_tx);
        self.sender.blocking_send(op).unwrap();
        match callback_rx.blocking_recv().unwrap() {
            Ok(listener) => Ok(Listener::new(listener)),
            Err(e) => Err(e),
        }
    }
}

pub struct Listener(broadcast::Receiver<ListenedEvent>);

impl Listener {
    pub(crate) fn new(listener: broadcast::Receiver<ListenedEvent>) -> Listener {
        Self(listener)
    }
    pub async fn recv(&mut self) -> Result<ListenedEvent, Error> {
        Ok(self.0.recv().await.map_err(Error::Recv)?)
    }
}

#[derive(Debug)]
pub enum ConversionError {
    Mismatch,
}

#[derive(Debug)]
pub enum Error {
    Recv(broadcast::error::RecvError),
    Conversion(ConversionError),
}
impl From<broadcast::error::RecvError> for Error {
    fn from(value: broadcast::error::RecvError) -> Self {
        Self::Recv(value)
    }
}
impl From<ConversionError> for Error {
    fn from(value: ConversionError) -> Self {
        Self::Conversion(value)
    }
}

pub mod prelude {
    pub use super::listened_event::BehaviourEvent;
    pub use super::listened_event::{AsEventKind, BehaviourEventKind, EventKind};
    pub use super::{ConversionError, Error};
    pub use super::{ListenableEvent, ListenedEvent};
}
