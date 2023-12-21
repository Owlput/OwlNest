use super::error::{FileRecvError, FileSendReqError};
use super::{protocol, Config, Error, PROTOCOL_NAME};
use crate::handle_callback_sender;
use crate::net::p2p::handler_prelude::*;
use futures_timer::Delay;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::{collections::VecDeque, time::Duration};
use tokio::sync::oneshot;
use tracing::{error, info, trace, warn};
use super::behaviour::FILE_CHUNK_SIZE;

#[derive(Debug)]
pub enum FromBehaviourEvent {
    NewFileSend {
        file: File,
        file_name: String,
        local_send_id: u64,
        callback: oneshot::Sender<Result<Duration, FileSendReqError>>,
    },
    AcceptFile {
        remote_send_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    },
    Cancell {
        local_send_id: u64,
    },
}
#[derive(Debug)]
pub enum ToBehaviourEvent {
    RecvProgressed {
        remote_send_id: u64,
        contents: Vec<u8>,
    },
    SendProgressed {
        local_send_id: u64,
        rtt: Option<Duration>,
    },
    IncomingFile {
        file_name: String,
        remote_send_id: u64,
    },
    FileSendAccepted {
        local_send_id: u64,
    },
    Cancel {
        send_id: u64,
    },
    Error(Error),
    InboundNegotiated,
    OutboundNegotiated,
    Unsupported,
}

#[derive(Debug, Serialize, Deserialize)]
// TODO: rewrite with ProtoBuf for better performance
enum Packet {
    IncomingFile {
        file_name: String,
        /// remote when observed by receiver
        remote_send_id: u64,
    },
    AcceptFileSend {
        local_send_id: u64, /* local when observed by sender */
    },
    File {
        remote_send_id: u64,
        contents: Vec<u8>,
    },
    Cancelled {
        send_id: u64,
    },
}
impl Packet {
    fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Serialize to succeed")
    }
}

enum State {
    Inactive { reported: bool },
    Active,
}

pub struct Handler {
    state: State,
    pending_in_events: VecDeque<FromBehaviourEvent>,
    pending_out_events: VecDeque<ToBehaviourEvent>,
    timeout: Duration,
    inbound: Option<PendingVerf>,
    outbound: Option<OutboundState>,
    pending_file_send: Vec<PendingFileSend>,
    pending_file_recv: Vec<(u64, String)>, // (remote_send_id, file_name)
    ongoing_file_send: Option<(File, u64)>,
    ongoing_file_recv: Option<(File, u64)>,
}

use libp2p::swarm::{handler::DialUpgradeError, StreamUpgradeError};
impl Handler {
    pub fn new(config: Config) -> Self {
        Self {
            state: State::Active,
            pending_in_events: VecDeque::new(),
            pending_out_events: VecDeque::new(),
            timeout: config.timeout,
            inbound: None,
            outbound: None,
            pending_file_send: Vec::new(),
            pending_file_recv: Vec::new(),
            ongoing_file_recv: None,
            ongoing_file_send: None,
        }
    }
    fn new_pending_send(&mut self, file: File, file_name: String, local_send_id: u64) -> Packet {
        self.pending_file_send.push(PendingFileSend {
            local_send_id,
            file,
        });
        Packet::IncomingFile {
            file_name,
            remote_send_id: local_send_id,
        }
    }
    fn new_pending_recv(&mut self, remote_send_id: u64, file_name: String) {
        self.pending_file_recv.push((remote_send_id, file_name));
    }
    fn accept_recv(
        &mut self,

        remote_send_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    ) -> Result<oneshot::Sender<Result<Duration, FileRecvError>>, ()> {
        let mut entry_filter = self.pending_file_recv.extract_if(|v| v.0 == remote_send_id);
        let (_, _) = match entry_filter.next() {
            Some(v) => v,
            None => {
                handle_callback_sender!(Err(FileRecvError::PendingRecvNotFound)=>callback);
                return Err(());
            }
        };
        Ok(callback)
    }
    fn initate_sending(&mut self, local_send_id: u64) {
        let mut entry_filter = self
            .pending_file_send
            .extract_if(|v| v.local_send_id == local_send_id);
        if let Some(pending_send) = entry_filter.next() {
            self.ongoing_file_send = Some((pending_send.file, local_send_id));
        } else {
            self.pending_in_events
                .push_back(FromBehaviourEvent::Cancell { local_send_id })
        }
    }
    fn cancell_send(&mut self, target_local_send_id: u64) -> ToBehaviourEvent {
        let mut entry_filter = self
            .pending_file_send
            .extract_if(|v| v.local_send_id == target_local_send_id);
        if let Some(_) = entry_filter.next() {
            return ToBehaviourEvent::Cancel {
                send_id: target_local_send_id,
            };
        }
        ToBehaviourEvent::Cancel {
            send_id: target_local_send_id,
        }
    }
    fn cancell_recv_by_remote_send_id(&mut self, remote_send_id: u64) -> ToBehaviourEvent {
        let mut entry_filter = self.pending_file_recv.extract_if(|v| v.0 == remote_send_id);
        if let Some(_) = entry_filter.next() {
            return ToBehaviourEvent::Cancel {
                send_id: remote_send_id,
            };
        }
        if let None = self.ongoing_file_recv {
            warn!("Trying to cancel a non-existent pending send");
            return ToBehaviourEvent::Error(Error::SendIdNotFound(remote_send_id));
        }
        // Not found in pending list
        if let Some((file, send_id)) = self.ongoing_file_recv.take() {
            if send_id != remote_send_id {
                // Not everywhere
                self.ongoing_file_recv = Some((file, send_id));
                warn!("Trying to cancel a non-existent pending send");
                return ToBehaviourEvent::Error(Error::SendIdNotFound(remote_send_id));
            }
        }
        ToBehaviourEvent::Cancel {
            send_id: remote_send_id,
        }
    }
    #[inline]
    fn on_dial_upgrade_error(
        &mut self,
        DialUpgradeError { error, .. }: DialUpgradeError<
            <Self as ConnectionHandler>::OutboundOpenInfo,
            <Self as ConnectionHandler>::OutboundProtocol,
        >,
    ) {
        self.outbound = None;
        match error {
            StreamUpgradeError::NegotiationFailed => {
                self.state = State::Inactive { reported: false };
            }
            e => {
                let e = format!("{:?}", e);
                if !e.contains("Timeout") {
                    warn!(
                        "Error occurred when negotiating protocol {}: {:?}",
                        PROTOCOL_NAME, e
                    )
                }
            }
        }
    }
}

use libp2p::core::upgrade::ReadyUpgrade;
use libp2p::swarm::{ConnectionHandlerEvent, SubstreamProtocol};
impl ConnectionHandler for Handler {
    type FromBehaviour = FromBehaviourEvent;
    type ToBehaviour = ToBehaviourEvent;
    type InboundProtocol = ReadyUpgrade<&'static str>;
    type OutboundProtocol = ReadyUpgrade<&'static str>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();
    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ())
    }
    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        info!("Received event {:#?}", event);
        self.pending_in_events.push_front(event)
    }
    fn connection_keep_alive(&self) -> bool {
        true
    }
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
        >,
    > {
        match self.state {
            State::Inactive { reported: true } => return Poll::Pending,
            State::Inactive { reported: false } => {
                self.state = State::Inactive { reported: true };
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                    ToBehaviourEvent::Unsupported,
                ));
            }
            State::Active => {}
        };
        if let Some(fut) = self.inbound.as_mut() {
            match fut.poll_unpin(cx) {
                Poll::Pending => {}
                Poll::Ready(Err(e)) => {
                    info!("IO Error occurred!");
                    let error = Error::IO(format!("IO Error: {:?}", e));
                    self.pending_out_events
                        .push_front(ToBehaviourEvent::Error(error));
                    self.inbound = None;
                }
                Poll::Ready(Ok((stream, bytes))) => {
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                    let packet: Packet = match serde_json::from_slice(&bytes) {
                        Ok(v) => v,
                        Err(e) => {
                            self.pending_out_events.push_back(ToBehaviourEvent::Error(
                                Error::UnrecognizedMessage(format!("{:?}", e)),
                            ));
                            return Poll::Pending;
                        }
                    };
                    use Packet::*;
                    let event = match packet {
                        IncomingFile {
                            file_name,
                            remote_send_id, // send ID from remote peer
                        } => {
                            info!("New incoming file {}", file_name);
                            self.new_pending_recv(remote_send_id, file_name.clone());
                            ToBehaviourEvent::IncomingFile {
                                file_name,
                                remote_send_id,
                            }
                        }
                        AcceptFileSend {
                            local_send_id, /* send ID from remote, for local */
                        } => {
                            info!("Pending send accepted");
                            self.initate_sending(local_send_id);
                            ToBehaviourEvent::FileSendAccepted { local_send_id }
                        }
                        File {
                            remote_send_id,
                            contents,
                        } => {
                            info!("File stream with length {}", contents.len());
                            ToBehaviourEvent::RecvProgressed {
                                remote_send_id,
                                contents,
                            }
                        }
                        Cancelled { send_id } => {
                            info!("File send cancelled");
                            self.cancell_recv_by_remote_send_id(send_id)
                        }
                    };
                    return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(event));
                }
            }
        }
        loop {
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, send_type, mut timer)) => {
                    match task.poll_unpin(cx) {
                        Poll::Pending => {
                            if timer.poll_unpin(cx).is_ready() {
                                self.pending_out_events
                                    .push_back(ToBehaviourEvent::Error(Error::IO("Timeout".into())))
                            } else {
                                // Put the future back
                                self.outbound = Some(OutboundState::Busy(task, send_type, timer));
                                // End the loop because the outbound is busy
                                break;
                            }
                        }
                        // Ready
                        Poll::Ready(Ok((stream, rtt))) => {
                            match send_type {
                                SendType::ControlSend(callback) => {
                                    handle_callback_sender!(Ok(rtt)=>callback);
                                }
                                SendType::ControlRecv(callback) => {
                                    handle_callback_sender!(Ok(rtt)=>callback);
                                }
                                SendType::Cancel => {}
                                SendType::FileSend(id) => {
                                    self.pending_out_events.push_back(
                                        ToBehaviourEvent::SendProgressed {
                                            local_send_id: id,
                                            rtt: Some(rtt),
                                        },
                                    );
                                }
                            }

                            // Free the outbound
                            self.outbound = Some(OutboundState::Idle(stream));
                        }
                        // Ready but resolved to an error
                        Poll::Ready(Err(e)) => {
                            self.pending_out_events
                                .push_back(ToBehaviourEvent::Error(Error::IO(e.to_string())));
                        }
                    }
                }
                // Outbound is free, get the next message sent
                Some(OutboundState::Idle(stream)) => {
                    // in favour of control messages
                    if let Some(ev) = self.pending_in_events.pop_back() {
                        use FromBehaviourEvent::*;
                        match ev {
                            NewFileSend {
                                file,
                                file_name,
                                local_send_id,
                                callback,
                            } => {
                                info!("received new file send");
                                self.new_pending_send(file, file_name.clone(), local_send_id);
                                let packet = Packet::IncomingFile {
                                    file_name,
                                    remote_send_id: local_send_id,
                                };
                                // Put Outbound into send state
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(stream, packet.as_bytes()).boxed(),
                                    SendType::ControlSend(callback),
                                    Delay::new(self.timeout),
                                ))
                            }
                            AcceptFile {
                                remote_send_id,
                                callback,
                            } => {
                                if let Ok(callback) = self.accept_recv(remote_send_id, callback) {
                                    let packet = Packet::AcceptFileSend {
                                        local_send_id: remote_send_id,
                                    };
                                    // Put Outbound into send state
                                    self.outbound = Some(OutboundState::Busy(
                                        protocol::send(stream, packet.as_bytes()).boxed(),
                                        SendType::ControlRecv(callback),
                                        Delay::new(self.timeout),
                                    ))
                                }
                            }
                            Cancell { local_send_id } => {
                                let ev = self.cancell_send(local_send_id);
                                self.pending_out_events.push_back(ev);
                                let packet = Packet::Cancelled {
                                    send_id: local_send_id,
                                };
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(stream, packet.as_bytes()).boxed(),
                                    SendType::Cancel,
                                    Delay::new(self.timeout),
                                ))
                            }
                        }

                        continue;
                    };
                    if let Some((mut file, local_send_id)) = self.ongoing_file_send.take() {
                        let mut buf = [0u8; FILE_CHUNK_SIZE];
                        let bytes_read = match file.read(&mut buf) {
                            Err(e) => {
                                error!("{:?}", e);
                                let ev = self.cancell_send(local_send_id);
                                self.pending_out_events.push_back(ev);
                                let packet = Packet::Cancelled {
                                    send_id: local_send_id,
                                };
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(stream, packet.as_bytes()).boxed(),
                                    SendType::FileSend(local_send_id),
                                    Delay::new(self.timeout),
                                ));
                                continue;
                            }
                            Ok(bytes_read) => bytes_read,
                        };

                        let mut vec: Vec<u8> = buf.into();
                        if bytes_read < FILE_CHUNK_SIZE {
                            info!("truncate to {}",bytes_read);
                            vec.truncate(bytes_read)
                        }
                        let packet = Packet::File {
                            remote_send_id: local_send_id,
                            contents: vec,
                        };
                        self.outbound = Some(OutboundState::Busy(
                            protocol::send(stream, packet.as_bytes()).boxed(),
                            SendType::FileSend(local_send_id),
                            Delay::new(self.timeout),
                        ));
                        if bytes_read == 0 {
                            info!("bytes read is zero!");
                            self.cancell_send(local_send_id);
                            self.pending_out_events
                                .push_back(ToBehaviourEvent::SendProgressed {
                                    local_send_id,
                                    rtt: None,
                                });
                            break;
                        }
                        self.ongoing_file_send = Some((file,local_send_id))
                    } else {
                        self.outbound = Some(OutboundState::Idle(stream));
                        break;
                    }
                }
                Some(OutboundState::OpenStream) => {
                    self.outbound = Some(OutboundState::OpenStream);
                    break;
                }
                None => {
                    self.outbound = Some(OutboundState::OpenStream);
                    let protocol =
                        SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ());
                    let event = ConnectionHandlerEvent::OutboundSubstreamRequest { protocol };
                    return Poll::Ready(event);
                }
            }
        }
        if let Some(ev) = self.pending_out_events.pop_back() {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
        }
        Poll::Pending
    }
    fn on_connection_event(
        &mut self,
        event: ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol: stream,
                info: (),
            }) => {
                self.inbound = Some(super::protocol::recv(stream).boxed());
                self.pending_out_events
                    .push_back(ToBehaviourEvent::InboundNegotiated)
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol: stream,
                ..
            }) => {
                self.outbound = Some(OutboundState::Idle(stream));
                self.pending_out_events
                    .push_back(ToBehaviourEvent::OutboundNegotiated)
            }
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
            ConnectionEvent::LocalProtocolsChange(_) => {}
            ConnectionEvent::RemoteProtocolsChange(_) => {}
            _ => unimplemented!("New branch not handled!"),
        }
    }
}

type PendingVerf = BoxFuture<'static, Result<(Stream, Vec<u8>), io::Error>>;
type PendingSend = BoxFuture<'static, Result<(Stream, Duration), io::Error>>;

enum OutboundState {
    OpenStream,
    Idle(Stream),
    Busy(PendingSend, SendType, Delay),
}

struct PendingFileSend {
    local_send_id: u64,
    file: File,
}

enum SendType {
    ControlSend(oneshot::Sender<Result<Duration, FileSendReqError>>),
    ControlRecv(oneshot::Sender<Result<Duration, FileRecvError>>),
    Cancel,
    FileSend(u64),
}
