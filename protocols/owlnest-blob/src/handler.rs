use super::error::{FileRecvError, FileSendError};
use super::{error::Error, protocol, PROTOCOL_NAME};
use futures_timer::Delay;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::handler_prelude::*;
use prost::{DecodeError, Message};
use std::{collections::VecDeque, time::Duration};
use tokio::sync::oneshot;
use tracing::{debug, trace, trace_span, Span};

#[derive(Debug)]
pub enum FromBehaviourEvent {
    NewFileSend {
        file_name: String,
        local_send_id: u64,
        callback: oneshot::Sender<Result<u64, FileSendError>>,
        bytes_total: u64,
    },
    /// A chunk of file
    FileChunk {
        local_send_id: u64,
        bytes_to_send: Vec<u8>,
    },
    AcceptFile {
        remote_send_id: u64,
        callback: oneshot::Sender<Result<Duration, FileRecvError>>,
    },
    /// Cancel command sent by file sender
    LocalCancelSend { local_send_id: u64, span: Span },
    /// Cancel command sent by file receiver
    LocalCancelRecv { remote_send_id: u64, span: Span },
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
    RemoteCancelSend {
        local_send_id: u64,
    },
    /// A recv request has been cancelled by sender.
    RemoteCancelRecv {
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
impl From<messages::CancelSend> for ToBehaviourEvent {
    fn from(value: messages::CancelSend) -> Self {
        let messages::CancelSend { remote_send_id } = value;
        ToBehaviourEvent::RemoteCancelRecv { remote_send_id }
    }
}
impl From<messages::CancelRecv> for ToBehaviourEvent {
    fn from(value: messages::CancelRecv) -> Self {
        let messages::CancelRecv { local_send_id } = value;
        ToBehaviourEvent::RemoteCancelSend { local_send_id }
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
pub mod messages {
    #[cfg(target_os = "windows")]
    include!(concat!(env!("OUT_DIR"), "\\messages.rs"));
    #[cfg(target_os = "linux")]
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
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
    pub(crate) fn new(config: super::Config) -> Self {
        Self {
            state: State::Active,
            pending_in_events: VecDeque::new(),
            pending_out_events: VecDeque::new(),
            timeout: Duration::from_millis(config.timeout_ms),
            inbound: None,
            outbound: None,
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
        if let Some(poll) = self.poll_inbound(cx) {
            return Poll::Ready(poll);
        }
        if let Some(poll) = self.poll_outbound(cx) {
            return Poll::Ready(poll);
        }
        if let Some(ev) = self.pending_out_events.pop_front() {
            trace!("Generate event to behaviour");
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
        }
        trace!("nothing to do, returning");
        Poll::Pending // Only reaches here when outbound is pending and no events to be fired
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
            // ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
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
    ControlSend(Option<oneshot::Sender<Result<u64, FileSendError>>>, u64),
    ControlRecv(Option<oneshot::Sender<Result<Duration, FileRecvError>>>),
    Cancel(Span),
    FileSend(u64),
}

type PollResult = ConnectionHandlerEvent<
    <Handler as ConnectionHandler>::OutboundProtocol,
    <Handler as ConnectionHandler>::OutboundOpenInfo,
    <Handler as ConnectionHandler>::ToBehaviour,
>;

impl Handler {
    fn poll_inbound(&mut self, cx: &mut std::task::Context<'_>) -> Option<PollResult> {
        if let Some(fut) = self.inbound.as_mut() {
            trace!("Polling inbound");
            let poll_result = fut.poll_unpin(cx);
            if let Poll::Ready(Err(e)) = poll_result {
                trace!("Inbound error: {}", e);
                self.inbound.take();
                let error = Error::IO(format!("IO Error: {:?}", e));
                return Some(ConnectionHandlerEvent::NotifyBehaviour(
                    ToBehaviourEvent::Error(error),
                ));
            }
            if let Poll::Ready(Ok((stream, bytes, message_type))) = poll_result {
                self.inbound = Some(super::protocol::recv(stream).boxed());
                if let Err(e) = self.on_message(bytes.as_ref(), message_type) {
                    self.pending_out_events.push_back(ToBehaviourEvent::Error(
                        Error::UnrecognizedMessage(format!(
                            "Unrecognized message({}): {:20}, {}",
                            message_type,
                            e,
                            String::from_utf8_lossy(&bytes[..20])
                        )),
                    ));
                }
                trace!("Inbound handled");
            }
        }
        None
    }
    fn poll_outbound(&mut self, cx: &mut std::task::Context<'_>) -> Option<PollResult> {
        loop {
            trace!("Polling outbound");
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, send_type, mut timer)) => {
                    let poll_result = task.poll_unpin(cx);
                    if poll_result.is_pending() {
                        trace!("Outbound busy");
                        if timer.poll_unpin(cx).is_ready() {
                            return Some(ConnectionHandlerEvent::NotifyBehaviour(
                                ToBehaviourEvent::Error(Error::IO("Timeout".into())),
                            ));
                        }
                        self.outbound = Some(OutboundState::Busy(task, send_type, timer));
                        break;
                    }
                    // Ready but resolved to an error
                    if let Poll::Ready(Err(e)) = poll_result {
                        debug!("Outbound error: {}", e);
                        return Some(ConnectionHandlerEvent::NotifyBehaviour(
                            ToBehaviourEvent::Error(Error::IO(e.to_string())),
                        )); // return with `self.outbound` == None
                    }
                    if let Poll::Ready(Ok((stream, rtt))) = poll_result {
                        trace!("Outbound ready");
                        match send_type {
                            SendType::ControlSend(callback, local_send_id) => {
                                self.pending_out_events
                                    .push_back(ToBehaviourEvent::FileSendPending { local_send_id });
                                handle_callback_sender!(Ok(local_send_id)=>callback.unwrap());
                            }
                            SendType::ControlRecv(callback) => {
                                handle_callback_sender!(Ok(rtt)=>callback.unwrap());
                            }
                            SendType::FileSend(id) => {
                                self.pending_out_events.push_back(
                                    ToBehaviourEvent::SendProgressed {
                                        local_send_id: id,
                                        rtt,
                                    },
                                );
                            }
                            SendType::Cancel(span) => {
                                span.in_scope(||debug!("Cancellation message has been sent successfully. Lifecycle ended."))
                            }
                        }
                        self.outbound = Some(OutboundState::Idle(stream));
                    }
                }
                // Outbound is free, get the next message sent
                Some(OutboundState::Idle(stream)) => {
                    trace!("Outbound idle");
                    // in favour of control messages
                    if self.pending_in_events.is_empty() {
                        self.outbound = Some(OutboundState::Idle(stream));
                        break; // exit because of no pending activity
                    }
                    let ev = self.pending_in_events.pop_front().expect("already handled");
                    self.progress_outbound(ev, stream);
                } // new outbound task needs to be polled for wake-up to be scheduled
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
                    return Some(event);
                }
            }
        }
        None
    }
    fn on_message(&mut self, bytes: &[u8], message_type: u8) -> Result<(), DecodeError> {
        use messages::*;
        trace!("Inbound ready");
        let ev = match message_type {
            0 => IncomingFile::decode(bytes)?.into(),
            1 => AcceptFile::decode(bytes)?.into(),
            2 => CancelSend::decode(bytes)?.into(),
            3 => CancelRecv::decode(bytes)?.into(),
            4 => FileChunk::decode(bytes)?.into(),
            _ => ToBehaviourEvent::Error(Error::IO("Unexpected header value".into())),
        };
        self.pending_out_events.push_back(ev);
        Ok(())
    }
    fn progress_outbound(&mut self, ev: FromBehaviourEvent, stream: Stream) {
        use FromBehaviourEvent::*;
        let send_type;
        let bytes;
        let message_type;
        match ev {
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
                send_type = SendType::ControlSend(Some(callback), local_send_id);
                bytes = message.encode_to_vec();
                message_type = 0;
            }
            AcceptFile {
                remote_send_id,
                callback,
            } => {
                let message = messages::AcceptFile {
                    local_send_id: remote_send_id,
                };
                bytes = message.encode_to_vec();
                send_type = SendType::ControlRecv(Some(callback));
                message_type = 1;
            }
            LocalCancelSend {
                local_send_id,
                span,
            } => {
                let message = messages::CancelSend {
                    remote_send_id: local_send_id,
                };
                bytes = message.encode_to_vec();
                send_type = SendType::Cancel(span);
                message_type = 2;
            }
            LocalCancelRecv {
                remote_send_id,
                span,
            } => {
                let message = messages::CancelRecv {
                    local_send_id: remote_send_id,
                };
                bytes = message.encode_to_vec();
                send_type = SendType::Cancel(span);
                message_type = 3;
            }
            FileChunk {
                local_send_id,
                bytes_to_send,
            } => {
                let message = messages::FileChunk {
                    remote_send_id: local_send_id,
                    content: bytes_to_send,
                };
                bytes = message.encode_to_vec();
                send_type = SendType::FileSend(local_send_id);
                message_type = 4;
            }
        }
        self.outbound = Some(OutboundState::Busy(
            protocol::send(stream, bytes, message_type).boxed(),
            send_type,
            Delay::new(self.timeout),
        ));
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
}
