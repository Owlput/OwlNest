use super::*;
use handler::FromBehaviourEvent;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::behaviour_prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use tracing::{info, trace_span};

pub(crate) const FILE_CHUNK_SIZE: usize = 16384; // 120KB

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
    pending_recv: HashMap<u64, PendingRecv>,
    /// List of pending send indexed by send ID.
    pending_send: HashMap<u64, PendingSend>,
    /// Ongoing receive indexed by remote send id.
    /// If the record is removed from the list, no more bytes will be written to the file.
    ongoing_recv: HashMap<u64, OngoingFileRecv>,
    /// Unindexed list of ongoing send.
    /// The bandwith will be shared evenly between all process.
    /// If the record is removed from the list, no more bytes will be send to remote.
    ongoing_send: HashMap<u64, OngoingFileSend>,
}

#[derive(Debug, Clone)]
struct PendingRecv {
    local_recv_id: u64,
    remote_send_id: u64,
    bytes_total: u64,
    file_name: String,
    remote: PeerId,
    span: tracing::Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecvInfo {
    pub local_recv_id: u64,
    pub bytes_total: u64,
    pub file_name: String,
    pub remote: PeerId,
}
impl From<&PendingRecv> for RecvInfo {
    fn from(value: &PendingRecv) -> Self {
        Self {
            local_recv_id: value.local_recv_id,
            remote: value.remote,
            file_name: value.file_name.clone(),
            bytes_total: value.bytes_total,
        }
    }
}

#[derive(Debug)]
struct PendingSend {
    local_send_id: u64,
    remote: PeerId,
    file_path: PathBuf,
    bytes_total: u64,
    file: File,
    span: tracing::Span,
}

#[derive(Debug, Serialize, Clone)]
pub struct SendInfo {
    pub local_send_id: u64,
    pub remote: PeerId,
    pub file_path: PathBuf,
}
impl From<&PendingSend> for SendInfo {
    fn from(value: &PendingSend) -> Self {
        Self {
            local_send_id: value.local_send_id,
            remote: value.remote,
            file_path: value.file_path.clone(),
        }
    }
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
            ongoing_send: HashMap::new(),
        }
    }
    /// Call this to insert an event.
    /// ## Silent failure
    /// This function will return without producing error result:
    /// - File metadata cannot be retreived.
    pub fn push_event(&mut self, ev: InEvent) {
        use InEvent::*;
        match ev {
            SendFile {
                file,
                file_path,
                to,
                local_send_id,
                callback,
            } => {
                let span = trace_span!("Blob Send", id = local_send_id);
                let entered = span.enter();
                trace!("Send request spawned.");
                if !self.connected_peers.contains(&to) {
                    // Return error when the peer is not connected
                    callback
                        .send(Err(FileSendError::PeerNotFound))
                        .expect("callback to succeed");
                    trace!(
                        "Send request {} is dropped because the target is not found.",
                        local_send_id
                    );
                    return;
                }
                let bytes_total = match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(_) => {
                        trace!("Send request dropped because metadata cannot be read.");
                        return;
                    }
                };
                trace!("Send request queued.");
                drop(entered);
                // Queue the send
                self.pending_send.insert(
                    local_send_id,
                    PendingSend {
                        local_send_id,
                        remote: to,
                        file_path: file_path.clone(),
                        span,
                        bytes_total,
                        file,
                    },
                );
                // Notify the remote for new send
                self.pending_handler_event
                    .push_back(ToSwarm::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourEvent::NewFileSend {
                            file_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                            local_send_id,
                            callback,
                            bytes_total,
                        },
                    });
            }
            AcceptFile {
                file: file_or_folder,
                recv_id,
                callback,
            } => self.accept_pending_recv(file_or_folder, recv_id, callback),
            CancelRecv {
                local_recv_id: recv_id,
                callback,
            } => {
                if self.cancel_recv_by_local_recv_id(recv_id) {
                    handle_callback_sender!(Ok(())=>callback);
                    return;
                }
                handle_callback_sender!(Err(())=>callback);
            }
            CancelSend {
                local_send_id,
                callback,
            } => {
                if self.cancel_send_by_local_send_id(local_send_id) {
                    handle_callback_sender!(Ok(())=>callback);
                    return;
                }
                handle_callback_sender!(Err(())=>callback);
            }
            ListPendingRecv(callback) => {
                println!("listing pending recv");
                handle_callback_sender!(self.pending_recv.values().map(|v|v.into()).collect()=>callback)
            }
            ListPendingSend(callback) => {
                println!("listing pending send");
                handle_callback_sender!(self.pending_send.values().map(|v|v.into()).collect()=>callback)
            }
        }
    }
    /// Called when received send request from remote.
    fn on_new_pending_recv(
        &mut self,
        from: PeerId,
        file_name: String,
        remote_send_id: u64,
        bytes_total: u64,
    ) {
        let local_recv_id = self.next_recv_id();
        let span = trace_span!("Blob Recv", id = local_recv_id);
        let entered = span.enter();
        drop(entered);
        trace!("New pending recv created");
        self.pending_recv.insert(
            local_recv_id,
            PendingRecv {
                local_recv_id,
                remote_send_id,
                remote: from,
                bytes_total,
                file_name: file_name.clone(),
                span,
            },
        );
        self.out_events.push_back(OutEvent::IncomingFile {
            file_name,
            from,
            local_recv_id,
            bytes_total,
        });
    }
    /// Called when local decided to accept the file.
    fn accept_pending_recv(
        &mut self,
        file: File,
        recv_id: u64,
        callback: oneshot::Sender<Result<Duration, error::FileRecvError>>,
    ) {
        let PendingRecv {
            remote_send_id,
            remote,
            bytes_total,
            span,
            ..
        } = match self.pending_recv.remove(&recv_id) {
            Some(v) => v,
            None => {
                // Not found in pending recv, report the error directly to caller using callback
                handle_callback_sender!( Err(error::FileRecvError::PendingRecvNotFound) => callback);
                return;
            }
        };
        let entered = span.enter();
        trace!("Pending recv accepted");
        drop(entered);
        self.ongoing_recv.insert(
            remote_send_id,
            OngoingFileRecv {
                remote_send_id,
                local_recv_id: recv_id,
                file_handle: file,
                bytes_received: 0,
                bytes_total,
                remote,
                span,
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

    /// Called when rmeote accepts a pending send.
    /// If not found in the pending list, will return `Result::Err`.
    fn pending_send_accepted(&mut self, local_send_id: u64) -> Result<u64, ()> {
        if self.pending_send.get(&local_send_id).is_none()
            && self.ongoing_send.get(&local_send_id).is_none()
        {
            return Err(());
        }
        let PendingSend {
            local_send_id,
            remote,
            file,
            span,
            bytes_total,
            ..
        } = self.pending_send.remove(&local_send_id).unwrap();
        let entered = span.enter();
        trace!("Send request accepted.");
        drop(entered);
        self.ongoing_send.insert(
            local_send_id,
            OngoingFileSend {
                local_send_id,
                remote,
                bytes_total,
                file_handle: file,
                bytes_sent: 0,
                span,
            },
        );
        self.progress_ongoing_send(local_send_id);
        Ok(bytes_total)
    }
    /// Called when local node cancels the transmission.
    /// Once called, it is guaranteed that no more bytes will be written to the file.
    /// Receiving operation on local node will stop immediately without acknowledgement from remote.
    fn cancel_recv_by_local_recv_id(&mut self, local_recv_id: u64) -> bool {
        trace!("Cancelling recv id {}",local_recv_id);
        // Try to remove all recv record associted with the provided id.
        // If none is found, false is returned.
        if let Some((remote, remote_send_id)) = self.remove_recv_record(local_recv_id) {
            // Notify the remote about the cancellation.
            self.pending_handler_event
                .push_back(ToSwarm::NotifyHandler {
                    peer_id: remote,
                    handler: NotifyHandler::Any,
                    event: FromBehaviourEvent::CancelRecv { remote_send_id },
                });
            self.out_events
                .push_back(OutEvent::CancelledRecv(local_recv_id));
            return true;
        }
        false
    }
    /// Called when local cancelled transmission.
    /// Once called, it is guaranteed that no more bytes will be read.
    fn cancel_send_by_local_send_id(&mut self, local_send_id: u64) -> bool {
        trace!("Cancelling send id {}",local_send_id);
        if let Some(remote) = self.remove_send_record(local_send_id) {
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
    /// Call this to progress a send operation.
    fn progress_ongoing_send(&mut self, local_send_id: u64) {
        if self.ongoing_send.get_mut(&local_send_id).is_none(){
            return;
        }
        let ongoing_send = self.ongoing_send.get_mut(&local_send_id).unwrap();
        let entered = ongoing_send.span.enter();
        trace!("Progressing send");
        drop(entered);
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
        self.pending_handler_event
            .push_back(ToSwarm::NotifyHandler {
                peer_id: ongoing_send.remote,
                handler: NotifyHandler::Any,
                event: FromBehaviourEvent::File {
                    contents: vec,
                    local_send_id: ongoing_send.local_send_id,
                },
            });
        let entered = ongoing_send.span.enter();
        println!("Reading {} bytes", bytes_read);
        if bytes_read == 0 {
            trace!(
                "Removing this send because it's finished, {} bytes total, {} bytes sent",
                ongoing_send.bytes_total,
                ongoing_send.bytes_sent
            );
            drop(entered);
            self.ongoing_send.remove(&local_send_id);
        }
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
    fn remove_send_record(&mut self, local_send_id: u64) -> Option<PeerId> {
        if let Some(PendingSend { remote, .. }) = self.pending_send.remove(&local_send_id) {
            return Some(remote);
        };
        if let Some(OngoingFileSend { remote, .. }) = self.ongoing_send.remove(&local_send_id) {
            return Some(remote);
        }
        None
    }
    fn on_disconnect(&mut self, info: &ConnectionClosed) {
        if info.remaining_established < 1 {
            self.connected_peers.remove(&info.peer_id);
            trace!("Peer {} disconnected", info.peer_id);
            self.pending_send.retain(|_, v| v.remote != info.peer_id);
            self.pending_recv.retain(|_, v| v.remote != info.peer_id);
            self.ongoing_send.retain(|_, v| v.remote != info.peer_id);
            self.ongoing_recv.retain(|_, v| v.remote != info.peer_id);
        }
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
        event: <Self::ConnectionHandler as ConnectionHandler>::ToBehaviour,
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
                        bytes_received: ongoing_recv.bytes_total,
                        bytes_total: ongoing_recv.bytes_total,
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
                ongoing_recv.bytes_received += contents.len() as u64;
                if ongoing_recv.bytes_received % 1 << 15 == 0 {
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
                bytes_total,
            } => self.on_new_pending_recv(peer_id, file_name, remote_send_id, bytes_total),
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
                if let Ok(bytes_total) = self.pending_send_accepted(local_send_id) {
                    self.out_events.push_back(OutEvent::SendProgressed {
                        local_send_id,
                        bytes_total,
                        bytes_sent: 0,
                    });
                }
            }
            SendProgressed { local_send_id, .. } => {
                if let Some(v) = self.ongoing_send.get(&local_send_id) {
                    self.out_events.push_back(OutEvent::SendProgressed {
                        local_send_id,
                        bytes_sent: v.bytes_sent,
                        bytes_total: v.bytes_total,
                    });
                }
                self.progress_ongoing_send(local_send_id);
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
        trace!(name:"Poll","Polling owlnest_blob::Behaviour");
        if let Some(ev) = self.out_events.pop_front() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.pending_handler_event.pop_front() {
            return Poll::Ready(ev);
        }
        trace!(name:"Poll","Nothing to do owlnest_blob::Behaviour");
        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        match &event {
            FromSwarm::ConnectionClosed(info) => self.on_disconnect(info),
            _ => {}
        }
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }
}

struct OngoingFileRecv {
    remote_send_id: u64,
    local_recv_id: u64,
    remote: PeerId,
    bytes_received: u64,
    bytes_total: u64,
    file_handle: std::fs::File,
    span: tracing::Span,
}

struct OngoingFileSend {
    local_send_id: u64,
    remote: PeerId,
    bytes_sent: u64,
    bytes_total: u64,
    file_handle: std::fs::File,
    span: tracing::Span,
}
