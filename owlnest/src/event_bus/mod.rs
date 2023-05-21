use std::{fmt::Debug, marker::PhantomData};

use self::listened_event::{EventKind, EventListenerOp, ListenedEvent};
use tokio::sync::{broadcast, mpsc, oneshot};

pub mod bus;
pub mod listened_event;

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
    pub fn add<T>(&self, kind: impl Into<EventKind>) -> Result<Listener<T>, Error>
    where
        T: TryFrom<ListenedEvent, Error = ConversionError> + Send + Clone,
    {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = EventListenerOp::Add(kind.into(), callback_tx);
        self.sender.blocking_send(op).unwrap();
        match callback_rx.blocking_recv().unwrap() {
            Ok(listener) => Ok(Listener::new(listener)),
            Err(e) => Err(e),
        }
    }
}

pub struct Listener<T>
where
    T: TryFrom<ListenedEvent> + Send + Clone,
{
    inner: broadcast::Receiver<ListenedEvent>,
    phantom: PhantomData<T>,
}
impl<T> Listener<T>
where
    T: TryFrom<ListenedEvent, Error = ConversionError> + Send + Clone,
{
    pub fn new(listener: broadcast::Receiver<ListenedEvent>) -> Listener<T> {
        Self {
            inner: listener,
            phantom: PhantomData::default(),
        }
    }
    pub async fn recv(&mut self) -> Result<T, Error> {
        let ev = self.inner.recv().await.map_err(Error::Recv)?;
        ev.try_into().map_err(Error::Conversion)
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
    pub use super::listened_event::{AsEventKind, EventKind, BehaviourEventKind};
    pub use super::listened_event::{ListenedEvent, BehaviourEvent};
    pub use super::{ConversionError, Error};
}
