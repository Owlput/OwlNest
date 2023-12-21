use crate::net::p2p::swarm::EventSender;
use either::Either;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tracing::{error, info, trace, warn};

mod behaviour;
mod config;
mod error;
mod handler;
mod op;
mod protocol;

pub use behaviour::Behaviour;
pub use config::Config;
pub use error::Error;
pub use protocol::PROTOCOL_NAME;

pub enum InEvent {
    SendFile {
        file: File,
        file_name: String,
        to: PeerId,
        send_id: u64,
        callback: oneshot::Sender<Result<Duration, FileSendReqError>>,
    },
    AcceptFile {
        file_or_folder: Either<File, PathBuf>,
        recv_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    },
}

#[derive(Debug)]
pub enum OutEvent {
    IncomingFile {
        from: PeerId,
        file_name: String,
        local_recv_id: u64,
    },
    RecvProgressed {
        local_recv_id: u64,
        finished: bool,
    },
    OngoingRecvError {
        local_recv_id: u64,
        error: String,
    },
    SendProgressed {
        local_send_id: u64,
        rtt: Option<Duration>,
    },
    ReqSendResult(oneshot::Sender<Result<Duration, SendError>>),
    Cancelled(u64),
    Error(Error),
    InboundNegotiated(PeerId),
    OutboundNegotiated(PeerId),
    Unsupported(PeerId),
}

use tokio::sync::{mpsc, oneshot};

use crate::with_timeout;
#[allow(unused_macros)]
macro_rules! event_op {
    ($listener:ident,$pattern:pat,{$($ops:tt)+}) => {
        async move{
        loop{
            let ev = crate::handle_listener_result!($listener);
            if let SwarmEvent::Behaviour(BehaviourEvent::Messaging($pattern)) = ev.as_ref() {
                $($ops)+
            } else {
                continue;
            }
        }}
    };
}
use self::error::{FileRecvError, FileSendReqError, SendError};
use std::sync::atomic::{AtomicU64, Ordering};
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
    ) -> Result<Duration, FileSendReqError> {
        if path.as_ref().is_dir() {
            // Reject sending directory
            return Err(FileSendReqError::IsDirectory);
        }
        // Get the handle to the file(locking)
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open(path.as_ref())
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSendReqError::FileNotFound,
                std::io::ErrorKind::PermissionDenied => FileSendReqError::PermissionDenied,
                e => FileSendReqError::OtherFsError(e),
            })?;
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::SendFile {
            file,
            file_name: path
                .as_ref()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            to,
            send_id: self.next_id(),
            callback: tx,
        };
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(rx, 10) {
            Ok(v) => v.expect("callback to succeed"),
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(FileSendReqError::Timeout)
            }
        }
    }
    pub async fn recv_file(
        &self,
        recv_id: u64,
        path_to_write: impl AsRef<Path>,
    ) -> Result<Duration, FileRecvError> {
        let path_to_write = path_to_write.as_ref();
        let folder_or_file = match fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path_to_write)
        {
            Ok(file) => Either::Left(file),
            Err(err) => {
                error!("{:?}", err);
                match err.kind() {
                    std::io::ErrorKind::IsADirectory => Either::Right(path_to_write.to_owned()),
                    e => return Err(FileRecvError::FsError(e)),
                }
            }
        };

        let (tx, rx) = oneshot::channel();
        let ev = InEvent::AcceptFile {
            file_or_folder: folder_or_file,
            recv_id,
            callback: tx,
        };
        info!("recv check passed");
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(rx, 10) {
            Ok(rtt) => rtt.expect("callback to succeed"),
            Err(_)=>{
                warn!("timeout reached for a timed future");
                Err(FileRecvError::Timeout)
            }
        }
    }
    fn next_id(&self) -> u64 {
        self.send_counter.fetch_add(1, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod test {
    use std::{str::FromStr, thread, time::Duration, fs, io::Read};

    use libp2p::Multiaddr;

    use crate::net::p2p::{
        setup_default, setup_logging,
        swarm::{behaviour::BehaviourEvent, SwarmEvent},
    };

    #[test]
    fn test_file() {
        const SOURCE_FILE:&str = "../Cargo.lock";
        const DEST_FILE: &str = "../test_lock_file";
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        setup_logging();
        peer1_m
            .executor()
            .block_on(
                peer1_m
                    .swarm()
                    .listen(&Multiaddr::from_str("/ip4/127.0.0.1/tcp/0").unwrap()),
            )
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        let peer1_listen = &peer1_m.swarm().list_listeners_blocking()[0];
        thread::sleep(Duration::from_millis(100));
        peer2_m.swarm().dial_blocking(peer1_listen).unwrap();
        thread::sleep(Duration::from_millis(200));
        peer1_m
            .executor()
            .block_on(peer1_m.blob_transfer().send_file(
                peer2_m.identity().get_peer_id(),
                SOURCE_FILE,
            ))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        let peer2_m_clone = peer2_m.clone();
        let handle = peer2_m.executor().spawn(async move {
            let mut listener = peer2_m_clone.event_subscriber().subscribe();
            while let Ok(ev) = listener.recv().await {
                if let SwarmEvent::Behaviour(BehaviourEvent::BlobTransfer(
                    super::OutEvent::RecvProgressed { finished, .. },
                )) = ev.as_ref()
                {
                    if *finished {
                        return;
                    }
                }
            }
        });
        peer2_m
            .executor()
            .block_on(
                peer2_m
                    .blob_transfer()
                    .recv_file(0, DEST_FILE),
            )
            .unwrap();
        peer2_m.executor().block_on(handle).unwrap();
        let mut source_file_buf = Vec::new();
        fs::OpenOptions::new().read(true).open(SOURCE_FILE).unwrap().read_to_end(&mut source_file_buf).unwrap();
        let source_file_hash = xxhash_rust::xxh3::xxh3_128(&source_file_buf);
        drop(source_file_buf);
        let mut dest_file_buf = Vec::new();
        fs::OpenOptions::new().read(true).write(true).open(SOURCE_FILE).unwrap().read_to_end(&mut dest_file_buf).unwrap();
        fs::remove_file(DEST_FILE).unwrap();
        let dest_file_hash = xxhash_rust::xxh3::xxh3_128(&dest_file_buf);
        drop(dest_file_buf);
        assert_eq!(source_file_hash, dest_file_hash)
    }
}
