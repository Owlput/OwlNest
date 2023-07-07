use super::{inbound_upgrade, outbound_upgrade, protocol, PUSH_PROTOCOL_NAME};
use crate::net::p2p::handler_prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{collections::VecDeque, fmt::Display};
use tracing::warn;

#[derive(Debug)]
pub struct InEvent {
    push_type: PushType,
    callback: CallbackSender,
}
impl InEvent {
    pub fn new(push_type: PushType, callback: CallbackSender) -> Self {
        InEvent {
            push_type,
            callback,
        }
    }
    pub fn into_inner(self) -> (PushType, CallbackSender) {
        (self.push_type, self.callback)
    }
}

#[derive(Debug, Clone)]
pub enum PushType {
    Msg(String),
}

/// Data structure be sent through the wire.
#[derive(Debug, Serialize, Deserialize)]
enum Packet {
    Msg(String),
}

#[derive(Debug)]
pub enum OutEvent {
    Message(String),
    Error(Error),
    Unsupported,
}

#[derive(Debug)]
pub enum Error {
    Corrupted(serde_json::Error, Vec<u8>),
    IO(io::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupted(e, raw) => f.write_str(&format!(
                "Corrupted data when deserializing: {:?}, {:#?}",
                e, raw
            )),
            Error::IO(e) => f.write_str(&format!("IO error: {:#?}", e)),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::IO(e) = self {
            e.source()
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum HandleError {}

enum State {
    Inactive { reported: bool },
    Active,
}
pub struct PushHandler {
    pending_in_events: VecDeque<InEvent>,
    pending_out_events: VecDeque<OutEvent>,
    inbound: Option<PendingInbound>,
    outbound: Option<OutboundState>,
    state: State,
}
impl PushHandler {
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
                warn!(
                    "Error occurred when negotiating protocol {}: {:?}",
                    PUSH_PROTOCOL_NAME, e
                )
            }
        }
    }
}
impl Default for PushHandler {
    fn default() -> Self {
        Self {
            pending_in_events: Default::default(),
            pending_out_events: Default::default(),
            inbound: Default::default(),
            outbound: Default::default(),
            state: State::Active,
        }
    }
}

impl ConnectionHandler for PushHandler {
    type FromBehaviour = InEvent;
    type ToBehaviour = OutEvent;
    type Error = Error;
    type InboundProtocol = inbound_upgrade::Upgrade;
    type OutboundProtocol = outbound_upgrade::Upgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(inbound_upgrade::Upgrade, ())
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        self.pending_in_events.push_front(event);
    }
    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        KeepAlive::Yes
    }
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
            Self::Error,
        >,
    > {
        match self.state {
            State::Inactive { reported: true } => {
                return Poll::Pending;
            }
            State::Inactive { reported: false } => {
                self.state = State::Inactive { reported: true };
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(OutEvent::Unsupported));
            }
            State::Active => {}
        }
        if let Some(fut) = self.inbound.as_mut() {
            match fut.poll_unpin(cx) {
                Poll::Pending => {}
                Poll::Ready(Err(e)) => {
                    self.pending_out_events
                        .push_front(OutEvent::Error(Error::IO(e)));
                    self.inbound = None;
                }
                Poll::Ready(Ok((stream, bytes))) => {
                    let packet = match serde_json::from_slice::<Packet>(&bytes) {
                        Ok(packet) => packet,
                        Err(e) => {
                            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(OutEvent::Error(
                                Error::Corrupted(e, bytes),
                            )))
                        }
                    };
                    self.inbound = Some(protocol::recv(stream).boxed());
                    match packet {
                        Packet::Msg(msg) => {
                            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(OutEvent::Message(
                                msg,
                            )))
                        }
                    }
                }
            }
        }
        loop {
            if let Some(ev) = self.pending_out_events.pop_back() {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
            }
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, callback)) => match task.poll_unpin(cx) {
                    Poll::Pending => {
                        self.outbound = Some(OutboundState::Busy(task, callback));
                        break;
                    }
                    Poll::Ready(Ok((stream, _rtt))) => {
                        // TODO: implement more functionality
                        self.outbound = Some(OutboundState::Idle(stream));
                    }
                    Poll::Ready(Err(e)) => {
                        print!("{}", e);
                        self.pending_out_events
                            .push_front(OutEvent::Error(Error::IO(e)))
                    }
                },
                Some(OutboundState::Idle(stream)) => match self.pending_in_events.pop_back() {
                    Some(ev) => {
                        let (inner, callback) = ev.into_inner();
                        let bytes = match inner {
                            PushType::Msg(msg) => serde_json::to_vec(&Packet::Msg(msg)).unwrap(),
                        };
                        self.outbound = Some(OutboundState::Busy(
                            protocol::send(stream, bytes).boxed(),
                            callback,
                        ));
                        break;
                    }
                    None => {
                        self.outbound = Some(OutboundState::Idle(stream));
                        break;
                    }
                },
                Some(OutboundState::OpenStream) => {
                    self.outbound = Some(OutboundState::OpenStream);
                    break;
                }
                None => {
                    self.outbound = Some(OutboundState::OpenStream);
                    let protocol = SubstreamProtocol::new(outbound_upgrade::Upgrade, ());
                    return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                        protocol,
                    });
                }
            }
        }
        Poll::Pending
    }
    fn on_connection_event(
        &mut self,
        event: libp2p::swarm::handler::ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(FullyNegotiatedInbound {
                protocol: stream,
                ..
            }) => {
                self.inbound = Some(protocol::recv(stream).boxed());
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol: stream,
                ..
            }) => self.outbound = Some(OutboundState::Idle(stream)),
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
            ConnectionEvent::LocalProtocolsChange(_) => {}
            ConnectionEvent::RemoteProtocolsChange(_) => {}
        }
    }
}

type PendingInbound = BoxFuture<'static, Result<(Stream, Vec<u8>), io::Error>>;
type PendingSend = BoxFuture<'static, Result<(Stream, Duration), io::Error>>;

enum OutboundState {
    OpenStream,
    Idle(Stream),
    Busy(PendingSend, CallbackSender),
}
