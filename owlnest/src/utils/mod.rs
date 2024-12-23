use derive_more::derive::From;

/// Preludes for setting up logging.
pub mod logging_prelude {
    pub use chrono;
    pub use tracing::{level_filters::LevelFilter, Level};
    pub use tracing_log::LogTracer;
    pub use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
}

/// Error related to channel communication.
#[derive(Debug)]
pub enum ChannelError {
    /// The operation did not complete due to a timeout had been reached.  
    /// Future answers will be discarded.
    Timeout,
    /// The sender or receiver half of the channel has been dropped prematurely.
    ChannelClosed,
}
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ChannelError {
    fn from(_value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ChannelClosed
    }
}
impl From<tokio::sync::oneshot::error::RecvError> for ChannelError {
    fn from(_value: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::ChannelClosed
    }
}
impl std::fmt::Display for ChannelError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            ChannelError::Timeout => write!(f, "Timeout waiting for response"),
            ChannelError::ChannelClosed => write!(f, "Channel closed unexpectedly"),
        }
    }
}
impl std::error::Error for ChannelError {}

#[derive(Debug, From)]
pub enum Error<T> {
    #[from(skip)]
    Err(T),
    Channel(ChannelError),
}
impl<T> Error<T> {
    pub fn is_actual_err(&self) -> bool {
        if let Error::Err(_) = self {
            return true;
        }
        false
    }
}
impl<T, U> From<tokio::sync::mpsc::error::SendError<T>> for Error<U> {
    fn from(_value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Channel(ChannelError::ChannelClosed)
    }
}
impl<T> From<tokio::sync::oneshot::error::RecvError> for Error<T> {
    fn from(_value: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Channel(ChannelError::ChannelClosed)
    }
}

#[macro_export]
macro_rules! with_timeout {
    ($future:ident,$timeout:literal) => {{
        let timer = futures_timer::Delay::new(std::time::Duration::from_secs($timeout));
        tokio::select! {
            _ = timer =>{
                Err($crate::utils::ChannelError::Timeout)
            }
            v = $future => {
                Ok(v)
            }
        }
    }};
}
