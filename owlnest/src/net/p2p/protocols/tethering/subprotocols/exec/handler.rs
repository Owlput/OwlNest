use super::{inbound_upgrade, op::Op, outbound_upgrade, protocol, OpResult};
use crate::net::p2p::handler_prelude::*;
use crate::net::p2p::protocols::tethering::subprotocols::EXEC_PROTOCOL_NAME;
use libp2p::Stream;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
};
use tracing::warn;

#[derive(Debug)]
pub struct InEvent {
    inner: Inner,
    id: u64,
}
impl InEvent {
    pub fn new_exec(op: Op, id: u64) -> Self {
        InEvent {
            inner: Inner::Exec(op, id),
            id,
        }
    }
    pub fn new_callback(id: u64, result: OpResult) -> Self {
        InEvent {
            inner: Inner::Callback(id, result),
            id: 0,
        }
    }
    pub fn into_inner(self) -> (Inner, u64) {
        (self.inner, self.id)
    }
}

#[derive(Debug)]
pub enum Inner {
    Exec(Op, u64),
    Callback(u64, OpResult),
}

/// Data structure be sent through the wire.
#[derive(Debug, Serialize, Deserialize)]
enum Packet {
    Op(Op, u64),
    Callback(OpResult, u64),
}

#[derive(Debug)]
pub enum OutEvent {
    RemoteExecReq(Op, u64),
    HandleResult(Result<(), ()>, u64),
    CallbackResult(OpResult, u64),
    Error(Error),
}

#[derive(Debug)]
pub enum Error {
    Corrupted(serde_json::Error, Vec<u8>),
    IO(io::Error),
    CallbackNotFound(u64, OpResult),
    CallbackFailed(OpResult),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupted(e, raw) => f.write_str(&format!(
                "Corrupted data when deserializing: {:?}, {:#?}",
                e, raw
            )),
            Error::IO(e) => f.write_str(&format!("IO error: {:#?}", e)),
            Error::CallbackFailed(result) => {
                f.write_str(&format!("Cannot send callback: {:?}", result))
            }
            Error::CallbackNotFound(stamp, result) => f.write_str(&format!(
                "Callback handle not found with stamp {}, content: {:?}",
                stamp, result
            )),
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

pub enum State {
    Inactive { reported: bool },
    Active,
}
pub struct ExecHandler {
    pending_in_events: VecDeque<InEvent>,
    pending_out_events: VecDeque<OutEvent>,
    pending_callbacks: HashSet<u64>,
    inbound: Option<PendingInbound>,
    outbound: Option<OutboundState>,
    state: State,
}
impl ExecHandler {
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
            libp2p_swarm::StreamUpgradeError::NegotiationFailed => {
                self.state = State::Inactive { reported: false };
            }
            e => {
                tracing::debug!(
                    "Error occurred when negotiating protocol {}: {}",
                    EXEC_PROTOCOL_NAME, e
                )
            }
        }
    }
}
impl Default for ExecHandler {
    fn default() -> Self {
        Self {
            pending_in_events: Default::default(),
            pending_out_events: Default::default(),
            pending_callbacks: Default::default(),
            inbound: Default::default(),
            outbound: Default::default(),
            state: State::Active,
        }
    }
}

impl ConnectionHandler for ExecHandler {
    type FromBehaviour = InEvent;
    type ToBehaviour = OutEvent;
    type InboundProtocol = inbound_upgrade::Upgrade;
    type OutboundProtocol = outbound_upgrade::Upgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(inbound_upgrade::Upgrade, ())
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        self.pending_in_events.push_back(event);
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
            State::Inactive { reported: true } => {
                return Poll::Pending;
            }
            State::Inactive { reported: false } => {
                self.state = State::Inactive { reported: true };
            }
            State::Active => {}
        }
        if let Some(fut) = self.inbound.as_mut() {
            match fut.poll_unpin(cx) {
                Poll::Pending => {}
                Poll::Ready(Err(e)) => {
                    self.pending_out_events
                        .push_back(OutEvent::Error(Error::IO(e)));
                    self.inbound = None;
                }
                Poll::Ready(Ok((stream, bytes))) => {
                    let packet = match serde_json::from_slice::<Packet>(&bytes) {
                        Ok(packet) => packet,
                        Err(e) => {
                            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                                OutEvent::Error(Error::Corrupted(e, bytes)),
                            ))
                        }
                    };
                    match packet {
                        Packet::Op(op, id) => {
                            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                                OutEvent::RemoteExecReq(op, id),
                            ))
                        }
                        Packet::Callback(result, id) => {
                            if self.pending_callbacks.remove(&id) {
                                self.pending_out_events
                                    .push_back(OutEvent::CallbackResult(result, id))
                            } else {
                                self.pending_out_events.push_back(OutEvent::Error(
                                    Error::CallbackNotFound(id, result),
                                ))
                            }
                        }
                    }
                    self.inbound = Some(protocol::recv(stream).boxed());
                }
            }
        }
        loop {
            if let Some(ev) = self.pending_out_events.pop_front() {
                return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
            }
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, id)) => match task.poll_unpin(cx) {
                    Poll::Pending => {
                        self.outbound = Some(OutboundState::Busy(task, id));
                        break;
                    }
                    Poll::Ready(Ok((stream, _))) => {
                        self.pending_out_events
                            .push_back(OutEvent::HandleResult(Ok(()), id));
                        self.outbound = Some(OutboundState::Idle(stream));
                    }
                    Poll::Ready(Err(e)) => {
                        print!("{}", e);
                        self.pending_out_events
                            .push_back(OutEvent::Error(Error::IO(e)))
                    }
                },
                Some(OutboundState::Idle(stream)) => match self.pending_in_events.pop_front() {
                    Some(ev) => {
                        let (inner, callback) = ev.into_inner();
                        let bytes = match inner {
                            Inner::Exec(op, id) => {
                                self.pending_callbacks.insert(id);
                                serde_json::to_vec(&Packet::Op(op, id)).unwrap()
                            }
                            Inner::Callback(stamp, result) => {
                                serde_json::to_vec(&Packet::Callback(result, stamp)).unwrap()
                            }
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
            ConnectionEvent::ListenUpgradeError(_)=>{}
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
            ConnectionEvent::LocalProtocolsChange(_) => {}
            ConnectionEvent::RemoteProtocolsChange(_) => {}
            _ => unimplemented!("New branch not covered")
        }
    }
}

type PendingInbound = BoxFuture<'static, Result<(Stream, Vec<u8>), io::Error>>;
type PendingSend = BoxFuture<'static, Result<(Stream, Duration), io::Error>>;

enum OutboundState {
    OpenStream,
    Idle(Stream),
    Busy(PendingSend, u64),
}
