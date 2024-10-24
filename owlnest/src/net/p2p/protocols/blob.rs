use crate::net::p2p::swarm::EventSender;
use libp2p::PeerId;
pub use owlnest_blob::{config, error, Behaviour, InEvent, OutEvent};
pub use owlnest_blob::{RecvInfo, SendInfo};
use owlnest_macro::{generate_handler_method, with_timeout};
use std::path::Path;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::{trace, warn};
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    swarm_event_source: EventSender,
}
impl Handle {
    pub fn new(buffer: usize, swarm_event_source: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                swarm_event_source: swarm_event_source.clone(),
            },
            rx,
        )
    }

    pub async fn send_file(
        &self,
        to: PeerId,
        path: impl AsRef<Path>,
    ) -> Result<u64, error::FileSendError> {
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
        let ev = InEvent::SendFile {
            file,
            file_path: path.as_ref().to_owned(),
            to,
            callback: tx,
        };
        self.sender.send(ev).await.expect("send to succeed");
        match with_timeout!(rx, 10) {
            Ok(v) => v.expect("callback to succeed"),
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
        trace!("Accepting recv id {}", recv_id);
        let path_to_write = path_to_write.as_ref();
        let file = match std::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(path_to_write)
        {
            Ok(file) => file,
            Err(err) => {
                return Err(error::FileRecvError::FsError {
                    path: path_to_write.to_string_lossy().to_string(),
                    error: err.kind(),
                });
            }
        };

        let (tx, rx) = oneshot::channel();
        let ev = InEvent::AcceptFile {
            file,
            recv_id,
            callback: tx,
            path: path_to_write.into(),
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
    /// Cancel a send operation on local node.
    /// Remote will be notified.
    /// Return an error if the send operation is not found.
    /// If `Ok(())` is returned, it is guaranteed that no more bytes will be sent to remote.
    pub async fn cancel_send(&self, local_send_id: u64) -> Result<(), ()> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::CancelSend {
            local_send_id,
            callback: tx,
        };
        self.sender.send(ev).await.expect("Send to succeed");
        match with_timeout!(rx, 10) {
            Ok(result) => result.expect("callback to succeed"),
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(())
            }
        }
    }
    /// Cancel a recv operation on local node.
    /// Remote will be notified.
    /// Return an error if the recv operation is not found.
    /// If `Ok(())` is returned, it is guaranteed that no more bytes will be written to the file.
    pub async fn cancel_recv(&self, local_recv_id: u64) -> Result<(), ()> {
        let (tx, rx) = oneshot::channel();
        let ev = InEvent::CancelRecv {
            local_recv_id,
            callback: tx,
        };
        self.sender.send(ev).await.expect("Send to succeed");
        match with_timeout!(rx, 10) {
            Ok(result) => result.expect("callback to succeed"),
            Err(_) => {
                warn!("timeout reached for a timed future");
                Err(())
            }
        }
    }
    generate_handler_method!(
        /// List receives that are still in pending phase.
        /// Ongoing receives should be tracked by the user interface.
        ListRecv:list_pending_recv()->Box<[RecvInfo]>;
        /// List sends that are still in pending phase.
        /// Ongoing sends should be tracked by the user interface.
        ListSend:list_pending_send()->Box<[SendInfo]>;
        /// List all peers that have successfully negotiated this protocol.
        ListConnected:list_connected()->Box<[PeerId]>;
    );
}

pub(crate) mod cli {
    use super::Handle;
    use clap::Subcommand;
    use prettytable::table;
    use printable::iter::PrintableIter;

    #[derive(Debug, Subcommand)]
    pub enum Blob {
        /// Send a file to remote. Does not take folders or multiple files.
        #[command(arg_required_else_help = true)]
        Send {
            /// Peer to send the file to.
            #[arg(required = true)]
            remote: libp2p::PeerId,
            /// Path to the file.
            #[arg(required = true)]
            file_path: String,
        },
        /// List all send operation, pending and ongoing.
        ListSend,
        /// List all recv operation, pending or ongoing.
        ListRecv,
        /// Accept a send request from remote.
        #[command(arg_required_else_help = true)]
        Recv {
            /// Recieve ID associated with the receive request.
            #[arg(required = true)]
            local_recv_id: u64,
            /// Path to write the file to.
            /// If supplied with a folder, a file with its original name is created,
            /// fail if cannot be created.
            /// If supplied with a file, the content will be written to that file
            /// without using the original name, fail if already exists(no overwrite).
            #[arg(default_value = ".")]
            path_to_write: String,
        },
        /// Cancel a pending or ongoing send operation.
        #[command(arg_required_else_help = true)]
        CancelSend {
            /// Send ID associated with the receive request.
            #[arg(required = true)]
            local_send_id: u64,
        },
        /// Cancel a pending or ongoing receive operation.
        #[command(arg_required_else_help = true)]
        CancelRecv {
            /// Recieve ID associated with the receive request.
            #[arg(required = true)]
            local_recv_id: u64,
        },
    }

    pub async fn handle_blob(handle: &Handle, command: Blob) {
        use Blob::*;
        match command {
            ListSend => {
                let list = handle.list_pending_send().await;
                let print_pending = list
                    .iter()
                    .filter(|v| !v.started)
                    .printable()
                    .with_left_bound("")
                    .with_right_bound("")
                    .with_separator("\n");
                let print_started = list
                    .iter()
                    .filter(|v| !v.started)
                    .printable()
                    .with_left_bound("")
                    .with_right_bound("")
                    .with_separator("\n");
                let table = table!(
                    ["Pending Send", "Ongoing Send"],
                    [print_pending, print_started]
                );
                table.printstd()
            }
            Send { remote, file_path } => {
                let result = handle.send_file(remote, file_path).await;
                match result {
                    Ok(id) => println!("Send initated with ID {}", id),
                    Err(e) => println!("Send failed with error {:?}", e),
                }
            }
            Recv {
                local_recv_id,
                path_to_write,
            } => {
                let result = handle.recv_file(local_recv_id, path_to_write).await;
                match result {
                    Ok(_rtt) => println!("Recv ID {} accepted", local_recv_id),
                    Err(e) => println!("Send failed with error {}", e),
                }
            }
            _ => todo!(),
        }
    }
    pub mod send {
        use clap::Parser;
        use libp2p::PeerId;

        #[derive(Parser, Debug)]
        struct Args {
            #[arg(long)]
            peer: PeerId,
            source: String,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::net::p2p::swarm::{behaviour::BehaviourEvent, Manager, SwarmEvent};
    #[allow(unused)]
    use crate::net::p2p::test_suit::setup_default;
    use libp2p::Multiaddr;
    use serial_test::serial;
    use std::{io::Read, str::FromStr, thread};
    use temp_dir::TempDir;
    const SOURCE_FILE: &str = "../Cargo.lock";

    #[test]
    #[serial]
    fn single_send_recv() {
        let (peer1_m, peer2_m) = setup_peer();
        send_recv(&peer1_m, &peer2_m)
    }
    #[test]
    #[serial]
    fn multi_send_recv() {
        let (peer1_m, peer2_m) = setup_peer();
        send_recv(&peer1_m, &peer2_m);
        send_recv(&peer1_m, &peer2_m);
        send_recv(&peer2_m, &peer1_m);
        send_recv(&peer1_m, &peer2_m);
        send_recv(&peer2_m, &peer1_m);
    }

    #[test]
    #[serial]
    fn cancel_single_send() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let _ = &peer2_m
            .executor()
            .block_on(peer2_m.blob().list_pending_recv())[0];
        peer1_m
            .executor()
            .block_on(peer1_m.blob().cancel_send(0))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.blob().list_pending_recv())
                .len()
                == 0
        )
    }

    #[test]
    #[serial]
    fn cancel_single_send_in_multiple() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let _ = peer2_m
            .executor()
            .block_on(peer2_m.blob().list_pending_recv())[2];
        peer1_m
            .executor()
            .block_on(peer1_m.blob().cancel_send(2))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.blob().list_pending_recv())
                .len()
                == 3
        );
        assert!(peer2_m
            .executor()
            .block_on(peer2_m.blob().list_pending_recv())
            .iter()
            .find(|v| v.local_recv_id == 2)
            .is_none()); // Check if the recv_id increments linearly
    }

    #[test]
    #[serial]
    fn cancel_single_recv() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let recv_id = peer2_m
            .executor()
            .block_on(peer2_m.blob().list_pending_recv())[0]
            .local_recv_id;
        peer2_m
            .executor()
            .block_on(peer2_m.blob().cancel_recv(recv_id))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer1_m
                .executor()
                .block_on(peer1_m.blob().list_pending_send())
                .len()
                == 0
        )
    }

    #[test]
    #[serial]
    fn cancel_single_recv_in_multiple() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let _ = peer1_m
            .executor()
            .block_on(peer1_m.blob().list_pending_send())[2];
        peer2_m
            .executor()
            .block_on(peer2_m.blob().cancel_recv(2))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer1_m
                .executor()
                .block_on(peer1_m.blob().list_pending_send())
                .len()
                == 3
        );
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.blob().list_pending_send())
            .iter()
            .find(|v| v.local_send_id == 2)
            .is_none()); // Check if the send_id increments linearly
    }

    fn setup_peer() -> (Manager, Manager) {
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        peer1_m
            .executor()
            .block_on(
                peer1_m
                    .swarm()
                    .listen(&Multiaddr::from_str("/ip4/127.0.0.1/tcp/0").unwrap()),
            )
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        let peer1_listen = &peer1_m.swarm().list_listeners_blocking()[0];
        thread::sleep(Duration::from_millis(100));
        peer2_m.swarm().dial_blocking(peer1_listen).unwrap();
        thread::sleep(Duration::from_millis(200));
        (peer1_m, peer2_m)
    }

    /// Send and sleep for a short while to sync state
    fn send(manager: &Manager, to: PeerId, file: &str) {
        manager
            .executor()
            .block_on(manager.blob().send_file(to, file))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
    }

    fn wait_recv(manager: &Manager, recv_id: u64, dir: &TempDir) {
        let manager_clone = manager.clone();
        let handle = manager.executor().spawn(async move {
            let mut listener = manager_clone.event_subscriber().subscribe();
            while let Ok(ev) = listener.recv().await {
                if let SwarmEvent::Behaviour(BehaviourEvent::Blob(OutEvent::RecvProgressed {
                    bytes_received,
                    bytes_total,
                    ..
                })) = ev.as_ref()
                {
                    if bytes_received == bytes_total {
                        return;
                    }
                }
            }
        });
        manager
            .executor()
            .block_on(
                manager
                    .blob()
                    .recv_file(recv_id, dir.path().join("test_locker_file")),
            )
            .unwrap();
        manager.executor().block_on(handle).unwrap();
    }

    fn send_recv(peer1: &Manager, peer2: &Manager) {
        let dest = TempDir::new().unwrap();
        send(&peer1, peer2.identity().get_peer_id(), SOURCE_FILE);
        assert_eq!(
            peer1
                .executor()
                .block_on(peer1.blob().list_pending_send())
                .len(),
            1
        );
        thread::sleep(Duration::from_millis(100));
        wait_recv(
            &peer2,
            peer2.executor().block_on(peer2.blob().list_pending_recv())[0].local_recv_id,
            &dest,
        );
        assert!(verify_file(
            SOURCE_FILE,
            dest.path().join("test_locker_file")
        ));
    }

    /// Verify and clean up
    fn verify_file(left: impl AsRef<Path>, right: impl AsRef<Path>) -> bool {
        use std::fs;
        let mut left_file_buf = Vec::new();
        fs::OpenOptions::new()
            .read(true)
            .open(left)
            .unwrap()
            .read_to_end(&mut left_file_buf)
            .unwrap();
        let left_file_hash = xxhash_rust::xxh3::xxh3_128(&left_file_buf);
        drop(left_file_buf);
        let mut right_file_buf = Vec::new();
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(right)
            .unwrap()
            .read_to_end(&mut right_file_buf)
            .unwrap();
        let right_file_hash = xxhash_rust::xxh3::xxh3_128(&right_file_buf);
        drop(right_file_buf);
        left_file_hash == right_file_hash
    }
    // Attach when necessary
    #[allow(unused)]
    fn setup_logging() {
        use crate::net::p2p::protocols::SUBSCRIBER_CONFLICT_ERROR_MESSAGE;
        use std::sync::Mutex;
        use tracing::Level;
        use tracing_log::LogTracer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Layer;
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("owlnest", Level::INFO)
            .with_target("owlnest_blob", Level::DEBUG)
            .with_target("", Level::WARN);
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_ansi(false)
            .with_writer(Mutex::new(std::io::stdout()))
            .with_filter(filter);
        let reg = tracing_subscriber::registry().with(layer);
        tracing::subscriber::set_global_default(reg).expect(SUBSCRIBER_CONFLICT_ERROR_MESSAGE);
        LogTracer::init().unwrap();
    }
}
