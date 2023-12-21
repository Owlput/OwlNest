use crate::handle_callback_sender;
use crate::net::p2p::protocols::blob_transfer::handler::FromBehaviourEvent;

use super::*;
use libp2p::swarm::{ConnectionId, NetworkBehaviour, NotifyHandler, ToSwarm};
use libp2p::PeerId;
use std::collections::HashSet;
use std::io::Write;
use std::{collections::VecDeque, task::Poll};
use tracing::info;

pub(crate) const FILE_CHUNK_SIZE: usize = 524288;

pub struct Behaviour {
    config: Config,
    /// Pending events to emit to `Swarm`
    out_events: VecDeque<OutEvent>,
    /// Pending events to be processed by this `Behaviour`.
    in_events: VecDeque<InEvent>,
    /// A set for all connected peers.
    connected_peers: HashSet<PeerId>,
    recv_counter: u64,
    pending_recv: Vec<(u64, u64, String, PeerId, ConnectionId)>, /*(local_recv_id,remote_send_id,  file name,remote_peer, connectionID)*/
    ongoing_recv: Vec<OngoingFileRecv>,
}

impl Behaviour {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            out_events: VecDeque::new(),
            in_events: VecDeque::new(),
            connected_peers: HashSet::new(),
            recv_counter: 0,
            pending_recv: Vec::new(),
            ongoing_recv: Vec::new(),
        }
    }
    pub fn push_event(&mut self, ev: InEvent) {
        self.in_events.push_front(ev)
    }
    fn new_pending_recv(
        &mut self,
        from: PeerId,
        file_name: String,
        remote_send_id: u64,
        connection: ConnectionId,
    ) {
        self.pending_recv.push((
            self.recv_counter,
            remote_send_id,
            file_name.clone(),
            from,
            connection,
        ));
        self.out_events.push_back(OutEvent::IncomingFile {
            file_name: file_name,
            from,
            local_recv_id: self.recv_counter,
        });
        self.recv_counter += 1;
    }
    fn accept_pending_recv(
        &mut self,
        file_or_folder: Either<File, PathBuf>,
        recv_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    ) -> Option<(PeerId, ConnectionId, FromBehaviourEvent)> {
        let mut entry_filter = self.pending_recv.extract_if(|v| v.1 == recv_id);
        let maybe_entry = entry_filter.next();
        let (_, remote_send_id, file_name, peer_id, connection) = match maybe_entry {
            Some(v) => v,
            None => {
                handle_callback_sender!( Err(FileRecvError::PendingRecvNotFound) => callback);
                return None;
            }
        };
        // if entry_filter.count() > 0{
        //     warn!("Duplicate entries of file to receive found")
        // }
        let file = match file_or_folder {
            Either::Left(file) => file,
            Either::Right(mut folder) => {
                if let Err(e) = fs::create_dir_all(&folder) {
                    handle_callback_sender!(Err(FileRecvError::FsError(e.kind()))=>callback);
                    return None;
                }
                folder.push(file_name);
                let file = folder;
                match fs::OpenOptions::new().create(true).write(true).open(file) {
                    Ok(file) => file,
                    Err(e) => {
                        handle_callback_sender!(Err(FileRecvError::FsError(e.kind()))=>callback);
                        return None;
                    }
                }
            }
        };
        self.ongoing_recv.push({
            OngoingFileRecv {
                remote_send_id,
                local_recv_id: recv_id,
                file_handle: file,
            }
        });
        let ev = FromBehaviourEvent::AcceptFile {
            remote_send_id,
            callback,
        };
        Some((peer_id, connection, ev))
    }
    fn cancel_by_remote_send_id(&mut self, remote_send_id: u64) {
        self.ongoing_recv
            .retain(|v| v.remote_send_id != remote_send_id)
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
                info!("recv progressed");
                let finished = contents.len() == 0;
                for item in &mut self.ongoing_recv {
                    if item.remote_send_id == remote_send_id {
                        info!("found file, is finished? {}",finished);
                        match item.file_handle.write(&contents) {
                            Ok(len) => {
                                if len != contents.len() {
                                    tracing::error!("Partially written file!")
                                }
                            }
                            Err(e) => {
                                let ev = OutEvent::OngoingRecvError {
                                    local_recv_id: item.local_recv_id,
                                    error: format!("{:?}", e),
                                };
                                self.out_events.push_back(ev);
                            }
                        };
                        // TODO: Better error handling
                        if let Err(e) = item.file_handle.flush() {
                            tracing::error!("Unsuccessful file flushing: {:?}", e)
                        };
                        self.out_events.push_back(OutEvent::RecvProgressed {
                            local_recv_id: item.local_recv_id,
                            finished,
                        });
                        break;
                    }
                }
                if finished {
                    self.cancel_by_remote_send_id(remote_send_id)
                }
            }
            IncomingFile {
                file_name,
                remote_send_id,
            } => self.new_pending_recv(peer_id, file_name, remote_send_id, connection_id),
            Error(e) => {
                info!(
                    "Error occurred on peer {}:{:?}: {:#?}",
                    peer_id, connection_id, e
                );
                self.out_events.push_front(OutEvent::Error(e));
            }
            InboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::InboundNegotiated(peer_id));
                trace!(
                    "Successfully negotiated inbound connection from peer {}",
                    peer_id
                )
            }
            OutboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::OutboundNegotiated(peer_id));
                trace!(
                    "Successfully negotiated outbound connection from peer {}",
                    peer_id
                )
            }
            Unsupported => {
                self.out_events.push_back(OutEvent::Unsupported(peer_id));
                trace!("Peer {} doesn't support {}", peer_id, PROTOCOL_NAME)
            }
            FileSendAccepted { local_send_id } => {
                self.out_events.push_back(OutEvent::SendProgressed {
                    local_send_id,
                    rtt: Some(Duration::from_secs(0)),
                })
            }
            SendProgressed { local_send_id, rtt } => self
                .out_events
                .push_back(OutEvent::SendProgressed { local_send_id, rtt }),
            Cancel { send_id } => {
                self.out_events.push_back(OutEvent::Cancelled(send_id));
                self.cancel_by_remote_send_id(send_id);
            }
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<super::OutEvent, handler::FromBehaviourEvent>> {
        if let Some(ev) = self.out_events.pop_back() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_back() {
            use InEvent::*;
            match ev {
                SendFile {
                    file: file_to_send,
                    file_name,
                    to,
                    send_id,
                    callback,
                } => {
                    if self.connected_peers.contains(&to) {
                        return Poll::Ready(ToSwarm::NotifyHandler {
                            peer_id: to,
                            handler: NotifyHandler::Any,
                            event: handler::FromBehaviourEvent::NewFileSend {
                                file: file_to_send,
                                file_name,
                                local_send_id: send_id,
                                callback,
                            },
                        });
                    } else {
                        callback
                            .send(Err(FileSendReqError::PeerNotFound))
                            .expect("callback to succeed");
                    }
                }
                AcceptFile {
                    file_or_folder,
                    recv_id,
                    callback,
                } => {
                    if let Some((peer_id, connection, event)) =
                        self.accept_pending_recv(file_or_folder, recv_id, callback)
                    {
                        return Poll::Ready(ToSwarm::NotifyHandler {
                            peer_id,
                            handler: NotifyHandler::One(connection),
                            event,
                        });
                    }
                }
            }
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
        match &event {
            libp2p_swarm::FromSwarm::ConnectionClosed(info) => {
                if info.remaining_established < 1 {
                    self.connected_peers.remove(&info.peer_id);
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
    file_handle: std::fs::File,
}