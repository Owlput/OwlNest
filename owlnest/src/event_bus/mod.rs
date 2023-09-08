use self::listened_event::EventListenerOp;
use tokio::sync::{broadcast, mpsc, oneshot};

pub mod bus;
pub mod listened_event;
pub use listened_event::ListenedEvent;

pub trait ToEventIdentifier {
    fn event_identifier(&self) -> String;
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
        self.0.recv().await.map_err(Error::Recv)
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
    pub use super::ListenedEvent;
    pub use super::{ConversionError, Error};
}
