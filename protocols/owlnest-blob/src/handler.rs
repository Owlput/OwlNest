use super::error::{FileRecvError, FileSendError};
use super::{error::Error, protocol, Config, PROTOCOL_NAME};
use futures_timer::Delay;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::handler_prelude::*;
use prost::Message;
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
        content: Vec<u8>,
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
impl From<messages::IncomingFile> for ToBehaviourEvent {
    fn from(value: messages::IncomingFile) -> Self {
        let messages::IncomingFile {
            remote_send_id,
            bytes_total,
            file_name,
        } = value;
        ToBehaviourEvent::IncomingFile {
            file_name,
            remote_send_id,
            bytes_total,
        }
    }
}
impl From<messages::AcceptFile> for ToBehaviourEvent {
    fn from(value: messages::AcceptFile) -> Self {
        let messages::AcceptFile { local_send_id } = value;
        ToBehaviourEvent::FileSendAccepted { local_send_id }
    }
}
impl From<messages::FileChunk> for ToBehaviourEvent {
    fn from(value: messages::FileChunk) -> Self {
        let messages::FileChunk {
            remote_send_id,
            content,
        } = value;
        ToBehaviourEvent::RecvProgressed {
            remote_send_id,
            content,
        }
    }
}
impl From<messages::CancelSend> for ToBehaviourEvent {
    fn from(value: messages::CancelSend) -> Self {
        let messages::CancelSend { remote_send_id } = value;
        ToBehaviourEvent::CancelRecv { remote_send_id }
    }
}
impl From<messages::CancelRecv> for ToBehaviourEvent {
    fn from(value: messages::CancelRecv) -> Self {
        let messages::CancelRecv { local_send_id } = value;
        ToBehaviourEvent::CancelSend { local_send_id }
    }
}
pub mod messages {
    include!(concat!(env!("OUT_DIR"), "\\messages.rs"));
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
    fn on_message(&mut self, bytes: Vec<u8>, msg_type: u8) {
        use messages::*;
        trace!("Inbound ready");
        let err_def = |_e| {
            ToBehaviourEvent::Error(Error::UnrecognizedMessage(format!(
                "Unrecognized message: {:20}",
                String::from_utf8_lossy(&bytes)
            )))
        };
        let ev = match msg_type {
            0 => IncomingFile::decode(&*bytes)
                .map(Into::into)
                .unwrap_or_else(err_def),
            1 => AcceptFile::decode(&*bytes)
                .map(Into::into)
                .unwrap_or_else(err_def),
            2 => FileChunk::decode(&*bytes)
                .map(Into::into)
                .unwrap_or_else(err_def),
            3 => CancelSend::decode(&*bytes)
                .map(Into::into)
                .unwrap_or_else(err_def),
            4 => CancelRecv::decode(&*bytes)
                .map(Into::into)
                .unwrap_or_else(err_def),
            _ => ToBehaviourEvent::Error(Error::UnrecognizedMessage(format!(
                "Unrecognized message: {:20}",
                String::from_utf8_lossy(&bytes)
            ))),
        };
        self.pending_out_events.push_back(ev);
    }
    fn progress_outbound(&mut self, ev: FromBehaviourEvent, stream: Stream) {
        use FromBehaviourEvent::*;
        match ev {
            File {
                contents,
                local_send_id,
            } => {
                let message = messages::FileChunk {
                    remote_send_id: local_send_id,
                    content: contents,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, message.encode_to_vec(), 2).boxed(),
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
                let message = messages::IncomingFile {
                    remote_send_id: local_send_id,
                    bytes_total,
                    file_name,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, message.encode_to_vec(), 0).boxed(),
                    SendType::ControlSend(callback, local_send_id),
                    Delay::new(self.timeout),
                ))
            }
            AcceptFile {
                remote_send_id,
                callback,
            } => {
                let message = messages::AcceptFile {
                    local_send_id: remote_send_id,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, message.encode_to_vec(), 1).boxed(),
                    SendType::ControlRecv(callback),
                    Delay::new(self.timeout),
                ))
            }
            CancelSend { local_send_id } => {
                self.cancell_send(local_send_id);

                let message = messages::CancelSend {
                    remote_send_id: local_send_id,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, message.encode_to_vec(), 3).boxed(),
                    SendType::Cancel,
                    Delay::new(self.timeout),
                ))
            }
            CancelRecv { remote_send_id } => {
                let message = messages::CancelRecv {
                    local_send_id: remote_send_id,
                };
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, message.encode_to_vec(), 4).boxed(),
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
                Poll::Ready(Ok((stream, bytes, msg_type))) => {
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                    self.on_message(bytes, msg_type);
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

type PendingVerf = BoxFuture<'static, Result<(Stream, Vec<u8>, u8), io::Error>>;
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
