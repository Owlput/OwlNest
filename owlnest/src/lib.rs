#![feature(byte_slice_trim_ascii)]
#![feature(io_error_downcast)]
#![feature(io_error_more)]
#![feature(map_try_insert)]
#![feature(hash_extract_if)]
#![feature(extract_if)]


pub mod cli;
pub mod macros;
pub mod net;
pub mod utils;
// pub mod db;

pub mod logging_prelude {
    pub use chrono;
    pub use tracing::{level_filters::LevelFilter, Level};
    pub use tracing_log::LogTracer;
    pub use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
}
