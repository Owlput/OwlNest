#![feature(byte_slice_trim_ascii)]
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

#[cfg(feature = "disabled")]
#[cfg(test)]
mod test {
    use super::*;
    #[allow(unused)]
    use crate::net::p2p::{
        setup_default, setup_logging,
        swarm::{behaviour::BehaviourEvent, Manager, SwarmEvent},
    };
    use serial_test::serial;
    use std::{fs, io::Read, str::FromStr, thread};
    const SOURCE_FILE: &str = "../Cargo.lock";
    const DEST_FILE: &str = "../test_lock_file";

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
    fn cancel_single_send() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let _ = &peer2_m
            .executor()
            .block_on(peer2_m.blob_transfer().list_pending_recv())[0];
        peer1_m
            .executor()
            .block_on(peer1_m.blob_transfer().cancel_send(0))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.blob_transfer().list_pending_recv())
                .len()
                == 0
        )
    }

    #[test]
    fn cancel_single_send_in_multiple() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let _ = peer2_m
            .executor()
            .block_on(peer2_m.blob_transfer().list_pending_recv())[2];
        peer1_m
            .executor()
            .block_on(peer1_m.blob_transfer().cancel_send(2))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.blob_transfer().list_pending_recv())
                .len()
                == 3
        );
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.blob_transfer().list_pending_recv())
                .extract_if(|v| v.local_recv_id == 2)
                .count()
                == 0
        ); // Check if the recv_id increments linearly
    }

    #[test]
    fn cancel_single_recv() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let recv_id = peer2_m
            .executor()
            .block_on(peer2_m.blob_transfer().list_pending_recv())[0]
            .local_recv_id;
        peer2_m
            .executor()
            .block_on(peer2_m.blob_transfer().cancel_recv(recv_id))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer1_m
                .executor()
                .block_on(peer1_m.blob_transfer().list_pending_send())
                .len()
                == 0
        )
    }

    #[test]
    fn cancel_single_recv_in_multiple() {
        let (peer1_m, peer2_m) = setup_peer();
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        send(&peer1_m, peer2_m.identity().get_peer_id(), SOURCE_FILE);
        let _ = peer1_m
            .executor()
            .block_on(peer1_m.blob_transfer().list_pending_send())[2];
        peer2_m
            .executor()
            .block_on(peer2_m.blob_transfer().cancel_recv(2))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer1_m
                .executor()
                .block_on(peer1_m.blob_transfer().list_pending_send())
                .len()
                == 3
        );
        assert!(
            peer1_m
                .executor()
                .block_on(peer1_m.blob_transfer().list_pending_send())
                .extract_if(|v| v.local_send_id == 2)
                .count()
                == 0
        ); // Check if the send_id increments linearly
    }

    fn setup_peer() -> (Manager, Manager) {
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        // setup_logging();
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
            .block_on(manager.blob_transfer().send_file(to, file))
            .unwrap();
        thread::sleep(Duration::from_millis(200));
    }

    fn wait_recv(manager: &Manager, recv_id: u64, file: &str) {
        let manager_clone = manager.clone();
        let handle = manager.executor().spawn(async move {
            let mut listener = manager_clone.event_subscriber().subscribe();
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
        manager
            .executor()
            .block_on(manager.blob_transfer().recv_file(recv_id, file))
            .unwrap();
        manager.executor().block_on(handle).unwrap();
    }

    fn send_recv(peer1: &Manager, peer2: &Manager) {
        send(&peer1, peer2.identity().get_peer_id(), SOURCE_FILE);
        assert_eq!(
            peer1
                .executor()
                .block_on(peer1.blob_transfer().list_pending_send())
                .len(),
            1
        );
        wait_recv(
            &peer2,
            peer2
                .executor()
                .block_on(peer2.blob_transfer().list_pending_recv())[0]
                .local_recv_id,
            DEST_FILE,
        );
        assert!(verify_file(SOURCE_FILE, DEST_FILE));
    }

    /// Verify and clean up
    fn verify_file(left: &str, right: &str) -> bool {
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
        fs::remove_file(DEST_FILE).unwrap();
        let right_file_hash = xxhash_rust::xxh3::xxh3_128(&right_file_buf);
        drop(right_file_buf);
        left_file_hash == right_file_hash
    }
}
