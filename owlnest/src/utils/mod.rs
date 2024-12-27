use std::fmt::{Debug, Display};

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
    /// The operation did not complete due to a timeout
    /// on a callback channel had been reached.  
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
impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::Timeout => write!(f, "Timeout waiting for response"),
            ChannelError::ChannelClosed => write!(f, "Channel closed unexpectedly"),
        }
    }
}
impl std::error::Error for ChannelError {}

#[derive(Debug)]
pub enum OperationError {
    Timeout,
    Interrupted,
}
impl Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(&self, f)
    }
}

#[derive(From)]
pub enum AsyncErr<T> {
    #[from(skip)]
    Err(T),
    Operation(OperationError),
    Channel(ChannelError),
}
impl<T, U> From<tokio::sync::mpsc::error::SendError<T>> for AsyncErr<U> {
    fn from(_value: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Channel(ChannelError::ChannelClosed)
    }
}
impl<T> From<tokio::sync::oneshot::error::RecvError> for AsyncErr<T> {
    fn from(_value: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Channel(ChannelError::ChannelClosed)
    }
}
impl<T> std::error::Error for AsyncErr<T> where T: Debug {}
impl<T> Display for AsyncErr<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Err(e) => T::fmt(e, f),
            Self::Operation(e) => std::fmt::Display::fmt(e, f),
            Self::Channel(e) => std::fmt::Display::fmt(e, f),
        }
    }
}
impl<T> std::fmt::Debug for AsyncErr<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Err(e) => T::fmt(e, f),
            Self::Operation(e) => <OperationError as std::fmt::Debug>::fmt(e, f),
            Self::Channel(e) => <ChannelError as std::fmt::Debug>::fmt(e, f),
        }
    }
}
#[macro_export]
macro_rules! with_timeout {
    ($future:ident,$timeout:literal) => {{
        let timer = futures_timer::Delay::new(std::time::Duration::from_secs($timeout));
        tokio::select! {
            _ = timer =>{
                Err($crate::utils::OperationError::Timeout)
            }
            v = $future => {
                Ok(v)
            }
        }
    }};
}
#[macro_export]
macro_rules! channel_timeout {
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
