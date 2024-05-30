#![feature(extract_if)]
#![feature(hash_extract_if)]

use error::FileSendError;
use owlnest_prelude::lib_prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, time::Duration};
use tokio::sync::oneshot;
use tracing::{error, trace};

mod behaviour;
pub mod config;
pub mod error;
mod handler;
mod op;
mod protocol;

pub use behaviour::Behaviour;
pub use behaviour::{RecvInfo, SendInfo};
pub use config::Config;
pub use protocol::PROTOCOL_NAME;

/// Events that this behaviour accepts.
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
        callback: oneshot::Sender<Result<u64, FileSendError>>,
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
    /// List all peers that are connected and support this protocol.
    ListConnected(oneshot::Sender<Box<[PeerId]>>),
    /// List all recv activities, including pending and ongoing.
    ListRecv(oneshot::Sender<Box<[RecvInfo]>>),
    /// List all send activities, including pending and ongoing.
    ListSend(oneshot::Sender<Box<[SendInfo]>>),
    /// Cancel a recv operation associated with the given ID.
    /// No more bytes will be written upon seen by the behaviour
    CancelRecv {
        local_recv_id: u64,
        callback: oneshot::Sender<Result<(), ()>>,
    },
    /// Cancel a send operation associated with the give ID.
    /// No more bytes will be read upon seen by the behaviour
    CancelSend {
        local_send_id: u64,
        callback: oneshot::Sender<Result<(), ()>>,
    },
}

#[derive(Debug)]
/// Events that can be emitted by this behaviour.
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
    /// Remote has sent us a chunk of file and has been written.
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
    /// The send operation associsted with the ID is cancelled.
    CancelledSend(u64),
    /// The recv operation associated with the ID is cancelled.
    CancelledRecv(u64),
    Error(error::Error),
    /// An inbound stream has been successfully negotiated.
    InboundNegotiated(PeerId),
    /// An outbound stream has been successfully negotiated.
    /// The peer will be added to the connected peer list
    /// if an outbound stream is successfully negotiated.
    OutboundNegotiated(PeerId),
    /// The peer doesn't support this protocol.
    Unsupported(PeerId),
}
