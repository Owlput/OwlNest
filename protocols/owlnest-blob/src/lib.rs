#![feature(byte_slice_trim_ascii)]
#![feature(extract_if)]
#![feature(hash_extract_if)]

use owlnest_prelude::lib_prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, time::Duration};
use tokio::sync::oneshot;
use tracing::{error, trace};

mod behaviour;
mod config;
pub mod error;
mod handler;
mod op;
mod protocol;

pub use behaviour::Behaviour;
pub use behaviour::{RecvInfo, SendInfo};
pub use config::Config;
pub use protocol::PROTOCOL_NAME;

pub enum InEvent {
    SendFile {
        file: File,
        file_name: String,
        file_path: PathBuf,
        to: PeerId,
        local_send_id: u64,
        callback: oneshot::Sender<Result<Duration, error::FileSendError>>,
    },
    AcceptFile {
        file: File,
        recv_id: u64,
        callback: oneshot::Sender<Result<Duration, error::FileRecvError>>,
    },
    ListPendingRecv(oneshot::Sender<Vec<RecvInfo>>),
    ListPendingSend(oneshot::Sender<Vec<SendInfo>>),
    CancelRecv(u64, oneshot::Sender<Result<(), ()>>), // (local_recv_id, callback)
    CancelSend(u64, oneshot::Sender<Result<(), ()>>), // (local_send_id, callback)
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingFile {
        from: PeerId,
        file_name: String,
        local_recv_id: u64,
        bytes_total: u64,
    },
    RecvProgressed {
        local_recv_id: u64,
        finished: bool,
        bytes_received: u64,
        bytes_total: u64,
    },
    OngoingRecvError {
        local_recv_id: u64,
        error: String,
    },
    SendProgressed {
        local_send_id: u64,
        finished: bool,
        time_taken: Option<Duration>,
        bytes_sent: u64,
        bytes_total: u64,
    },
    OngoingSendError {
        local_send_id: u64,
        error: String,
    },
    CancelledSend(u64),
    CancelledRecv(u64),
    Error(error::Error),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
    Unsupported(PeerId),
}
