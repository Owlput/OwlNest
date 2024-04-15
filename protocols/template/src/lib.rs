#![allow(unused)]
use owlnest_prelude::lib_prelude::*;
use std::time::Duration;

mod behaviour;
mod config;
pub mod error;
mod handler;

pub use behaviour::Behaviour;
pub use config::Config;
pub use error::Error;
pub use protocol::PROTOCOL_NAME;

#[derive(Debug)]
pub enum InEvent {
    // TODO
}

#[derive(Debug)]
pub enum OutEvent {
    //TODO
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "TODO";
    // pub use owlnest_prelude::utils::protocol::universal::*;
}
