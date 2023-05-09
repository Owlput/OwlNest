use self::listener_event::{EventListenerKind, EventListenerOp, ListenedEvent};
use tokio::sync::{mpsc, oneshot};

pub mod bus;
pub mod listener_event;

pub fn init() {}

#[derive(Debug)]
pub enum Error {
    QueueFull,
    NotFound,
}

/// A handle used for communicating with the task that handles
/// listener registration and removal.
#[derive(Debug, Clone)]
pub struct EventBusHandle {
    sender: mpsc::Sender<EventListenerOp>,
}
impl EventBusHandle {
    pub fn add(
        &self,
        kind: impl Into<EventListenerKind>,
        buf_size: usize,
    ) -> Result<(mpsc::Receiver<ListenedEvent>, u64), Error> {
        let (event_tx, event_rx) = mpsc::channel(buf_size);
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = EventListenerOp::Add(kind.into(), event_tx, callback_tx);
        self.sender.blocking_send(op).unwrap();
        match callback_rx.blocking_recv().unwrap() {
            Ok(listener_id) => Ok((event_rx, listener_id)),
            Err(e) => Err(e),
        }
    }
    pub fn remove(
        &self,
        kind: impl Into<EventListenerKind>,
        listener_id: u64,
    ) -> Result<(), Error> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = EventListenerOp::Remove(kind.into(), listener_id, callback_tx);
        self.sender.blocking_send(op).unwrap();
        callback_rx.blocking_recv().unwrap()
    }
}

pub trait ListenerHandler {
    type Event;
    type Kind;
    type ListenerId;
    type Error;

    fn add(
        &mut self,
        kind: Self::Kind,
        buf_size: usize,
    ) -> Result<(Self::ListenerId, mpsc::Receiver<Self::Event>), Error>;
    fn remove(&mut self, listener_id: u64) -> Result<(), Error>;
}