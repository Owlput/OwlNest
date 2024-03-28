use super::{protocol, Error};
use crate::net::p2p::handler_prelude::*;
use futures_timer::Delay;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Duration};
use tracing::trace;

#[derive(Debug)]
pub enum FromBehaviour {
    QueryAdvertisedPeer,
    AnswerAdvertisedPeer(Vec<PeerId>),
    SetAdvertiseSelf(bool, u64),
}
#[derive(Debug)]
pub enum ToBehaviour {
    IncomingQuery,
    QueryAnswered(Vec<PeerId>),
    IncomingAdvertiseReq(bool),
    Error(Error),
}

pub enum State {
    Inactive { reported: bool },
    Active,
}

#[derive(Debug, Serialize, Deserialize)]
enum Packet {
    AdvertiseSelf(bool, u64),
    QueryAdvertisedPeer,
    AnswerAdvertisedPeer(Vec<PeerId>),
}
impl Packet {
    #[inline]
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}
impl Into<ToBehaviour> for Packet {
    fn into(self) -> ToBehaviour {
        match self {
            Packet::AdvertiseSelf(bool, _) => ToBehaviour::IncomingAdvertiseReq(bool),
            Packet::QueryAdvertisedPeer => ToBehaviour::IncomingQuery,
            Packet::AnswerAdvertisedPeer(result) => ToBehaviour::QueryAnswered(result),
        }
    }
}

pub struct Handler {
    state: State,
    pending_in_events: VecDeque<FromBehaviour>,
    pending_out_events: VecDeque<ToBehaviour>,
    timeout: Duration,
    inbound: Option<PendingVerf>,
    outbound: Option<OutboundState>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            state: State::Active,
            pending_in_events: VecDeque::new(),
            pending_out_events: VecDeque::new(),
            timeout: Duration::from_secs(20),
            inbound: None,
            outbound: None,
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
                tracing::debug!(
                    "Error occurred when negotiating protocol {}: {:?}",
                    protocol::PROTOCOL_NAME,
                    e
                )
            }
        }
    }
}

use libp2p::core::upgrade::ReadyUpgrade;
impl ConnectionHandler for Handler {
    type FromBehaviour = FromBehaviour;
    type ToBehaviour = ToBehaviour;
    type InboundProtocol = ReadyUpgrade<&'static str>;
    type OutboundProtocol = ReadyUpgrade<&'static str>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();
    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ())
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
            }
            State::Active => {}
        };
        if let Some(fut) = self.inbound.as_mut() {
            match fut.poll_unpin(cx) {
                Poll::Pending => {}
                Poll::Ready(Err(e)) => {
                    let error = Error::IO(format!("IO Error: {:?}", e));
                    self.pending_out_events.push_back(ToBehaviour::Error(error));
                    self.inbound = None;
                }
                Poll::Ready(Ok((stream, bytes))) => {
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                    match serde_json::from_slice::<Packet>(&bytes) {
                        Ok(packet) => self.pending_out_events.push_back(packet.into()),
                        Err(e) => self.pending_out_events.push_back(ToBehaviour::Error(
                            Error::UnrecognizedMessage(format!(
                                "Unrecognized message: {}, raw data: {}",
                                e,
                                String::from_utf8_lossy(&bytes)
                            )),
                        )),
                    }
                    let event = ConnectionHandlerEvent::NotifyBehaviour(ToBehaviour::IncomingQuery);
                    return Poll::Ready(event);
                }
            }
        }
        loop {
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, mut timer)) => {
                    match task.poll_unpin(cx) {
                        Poll::Pending => {
                            if timer.poll_unpin(cx).is_ready() {
                                self.pending_out_events
                                    .push_back(ToBehaviour::Error(Error::Timeout))
                            } else {
                                // Put the future back
                                self.outbound = Some(OutboundState::Busy(task, timer));
                                // End the loop because the outbound is busy
                                break;
                            }
                        }
                        // Ready
                        Poll::Ready(Ok((stream, rtt))) => {
                            trace!("Successful IO send with rtt of {}ms", rtt.as_millis());
                            // Free the outbound
                            self.outbound = Some(OutboundState::Idle(stream));
                        }
                        // Ready but resolved to an error
                        Poll::Ready(Err(e)) => {
                            self.pending_out_events
                                .push_back(ToBehaviour::Error(Error::IO(format!(
                                    "IO Error: {:?}",
                                    e
                                ))));
                        }
                    }
                }
                // Outbound is free, get the next message sent
                Some(OutboundState::Idle(stream)) => {
                    if let Some(ev) = self.pending_in_events.pop_front() {
                        use FromBehaviour::*;
                        match ev {
                            QueryAdvertisedPeer => {
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(stream, Packet::QueryAdvertisedPeer.as_bytes())
                                        .boxed(),
                                    Delay::new(self.timeout),
                                ))
                            }
                            AnswerAdvertisedPeer(result) => {
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(
                                        stream,
                                        Packet::AnswerAdvertisedPeer(result).as_bytes(),
                                    )
                                    .boxed(),
                                    Delay::new(self.timeout),
                                ))
                            }
                            SetAdvertiseSelf(state, id) => {
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(
                                        stream,
                                        Packet::AdvertiseSelf(state, id).as_bytes(),
                                    )
                                    .boxed(),
                                    Delay::new(self.timeout),
                                ))
                            }
                        }
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
        if let Some(ev) = self.pending_out_events.pop_front() {
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
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol: stream,
                ..
            }) => {
                self.outbound = Some(OutboundState::Idle(stream));
            }
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
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
    Busy(PendingSend, Delay),
}
