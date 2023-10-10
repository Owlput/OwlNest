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
    pub fn blocking_add(&self, ident: String) -> Result<broadcast::Receiver<ListenedEvent>, Error> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = EventListenerOp::Add(ident, callback_tx);
        self.sender.blocking_send(op)?;
        Ok(callback_rx.blocking_recv()??)
    }
    pub async fn add(&self, ident: String) -> Result<broadcast::Receiver<ListenedEvent>, Error> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = EventListenerOp::Add(ident, callback_tx);
        self.sender.send(op).await?;
        Ok(callback_rx.await??)
    }
}

#[derive(Debug)]
pub enum ConversionError {
    Mismatch,
}

#[derive(Debug)]
pub enum Error {
    Recv(oneshot::error::RecvError),
    Send,
    Callback,
    Conversion(ConversionError),
}
impl From<oneshot::error::RecvError> for Error {
    fn from(value: oneshot::error::RecvError) -> Self {
        Self::Recv(value)
    }
}
impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_value: mpsc::error::SendError<T>) -> Self {
        Self::Send
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

#[macro_export]
macro_rules! single_value_filter {
    ($listener:ident::<$type_downcast:ty>,|$variable:ident|{$($condition:tt)*}) => {
        async move{
            let condition = |$variable:&$type_downcast| {$($condition)*};
            while let Ok(v) = $listener.recv().await {
                let ev = v.downcast_ref::<$type_downcast>().expect("downcast to succeed");
                if condition(ev){
                    return Ok(ev.clone())
                }
            }
            Err(())
        }
    };
    ($listener:ident::<$type_downcast:ty>,|$filter_var:ident|{$($condition:tt)*},|$eval_var:ident|{$($eval:tt)*}) => {
        async move{
            let condition = |$filter_var:&$type_downcast| {$($condition)*};
            let eval = |$eval_var:&$type_downcast| {$($eval)*};
            while let Ok(v) = $listener.recv().await {
                let ev = v.downcast_ref::<$type_downcast>().expect("downcast to succeed");
                if condition(ev){
                    return eval(ev);
                }
            }
            Err(())
        }
    };
}
