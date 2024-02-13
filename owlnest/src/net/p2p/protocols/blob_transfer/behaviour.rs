use crate::handle_callback_sender;
use crate::net::p2p::protocols::blob_transfer::handler::FromBehaviourEvent;

use super::*;
use libp2p::swarm::{ConnectionId, NetworkBehaviour, NotifyHandler, ToSwarm};
use libp2p::PeerId;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::{collections::VecDeque, task::Poll};
use tracing::info;

pub(crate) const FILE_CHUNK_SIZE: usize = 65536;

pub struct Behaviour {
    config: Config,
    /// Pending events to emit to `Swarm`
    out_events: VecDeque<OutEvent>,
    /// Pending events to be processed by this `Behaviour`.
    pending_handler_event: VecDeque<ToSwarm<OutEvent, FromBehaviourEvent>>,
    /// A set for all connected peers.
    connected_peers: HashSet<PeerId>,
    recv_counter: u64,
    /// List of pending receive indexed by recv ID.
    pending_recv: HashMap<u64, RecvInfo>,
    /// List of pending send indexed by send ID.
    pending_send: HashMap<u64, SendInfo>,
    /// Ongoing receive indexed by remote send id.
    /// If the record is removed from the list, no more bytes will be written to the file.
    ongoing_recv: HashMap<u64, OngoingFileRecv>,
    /// Unindexed list of ongoing send.
    /// The bandwith will be shared evenly between all process.
    /// If the record is removed from the list, no more bytes will be send to remote.
    ongoing_send: VecDeque<OngoingFileSend>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecvInfo {
    pub local_recv_id: u64,
    pub remote_send_id: u64,
    pub file_name: String,
    pub remote: PeerId,
}
#[derive(Debug, Clone, Serialize)]
pub struct SendInfo {
    pub local_send_id: u64,
    pub remote: PeerId,
    pub file_path: PathBuf,
    pub file_name: String,
}

impl Behaviour {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            out_events: VecDeque::new(),
            pending_handler_event: VecDeque::new(),
            connected_peers: HashSet::new(),
            recv_counter: 0,
            pending_recv: HashMap::new(),
            ongoing_recv: HashMap::new(),
            pending_send: HashMap::new(),
            ongoing_send: VecDeque::new(),
        }
    }
    pub fn push_event(&mut self, ev: InEvent) {
        use InEvent::*;
        match ev {
            SendFile {
                file_path,
                file_name,
                to,
                local_send_id,
                callback,
            } => {
                if !self.connected_peers.contains(&to) {
                    // Return error when the peer is not connected
                    callback
                        .send(Err(FileSendError::PeerNotFound))
                        .expect("callback to succeed");
                    return;
                }
                // Queue the send
                self.pending_send.insert(
                    local_send_id,
                    SendInfo {
                        local_send_id,
                        remote: to,
                        file_path,
                        file_name: file_name.clone(),
                    },
                );
                // Notify the remote for new send
                self.pending_handler_event
                    .push_back(ToSwarm::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourEvent::NewFileSend {
                            file_name,
                            local_send_id,
                            callback,
                        },
                    });
            }
            AcceptFile {
                file_or_folder,
                recv_id,
                callback,
            } => self.accept_pending_recv(file_or_folder, recv_id, callback),
            CancelRecv(recv_id, callback) => {
                if self.cancel_recv_by_local_recv_id(recv_id) {
                    handle_callback_sender!(Ok(())=>callback);
                    return;
                }
                handle_callback_sender!(Err(())=>callback);
            }
            CancelSend(local_send_id, callback) => {
                if self.cancel_send_by_local_send_id(local_send_id) {
                    handle_callback_sender!(Ok(())=>callback);
                    return;
                }
                handle_callback_sender!(Err(())=>callback);
            }
            ListPendingRecv(callback) => {
                handle_callback_sender!(self.pending_recv.values().cloned().collect()=>callback)
            }
            ListPendingSend(callback) => {
                handle_callback_sender!(self.pending_send.values().cloned().collect()=>callback)
            }
        }
    }
    fn new_pending_recv(&mut self, from: PeerId, file_name: String, remote_send_id: u64) {
        let local_recv_id = self.next_recv_id();
        self.pending_recv.insert(
            local_recv_id,
            RecvInfo {
                local_recv_id,
                remote_send_id,
                remote: from,
                file_name: file_name.clone(),
            },
        );
        self.out_events.push_back(OutEvent::IncomingFile {
            file_name,
            from,
            local_recv_id,
        });
    }
    fn accept_pending_recv(
        &mut self,
        file_or_folder: Either<File, PathBuf>,
        recv_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    ) {
        let RecvInfo {
            remote_send_id,
            remote,
            file_name,
            ..
        } = match self.pending_recv.remove(&recv_id) {
            Some(v) => v,
            None => {
                // Not found in pending recv, report the error directly to caller using callback
                handle_callback_sender!( Err(FileRecvError::PendingRecvNotFound) => callback);
                return;
            }
        };
        let file = match file_or_folder {
            Either::Left(file) => file,
            Either::Right(mut folder) => {
                if let Err(e) = fs::create_dir_all(&folder) {
                    handle_callback_sender!(Err(FileRecvError::FsError(e.kind()))=>callback);
                    return;
                }
                folder.push(file_name);
                let file = folder;
                match fs::OpenOptions::new().create(true).write(true).open(file) {
                    Ok(file) => file,
                    Err(e) => {
                        handle_callback_sender!(Err(FileRecvError::FsError(e.kind()))=>callback);
                        return;
                    }
                }
            }
        };
        self.ongoing_recv.insert(
            remote_send_id,
            OngoingFileRecv {
                remote_send_id,
                local_recv_id: recv_id,
                file_handle: file,
                chunks_received: 0,
                remote,
            },
        );
        let ev = ToSwarm::NotifyHandler {
            peer_id: remote,
            handler: NotifyHandler::Any,
            event: FromBehaviourEvent::AcceptFile {
                remote_send_id,
                callback,
            },
        };
        self.pending_handler_event.push_back(ev);
    }

    /// Accept a pending send. If not found in the pending list, will return `Result::Err`.
    fn pending_send_accepted(&mut self, local_send_id: u64) {
        if let None = self.pending_send.get(&local_send_id) {
            self.cancel_send_by_local_send_id(local_send_id);
            return;
        }
        let SendInfo {
            local_send_id,
            remote,
            file_path,
            ..
        } = self.pending_send.remove(&local_send_id).unwrap();
        let file_handle = match std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open(&file_path)
        {
            Ok(handle) => handle,
            Err(e) => {
                // let _ = match e.kind() {
                //     std::io::ErrorKind::NotFound => FileSendError::FileNotFound,
                //     std::io::ErrorKind::PermissionDenied => FileSendError::PermissionDenied,
                //     e => FileSendError::OtherFsError(e),
                // };
                self.out_events.push_back(OutEvent::OngoingSendError {
                    local_send_id,
                    error: e.to_string(),
                });
                self.cancel_send_by_local_send_id(local_send_id);
                return;
            }
        };
        self.ongoing_send.push_back(OngoingFileSend {
            local_send_id,
            remote,
            file_handle,
        });
    }
    /// Called when local node cancel transmission.
    /// Once called, it is guaranteed that no more bytes will be written to the file.
    fn cancel_recv_by_local_recv_id(&mut self, local_recv_id: u64) -> bool {
        // Try to remove all recv record associted with the provided id.
        // If none is found, false is returned.
        if let Some((remote, remote_send_id)) = self.remove_recv_record(local_recv_id) {
            // Notify the remote about the cancellation.
            // Receiving operation on local node will stop immediately without acknowledgement from remote.
            self.pending_handler_event
                .push_back(ToSwarm::NotifyHandler {
                    peer_id: remote,
                    handler: NotifyHandler::Any,
                    event: FromBehaviourEvent::CancelRecv { remote_send_id },
                });
            return true;
        }
        false
    }
    /// Called when local cancelled transmission.
    /// Once called, it is guaranteed that no more bytes will be read.
    fn cancel_send_by_local_send_id(&mut self, local_send_id: u64) -> bool {
        if let Some((remote, _)) = self.remove_send_record(local_send_id) {
            // Notify remote about the cancellation.
            self.pending_handler_event
                .push_back(ToSwarm::NotifyHandler {
                    peer_id: remote,
                    handler: NotifyHandler::Any,
                    event: FromBehaviourEvent::CancelSend { local_send_id },
                });
            return true;
        };
        false
    }
    /// Called when remote cancelled its sending.
    /// Once called, it is guaranteed that no more bytes will be written to the file.
    fn remove_recv_by_remote_send_id(&mut self, remote_send_id: u64) -> Option<(PeerId, u64)> {
        if let Some(v) = self.ongoing_recv.remove(&remote_send_id) {
            return Some((v.remote, v.local_recv_id));
        };
        if let Some((_, v)) = self
            .pending_recv
            .extract_if(|_, v| v.remote_send_id == remote_send_id)
            .next()
        {
            return Some((v.remote, v.remote_send_id));
        };
        None
    }
    fn handle_ongoing_send(&mut self) -> Option<(PeerId, FromBehaviourEvent)> {
        if self.ongoing_send.len() == 0 {
            return None;
        }
        let mut ongoing_send = self.ongoing_send.pop_front().unwrap();
        let mut buf = [0u8; FILE_CHUNK_SIZE];
        let bytes_read = match ongoing_send.file_handle.read(&mut buf) {
            Err(e) => {
                error!("{:?}", e);
                todo!()
            }
            Ok(bytes_read) => bytes_read,
        };

        let mut vec: Vec<u8> = buf.into();
        if bytes_read < FILE_CHUNK_SIZE {
            vec.truncate(bytes_read)
        }
        let v = (
            ongoing_send.remote,
            FromBehaviourEvent::File {
                contents: vec,
                local_send_id: ongoing_send.local_send_id,
            },
        );
        if bytes_read != 0 {
            self.ongoing_send.push_back(ongoing_send);
        }
        Some(v)
    }
    fn remove_recv_record(&mut self, local_recv_id: u64) -> Option<(PeerId, u64)> {
        if let Some(v) = self.pending_recv.remove(&local_recv_id) {
            return Some((v.remote, v.remote_send_id));
        }
        if let Some(v) = self.ongoing_recv.remove(&local_recv_id) {
            return Some((v.remote, v.remote_send_id));
        };
        None
    }
    /// Called when remote cancelled its receiving.
    /// Once called, it is guaranteed that no more bytes will be sent to remote.
    fn remove_send_record(&mut self, local_send_id: u64) -> Option<(PeerId, u64)> {
        if let Some(SendInfo {
            local_send_id,
            remote,
            ..
        }) = self.pending_send.remove(&local_send_id)
        {
            return Some((remote, local_send_id));
        };
        let mut found = None;
        self.ongoing_send.retain(|v| {
            if v.local_send_id == local_send_id {
                found = Some((v.remote, v.local_send_id));
                return false;
            }
            true
        });
        found
    }
    fn next_recv_id(&mut self) -> u64 {
        let id = self.recv_counter;
        self.recv_counter += 1;
        id
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = handler::Handler;
    type ToSwarm = OutEvent;

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::ToBehaviour,
    ) {
        use handler::ToBehaviourEvent::*;
        match event {
            RecvProgressed {
                remote_send_id,
                contents,
            } => {
                let mut ongoing_recv = if let Some(v) = self.ongoing_recv.remove(&remote_send_id) {
                    v
                } else {
                    return;
                };
                if contents.len() == 0 {
                    self.out_events.push_back(OutEvent::RecvProgressed {
                        local_recv_id: ongoing_recv.local_recv_id,
                        finished: true,
                    });
                    if let Err(e) = ongoing_recv.file_handle.flush() {
                        tracing::error!("Unsuccessful file flushing: {:?}", e)
                    };
                    return;
                }
                match ongoing_recv.file_handle.write(&contents) {
                    Ok(len) => {
                        if len != contents.len() {
                            tracing::error!("Partially written file!")
                        }
                    }
                    Err(e) => {
                        let ev = OutEvent::OngoingRecvError {
                            local_recv_id: ongoing_recv.local_recv_id,
                            error: format!("{:?}", e),
                        };
                        self.out_events.push_back(ev);
                    }
                };
                ongoing_recv.chunks_received += 1;
                if ongoing_recv.chunks_received % 16 == 0 {
                    // TODO: Better error handling
                    // Flush after 16 chunks
                    if let Err(e) = ongoing_recv.file_handle.flush() {
                        tracing::error!("Unsuccessful file flushing: {:?}", e)
                    };
                }
                self.ongoing_recv.insert(remote_send_id, ongoing_recv);
            }
            IncomingFile {
                file_name,
                remote_send_id,
            } => self.new_pending_recv(peer_id, file_name, remote_send_id),
            Error(e) => {
                info!(
                    "Error occurred on peer {}:{:?}: {:#?}",
                    peer_id, connection_id, e
                );
                self.out_events.push_back(OutEvent::Error(e));
            }
            InboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::InboundNegotiated(peer_id));
            }
            OutboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::OutboundNegotiated(peer_id));
            }
            Unsupported => {
                self.out_events.push_back(OutEvent::Unsupported(peer_id));
                trace!("Peer {} doesn't support {}", peer_id, PROTOCOL_NAME)
            }
            FileSendPending { local_send_id } => {
                trace!("Send ID {} acknowledged by remote", local_send_id)
            }
            FileSendAccepted { local_send_id } => {
                self.out_events.push_back(OutEvent::SendProgressed {
                    local_send_id,
                    time_taken: None,
                    finished: false,
                });
                self.pending_send_accepted(local_send_id)
            }
            SendProgressed { local_send_id, rtt } => {
                self.out_events.push_back(OutEvent::SendProgressed {
                    local_send_id,
                    time_taken: Some(rtt),
                    finished: false,
                })
            }
            CancelSend { local_send_id } => {
                self.out_events
                    .push_back(OutEvent::CancelledSend(local_send_id));
                self.remove_send_record(local_send_id);
            }
            CancelRecv { remote_send_id } => {
                if let Some((_, local_recv_id)) = self.remove_recv_by_remote_send_id(remote_send_id)
                {
                    self.out_events
                        .push_back(OutEvent::CancelledRecv(local_recv_id));
                }
            }
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<super::OutEvent, handler::FromBehaviourEvent>> {
        if let Some(ev) = self.out_events.pop_front() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.pending_handler_event.pop_front() {
            return Poll::Ready(ev);
        }
        if let Some((peer_id, event)) = self.handle_ongoing_send() {
            return Poll::Ready(ToSwarm::NotifyHandler {
                peer_id,
                handler: NotifyHandler::Any,
                event,
            });
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
        match &event {
            libp2p_swarm::FromSwarm::ConnectionClosed(info) => {
                if info.remaining_established < 1 {
                    self.connected_peers.remove(&info.peer_id);
                    self.pending_send.retain(|_, v| v.remote != info.peer_id);
                    self.pending_recv.retain(|_, v| v.remote != info.peer_id);
                    self.ongoing_send.retain(|v| v.remote != info.peer_id);
                    self.ongoing_recv.retain(|_, v| v.remote != info.peer_id);
                }
            }
            _ => {}
        }
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }
}

struct OngoingFileRecv {
    remote_send_id: u64,
    local_recv_id: u64,
    remote: PeerId,
    chunks_received: u64,
    file_handle: std::fs::File,
}

struct OngoingFileSend {
    local_send_id: u64,
    remote: PeerId,
    file_handle: std::fs::File,
}
