#![feature(byte_slice_trim_ascii)]
#![feature(io_error_downcast)]
#![feature(map_try_insert)]
pub mod cli;
pub mod macros;
pub mod net;
pub mod utils;
// pub mod db;

pub mod logging_prelude{
    pub use chrono;
    pub use tracing::{Level,level_filters::LevelFilter};
    pub use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
    pub use tracing_log::LogTracer;
}