#![feature(byte_slice_trim_ascii)]
#![feature(extract_if)]
#![feature(hash_extract_if)]

use error::FileSendError;
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

#[derive(Debug)]
pub enum InEvent {
    /// Initate a file-send request.
    /// `Ok(())` means the message has been sent to remote.
    /// `Err(())` means the peer is not found.
    SendFile {
        file: File,
        /// Full path to the file, including file name.
        file_path: PathBuf,
        to: PeerId,
        /// A monotonic ID assigned to this request.
        /// The ID is unique during the lifetime of the app,
        /// but the order is not guaranteed.
        local_send_id: u64,
        callback: oneshot::Sender<Result<(), FileSendError>>,
    },
    /// Local acceptes a pending recv.
    AcceptFile {
        /// An empty file to write to.
        file: File,
        /// A monotonic ID of this request.
        /// The ID is unique during the lifetime of the app,
        /// but the order is not guaranteed.
        recv_id: u64,
        path: PathBuf,
        callback: oneshot::Sender<Result<Duration, error::FileRecvError>>,
    },
    ListConnected(oneshot::Sender<Vec<PeerId>>),
    ListPendingRecv(oneshot::Sender<Vec<RecvInfo>>),
    ListPendingSend(oneshot::Sender<Vec<SendInfo>>),
    CancelRecv {
        local_recv_id: u64,
        callback: oneshot::Sender<Result<(), ()>>,
    },
    CancelSend {
        local_send_id: u64,
        callback: oneshot::Sender<Result<(), ()>>,
    },
}

#[derive(Debug)]
pub enum OutEvent {
    /// A remote informed local of a pending file.
    IncomingFile {
        from: PeerId,
        file_name: String,
        /// A monotonic ID assigned to this request.
        /// The ID is unique during the lifetime of the app,
        /// but the order is not guaranteed.
        local_recv_id: u64,
        bytes_total: u64,
    },
    RecvProgressed {
        local_recv_id: u64,
        /// The amount of bytes that have received.
        bytes_received: u64,
        bytes_total: u64,
    },
    OngoingRecvError {
        local_recv_id: u64,
        error: String,
    },
    SendProgressed {
        local_send_id: u64,
        /// The amount of bytes that have sent.
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
