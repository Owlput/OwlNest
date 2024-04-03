use crate::net::p2p::swarm::EventSender;
use libp2p::PeerId;
pub use owlnest_blob::{error, Behaviour, InEvent, OutEvent};
use owlnest_blob::{RecvInfo, SendInfo};
use owlnest_macro::{generate_handler_method, with_timeout};
use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use tracing::warn;
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    send_counter: Arc<AtomicU64>,
}
impl Handle {
    pub fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
                send_counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }

    pub async fn send_file(
        &self,
        to: PeerId,
        path: impl AsRef<Path>,
    ) -> Result<(u64, Duration), error::FileSendError> {
        if path.as_ref().is_dir() {
            // Reject sending directory
            return Err(error::FileSendError::IsDirectory);
        }
        // Get the handle to the file(locking)
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open(path.as_ref())
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => error::FileSendError::FileNotFound,
                std::io::ErrorKind::PermissionDenied => error::FileSendError::PermissionDenied,
                e => error::FileSendError::OtherFsError(e),
            })?;
        let (tx, rx) = oneshot::channel();
        let local_send_id = self.next_id();
        let ev = InEvent::SendFile {
            file,
            file_name: path
                .as_ref()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            file_path: path.as_ref().to_owned(),
            to,
            local_send_id,
            callback: tx,
        };
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(rx, 10) {
            Ok(v) => v.expect("callback to succeed").map(|v| (local_send_id, v)),
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(error::FileSendError::Timeout)
            }
        }
    }
    /// Accept a pending recv.
    /// If the path provided is an existing directory, the file will be written
    /// to the directory with its original name.
    /// If the path provided is an existing file, an error will be returned.
    pub async fn recv_file(
        &self,
        recv_id: u64,
        path_to_write: impl AsRef<Path>,
    ) -> Result<Duration, error::FileRecvError> {
        let path_to_write = path_to_write.as_ref();
        let file = match fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(path_to_write)
        {
            Ok(file) => file,
            Err(err) => {
                return Err(error::FileRecvError::FsError(err.kind()));
            }
        };

        let (tx, rx) = oneshot::channel();
        let ev = InEvent::AcceptFile {
            file,
            recv_id,
            callback: tx,
        };
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(rx, 10) {
            Ok(rtt) => rtt.expect("callback to succeed"),
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(error::FileRecvError::Timeout)
            }
        }
    }
    generate_handler_method!(
        ListPendingRecv:list_pending_recv()->Vec<RecvInfo>;
        ListPendingSend:list_pending_send()->Vec<SendInfo>;
        /// Cancel a send operation on local node.
        /// Remote will be notified.
        /// Return an error if the send operation is not found.
        /// If `Ok(())` is returned, it is guaranteed that no more bytes will be sent to remote.
        CancelSend:cancel_send(local_send_id:u64)->Result<(),()>;
        /// Cancel a recv operation on local node.
        /// Remote will be notified.
        /// Return an error if the recv operation is not found.
        /// If `Ok(())` is returned, it is guaranteed that no more bytes will be written to the file.
        CancelRecv:cancel_recv(local_recv_id:u64)->Result<(),()>;
    );
    fn next_id(&self) -> u64 {
        self.send_counter.fetch_add(1, Ordering::SeqCst)
    }
}
