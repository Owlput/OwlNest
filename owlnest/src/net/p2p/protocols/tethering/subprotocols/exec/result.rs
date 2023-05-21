use crate::net::p2p::protocols::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Operation result for sending to the peer who sends the request.
/// Won't be exposed via `tethering::OutEvent` but be sent through sender provided by the request caller after unwrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpResult {
    Messaging(messaging::OpResult, u128),
    Tethering(),
}

#[derive(Debug)]
pub enum HandleResult {
    Ok(Duration),
    Error(HandleError),
}

#[derive(Debug)]
pub enum HandleError {
    FailingSend,
    FailingRecv,
    IO(std::io::Error),
}
