use super::error::{FileRecvError, FileSendError};
use super::{error::Error, protocol, Config, PROTOCOL_NAME};
use futures_timer::Delay;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::handler_prelude::*;
use prost::Message;
use std::sync::Arc;
use std::{collections::VecDeque, time::Duration};
use tokio::sync::oneshot;
use tracing::{debug, trace, trace_span};

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
        local_send_id: u64,
        bytes_to_send: Arc<[u8]>,
        len: usize,
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
        len: usize,
        content: Arc<[u8]>,
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
    pub(crate) fn new(config: Config) -> Self {
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

type PendingVerf = BoxFuture<'static, Result<(Stream, Arc<[u8]>, usize, u8), io::Error>>;
type PendingSend = BoxFuture<'static, Result<(Stream, Duration), io::Error>>;

enum OutboundState {
    OpenStream,
    Idle(Stream),
    Busy(PendingSend, SendType, Delay),
}
enum SendType {
    ControlSend(Option<oneshot::Sender<Result<(), FileSendError>>>, u64),
    ControlRecv(Option<oneshot::Sender<Result<Duration, FileRecvError>>>),
    Cancel,
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
                trace!("Inbound error");
                self.inbound.take();
                let error = Error::IO(format!("IO Error: {:?}", e));
                return Some(ConnectionHandlerEvent::NotifyBehaviour(
                    ToBehaviourEvent::Error(error),
                ));
            }
            if let Poll::Ready(Ok((stream, bytes, len, msg_type))) = poll_result {
                self.inbound = Some(super::protocol::recv(stream).boxed());
                self.on_message(bytes, len, msg_type);
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
                    if let Poll::Pending = poll_result {
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
                        debug!("Outbound error");
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
                                handle_callback_sender!(Ok(())=>callback.unwrap());
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
                            SendType::Cancel => {}
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
    fn on_message(&mut self, bytes: Arc<[u8]>, len: usize, msg_type: u8) {
        use messages::*;
        trace!("Inbound ready");
        let err_def = |_e| {
            ToBehaviourEvent::Error(Error::UnrecognizedMessage(format!(
                "Unrecognized message: {:20}",
                String::from_utf8_lossy(&bytes[..20])
            )))
        };
        let ev = match msg_type {
            0 => IncomingFile::decode(&bytes[..len])
                .map(Into::into)
                .unwrap_or_else(err_def),
            1 => AcceptFile::decode(&bytes[..len])
                .map(Into::into)
                .unwrap_or_else(err_def),
            2 => {
                let mut num_be = [0u8; 8];
                num_be.copy_from_slice(&bytes[0..8]);
                ToBehaviourEvent::RecvProgressed {
                    remote_send_id: u64::from_be_bytes(num_be),
                    len,
                    content: bytes,
                }
            }
            3 => CancelSend::decode(&bytes[..len])
                .map(Into::into)
                .unwrap_or_else(err_def),
            4 => CancelRecv::decode(&bytes[..len])
                .map(Into::into)
                .unwrap_or_else(err_def),
            _ => ToBehaviourEvent::Error(Error::UnrecognizedMessage(format!(
                "Unrecognized message: {:20}",
                String::from_utf8_lossy(&bytes[..20])
            ))),
        };
        self.pending_out_events.push_back(ev);
    }
    fn progress_outbound(&mut self, ev: FromBehaviourEvent, stream: Stream) {
        use FromBehaviourEvent::*;
        match ev {
            File {
                local_send_id,
                bytes_to_send,
                len,
            } => {
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, bytes_to_send, len).boxed(),
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
                let mut buf = [0u8; 1 << 6];
                let len = message.encoded_len();
                let mut header = len.to_be_bytes();
                header[0] = 0;
                buf[0..8].copy_from_slice(&header);
                message.encode(&mut &mut buf[8..]).expect("result to fit");
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, Arc::new(buf), len + 8).boxed(),
                    SendType::ControlSend(Some(callback), local_send_id),
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
                let mut buf = [0u8; 1 << 5];
                let len = message.encoded_len();
                let mut header = len.to_be_bytes();
                header[0] = 1;
                buf[0..8].copy_from_slice(&header);
                message.encode(&mut &mut buf[8..]).expect("result to fit");
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, Arc::new(buf), len + 8).boxed(),
                    SendType::ControlRecv(Some(callback)),
                    Delay::new(self.timeout),
                ))
            }
            CancelSend { local_send_id } => {
                self.cancell_send(local_send_id);

                let message = messages::CancelSend {
                    remote_send_id: local_send_id,
                };
                let mut buf = [0u8; 1 << 5];
                let len = message.encoded_len();
                let mut header = len.to_be_bytes();
                header[0] = 3;
                buf[0..8].copy_from_slice(&header);
                message.encode(&mut &mut buf[8..]).expect("result to fit");
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, Arc::new(buf), len + 8).boxed(),
                    SendType::Cancel,
                    Delay::new(self.timeout),
                ))
            }
            CancelRecv { remote_send_id } => {
                let message = messages::CancelRecv {
                    local_send_id: remote_send_id,
                };
                let mut buf = [0u8; 1 << 5];
                let len = message.encoded_len();
                let mut header = len.to_be_bytes();
                header[0] = 4;
                buf[0..8].copy_from_slice(&header);
                message.encode(&mut &mut buf[8..]).expect("result to fit");
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, Arc::new(buf), len + 8).boxed(),
                    SendType::Cancel,
                    Delay::new(self.timeout),
                ))
            }
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
