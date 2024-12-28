/// Preludes for setting up logging.
pub mod logging_prelude {
    pub use chrono;
    pub use tracing::{level_filters::LevelFilter, Level};
    pub use tracing_log::LogTracer;
    pub use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
}

#[macro_export]
macro_rules! with_timeout {
    ($future:ident,$timeout:literal) => {{
        let timer = futures_timer::Delay::new(std::time::Duration::from_secs($timeout));
        tokio::select! {
            _ = timer =>{
                Err(owlnest_core::error::OperationError::Timeout)
            }
            v = $future => {
                Ok(v)
            }
        }
    }};
}
#[macro_export]
macro_rules! future_timeout {
    ($future:ident,$timeout:literal) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $future)
            .await
            .map_err(owlnest_core::error::OperationError::from)
    };
}
#[macro_export]
macro_rules! callback_timeout {
    ($callback:ident,$timeout:literal) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $future)
            .await
            .map_err(owlnest_core::error::ChannelError::Timeout)?
            .map_err(owlnest_core::error::ChannelError::Closed)
    };
}

#[macro_export]
macro_rules! channel_timeout {
    ($future:ident,$timeout:literal) => {{
        let timer = futures_timer::Delay::new(std::time::Duration::from_secs($timeout));
        tokio::select! {
            _ = timer =>{
                Err(owlnest_core::error::ChannelError::Timeout)
            }
            v = $future => {
                Ok(v)
            }
        }
    }};
}

#[macro_export]
macro_rules! send_swarm {
    ($sender:expr,$ev:expr) => {
        $sender
            .send($ev)
            .await
            .expect(owlnest_core::expect::SWARM_RECEIVER_KEPT_ALIVE)
    };
}
#[macro_export]
macro_rules! handle_callback {
    ($receiver:expr) => {
        $receiver.await.expect(owlnest_core::expect::CALLBACK_CLEAR)
    };
}
#[macro_export]
macro_rules! sleep {
    ($sleep_ms:expr) => {
        std::thread::sleep(std::time::Duration::from_millis($sleep_ms));
    };
}
