use super::error::{FileRecvError, FileSendError};
use super::{error::Error, protocol, Config, PROTOCOL_NAME};
use futures_timer::Delay;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::handler_prelude::*;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::{collections::VecDeque, time::Duration};
use tokio::sync::oneshot;
use tracing::{trace, trace_span};

#[derive(Debug)]
pub enum FromBehaviourEvent {
    NewFileSend {
        file_name: String,
        local_send_id: u64,
        callback: oneshot::Sender<Result<(), FileSendError>>,
        bytes_total: u64,
    },
    /// A chunk of file
    File {
        contents: Vec<u8>,
        local_send_id: u64,
    },
    AcceptFile {
        remote_send_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    },
    /// Cancel command sent by file sender
    CancelSend { local_send_id: u64 },
    /// Cancel command sent by file receiver
    CancelRecv { remote_send_id: u64 },
}
#[derive(Debug)]
pub enum ToBehaviourEvent {
    /// A chunk of file has reached this peer.  
    /// The state of receiving is managed on behaviour.
    RecvProgressed {
        remote_send_id: u64,
        contents: Vec<u8>,
    },
    /// A chunk of file has been sent.
    SendProgressed {
        local_send_id: u64,
        rtt: Duration,
    },
    /// Remote wants to send a file to local peer.
    IncomingFile {
        file_name: String,
        remote_send_id: u64,
        bytes_total: u64,
    },
    /// Remote has accepted our file.
    /// Now local peer can start streaming the file.
    FileSendAccepted {
        local_send_id: u64,
    },
    /// Remote has received our request to send a file.
    FileSendPending {
        local_send_id: u64,
    },
    /// A send request has been cancelled by receiver.
    CancelSend {
        local_send_id: u64,
    },
    /// A recv request has been cancelled by sender.
    CancelRecv {
        remote_send_id: u64,
    },
    Error(Error),
    InboundNegotiated,
    OutboundNegotiated,
    Unsupported,
}

#[derive(Debug, Serialize, Deserialize)]
// TODO: rewrite with ProtoBuf for better performance
enum Packet {
    /// A new file will be sent.
    /// When observed by remote, the remote will push a pending receive.
    IncomingFile {
        file_name: String,
        /// remote when observed by receiver
        remote_send_id: u64,
        bytes_total: u64,
    },
    /// Accept a send request.
    /// When observed by remote, the file with the corresponding send ID will be streamed via `Packet::File`.
    AcceptFileSend {
        local_send_id: u64, /* local when observed by sender */
    },
    /// Packet containing bytes of the file.
    /// When observed by remote, the contents will be written to the corresponding file.
    File {
        remote_send_id: u64,
        contents: ByteBuf,
    },
    /// Cancel a transfer with the corresponding send ID.
    /// When observed by remote, a receive will be cancelled
    SendCancelled { remote_send_id: u64 },
    /// Cancel a transfer with the corresponding send ID.
    /// When observed by remote, a send will be cancelled.
    ReceiveCancelled { local_send_id: u64 },
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
}

impl Handler {
    pub fn new(config: Config) -> Self {
        Self {
            state: State::Active,
            pending_in_events: VecDeque::new(),
            pending_out_events: VecDeque::new(),
            timeout: config.timeout,
            inbound: None,
            outbound: None,
        }
    }
    /// Remove the send request from pending and ongoing, also dropping the file handle so no more bytes of the file will be sent.  
    /// This also means the remote will no longer receive `Packet::File` with the corresponding send ID, so state sync is required.
    fn cancell_send(&mut self, local_send_id: u64) {
        self.pending_out_events
            .push_back(ToBehaviourEvent::CancelSend { local_send_id });
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
                    tracing::debug!(
                        "Error occurred when negotiating protocol {}: {:?}",
                        PROTOCOL_NAME,
                        e
                    )
                }
            }
        }
    }
    fn on_packet(&mut self, packet: Packet) {
        use Packet::*;
        let ev = match packet {
            IncomingFile {
                file_name,
                remote_send_id, // send ID from remote peer
                bytes_total,
            } => {
                trace!("New incoming file {}", file_name);
                ToBehaviourEvent::IncomingFile {
                    file_name,
                    remote_send_id,
                    bytes_total,
                }
            }
            AcceptFileSend { local_send_id } => {
                trace!("Pending send accepted");
                ToBehaviourEvent::FileSendAccepted { local_send_id }
            }
            File {
                remote_send_id,
                contents,
            } => {
                trace!("File stream with length {}", contents.len());
                ToBehaviourEvent::RecvProgressed {
                    remote_send_id,
                    contents: contents.to_vec(),
                }
            }
            ReceiveCancelled { local_send_id } => ToBehaviourEvent::CancelSend { local_send_id },
            SendCancelled { remote_send_id } => {
                trace!("File send cancelled by remote");
                ToBehaviourEvent::CancelRecv { remote_send_id }
            }
        };
        self.pending_out_events.push_back(ev);
    }
    fn progress_outbound(&mut self, ev: FromBehaviourEvent, stream: Stream) {
        use FromBehaviourEvent::*;
        match ev {
            File {
                mut contents,
                local_send_id,
            } => {
                contents.shrink_to_fit();
                let packet = Packet::File {
                    remote_send_id: local_send_id,
                    contents: ByteBuf::from(contents),
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, packet.as_bytes()).boxed(),
                    SendType::FileSend(local_send_id),
                    Delay::new(self.timeout),
                ));
            }
            NewFileSend {
                file_name,
                local_send_id,
                callback,
                bytes_total,
            } => {
                let packet = Packet::IncomingFile {
                    file_name,
                    remote_send_id: local_send_id,
                    bytes_total,
                };
                // Put Outbound into send state
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, packet.as_bytes()).boxed(),
                    SendType::ControlSend(callback, local_send_id),
                    Delay::new(self.timeout),
                ))
            }
            AcceptFile {
                remote_send_id,
                callback,
            } => {
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
            CancelSend { local_send_id } => {
                self.cancell_send(local_send_id);

                let packet = Packet::SendCancelled {
                    remote_send_id: local_send_id,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, packet.as_bytes()).boxed(),
                    SendType::Cancel,
                    Delay::new(self.timeout),
                ))
            }
            CancelRecv { remote_send_id } => {
                let packet = Packet::ReceiveCancelled {
                    local_send_id: remote_send_id,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, packet.as_bytes()).boxed(),
                    SendType::Cancel,
                    Delay::new(self.timeout),
                ))
            }
        }
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = FromBehaviourEvent;
    type ToBehaviour = ToBehaviourEvent;
    type InboundProtocol = ReadyUpgrade<&'static str>;
    type OutboundProtocol = ReadyUpgrade<&'static str>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();
    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ())
    }
    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        self.pending_in_events.push_back(event)
    }
    fn connection_keep_alive(&self) -> bool {
        true
    }
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::ToBehaviour>,
    > {
        let span = trace_span!("owlnest_blob::ConnectionHandler::poll");
        let _entered = span.enter();
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
            trace!("Polling inbound");
            match fut.poll_unpin(cx) {
                Poll::Pending => {
                    trace!("Inbound pending")
                }
                Poll::Ready(Err(e)) => {
                    let error = Error::IO(format!("IO Error: {:?}", e));
                    self.pending_out_events
                        .push_back(ToBehaviourEvent::Error(error));
                    trace!("Inbound error");
                    self.inbound = None;
                }
                Poll::Ready(Ok((stream, bytes))) => {
                    trace!("Inbound ready");
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                    match serde_json::from_slice::<Packet>(&bytes) {
                        Ok(packet) => self.on_packet(packet),
                        Err(e) => {
                            self.pending_out_events.push_back(ToBehaviourEvent::Error(
                                Error::UnrecognizedMessage(format!("{:?}", e)),
                            ));
                        }
                    };
                    trace!("Inbound handled");
                }
            }
        }
        loop {
            trace!("Polling outbound");
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, send_type, mut timer)) => {
                    match task.poll_unpin(cx) {
                        Poll::Pending => {
                            if timer.poll_unpin(cx).is_ready() {
                                self.pending_out_events
                                    .push_back(ToBehaviourEvent::Error(Error::IO("Timeout".into())))
                            } else {
                                trace!("Outbound busy");
                                // Put the future back
                                self.outbound = Some(OutboundState::Busy(task, send_type, timer));
                                return Poll::Pending;
                            }
                        }
                        // Ready
                        Poll::Ready(Ok((stream, rtt))) => {
                            trace!("Outbound ready");
                            match send_type {
                                SendType::ControlSend(callback, local_send_id) => {
                                    self.pending_out_events.push_back(
                                        ToBehaviourEvent::FileSendPending { local_send_id },
                                    );
                                    handle_callback_sender!(Ok(())=>callback);
                                }
                                SendType::ControlRecv(callback) => {
                                    handle_callback_sender!(Ok(rtt)=>callback);
                                }
                                SendType::FileSend(id) => {
                                    self.pending_out_events.push_back(
                                        ToBehaviourEvent::SendProgressed {
                                            local_send_id: id,
                                            rtt,
                                        },
                                    );
                                }
                                SendType::Cancel => {}
                            }
                            self.outbound = Some(OutboundState::Idle(stream));
                        }
                        // Ready but resolved to an error
                        Poll::Ready(Err(e)) => {
                            trace!("Outbound error");
                            self.pending_out_events
                                .push_back(ToBehaviourEvent::Error(Error::IO(e.to_string())));
                        }
                    }
                }
                // Outbound is free, get the next message sent
                Some(OutboundState::Idle(stream)) => {
                    trace!("Outbound idle");
                    // in favour of control messages
                    if let Some(ev) = self.pending_in_events.pop_front() {
                        self.progress_outbound(ev, stream);
                    } else {
                        self.outbound = Some(OutboundState::Idle(stream));
                        break;
                    };
                }
                Some(OutboundState::OpenStream) => {
                    self.outbound = Some(OutboundState::OpenStream);
                    break;
                }
                None => {
                    trace!("Opening stream");
                    self.outbound = Some(OutboundState::OpenStream);
                    let protocol =
                        SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ());
                    let event = ConnectionHandlerEvent::OutboundSubstreamRequest { protocol };
                    return Poll::Ready(event);
                }
            }
        }
        if let Some(ev) = self.pending_out_events.pop_front() {
            trace!("Generate event to behaviour");
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
        }
        trace!("nothing to do, returning");
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
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            ConnectionEvent::LocalProtocolsChange(_) => {}
            ConnectionEvent::RemoteProtocolsChange(_) => {}
            uncovered => unimplemented!("New branch {:?} not covered", uncovered),
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
enum SendType {
    ControlSend(oneshot::Sender<Result<(), FileSendError>>, u64),
    ControlRecv(oneshot::Sender<Result<Duration, FileRecvError>>),
    Cancel,
    FileSend(u64),
}
