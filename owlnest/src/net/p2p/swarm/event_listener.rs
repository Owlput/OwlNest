use std::collections::HashMap;
use super::SwarmEvent;
use futures::channel::oneshot;
use tokio::{select, sync::mpsc};

type Error = crate::event_bus::Error;

#[derive(Debug)]
pub enum Op {
    Add(
        Kind,
        mpsc::Sender<SwarmEvent>,
        oneshot::Sender<Result<u64, Error>>,
    ),
    Remove(Kind, u64, oneshot::Sender<Result<(), Error>>),
}

#[repr(u32)]
#[derive(Debug)]
pub enum Kind {
    OnConnectionEstablished = 0,
    OnIncomingConnection = 1,
    OnIncomingConnectionError = 2,
    OnOutgoingConnectionError = 3,
    OnNewListenAddr = 5,
    OnExpiredListenAddr = 6,
    OnListenerClosed = 7,
    OnListenerError = 8,
    OnDialing = 9,
}

