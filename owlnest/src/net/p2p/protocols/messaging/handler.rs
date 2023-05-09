use super::*;
use futures::{future::BoxFuture, FutureExt};
use futures_timer::Delay;
use libp2p::core::{
    upgrade::{NegotiationError, ReadyUpgrade},
    UpgradeError,
};
use libp2p::swarm::{
    handler::{ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound},
    ConnectionHandler, ConnectionHandlerEvent, ConnectionHandlerUpgrErr, NegotiatedSubstream,
    SubstreamProtocol,
};
use std::{collections::VecDeque, io, task::Poll, time::Duration};
use tracing::warn;

#[derive(Debug)]
pub enum InEvent {
    PostMessage(Message, oneshot::Sender<BehaviourOpResult>),
}
#[derive(Debug)]
pub enum OutEvent {
    IncomingMessage(Vec<u8>),
    Error(Error),
    Unsupported,
    InboundNegotiated,
    OutboundNegotiated,
}

pub enum State {
    Inactive { reported: bool },
    Active,
}

pub struct Handler {
    state: State,
    pending_in_events: VecDeque<InEvent>,
    pending_out_events: VecDeque<OutEvent>,
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
            ConnectionHandlerUpgrErr::Upgrade(UpgradeError::Select(NegotiationError::Failed)) => {
                self.state = State::Inactive { reported: false };
                return;
            }
            e => {
                warn!(
                    "Error occurred when negotiating protocol {}: {:?}",
                    protocol::PROTOCOL_NAME,
                    e
                )
            }
        }
    }
}

impl ConnectionHandler for Handler {
    type InEvent = InEvent;
    type OutEvent = OutEvent;
    type Error = Error;
    type InboundProtocol = ReadyUpgrade<&'static str>;
    type OutboundProtocol = ReadyUpgrade<&'static str>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();
    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ())
    }
    fn on_behaviour_event(&mut self, event: Self::InEvent) {
        debug!("Received event {:#?}", event);
        self.pending_in_events.push_front(event)
    }
    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        libp2p::swarm::KeepAlive::Yes
    }
    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        match self.state {
            State::Inactive { reported: true } => return Poll::Pending,
            State::Inactive { reported: false } => {
                self.state = State::Inactive { reported: true };
                return Poll::Ready(ConnectionHandlerEvent::Custom(OutEvent::Unsupported));
            }
            State::Active => {}
        };
        if let Some(fut) = self.inbound.as_mut() {
            match fut.poll_unpin(cx) {
                Poll::Pending => {}
                Poll::Ready(Err(e)) => {
                    let error = Error::IO(format!("IO Error: {:?}", e));
                    self.pending_out_events.push_front(OutEvent::Error(error));
                    self.inbound = None;
                }
                Poll::Ready(Ok((stream, bytes))) => {
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                    let event = ConnectionHandlerEvent::Custom(OutEvent::IncomingMessage(bytes));
                    return Poll::Ready(event);
                }
            }
        }
        loop {
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, callback, mut timer)) => {
                    match task.poll_unpin(cx) {
                        Poll::Pending => {
                            if timer.poll_unpin(cx).is_ready() {
                                let result =
                                    BehaviourOpResult::Messaging(OpResult::Error(Error::Timeout));
                                match callback.send(result) {
                                    Ok(_) => {}
                                    Err(res) => warn!("Failed to send callback {:?}", res),
                                };
                            } else {
                                // Put the future back
                                self.outbound = Some(OutboundState::Busy(task, callback, timer));
                                // End the loop because the outbound is busy
                                break;
                            }
                        }
                        // Ready
                        Poll::Ready(Ok((stream, rtt))) => {
                            let result =
                                BehaviourOpResult::Messaging(OpResult::SuccessfulPost(rtt));
                            match callback.send(result) {
                                Ok(_) => {}
                                Err(res) => warn!("Failed to send callback {:?}", res),
                            };
                            // Free the outbound
                            self.outbound = Some(OutboundState::Idle(stream));
                        }
                        // Ready but resolved to an error
                        Poll::Ready(Err(e)) => {
                            let result = BehaviourOpResult::Messaging(OpResult::Error(Error::IO(
                                format!("IO Error: {:?}", e),
                            )));
                            match callback.send(result) {
                                Ok(_) => {}
                                Err(res) => warn!("Failed to send result to callback: {:?}", res),
                            }
                        }
                    }
                }
                // Outbound is free, get the next message sent
                Some(OutboundState::Idle(stream)) => {
                    if let Some(ev) = self.pending_in_events.pop_back() {
                        match ev {
                            InEvent::PostMessage(msg, callback) => {
                                // Put Outbound into send state
                                self.outbound = Some(OutboundState::Busy(
                                    protocol::send(stream, msg.as_bytes()).boxed(),
                                    callback,
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
            if let Some(ev) = self.pending_out_events.pop_back() {
                return Poll::Ready(ConnectionHandlerEvent::Custom(ev));
            }
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
                    self.pending_out_events
                        .push_front(OutEvent::InboundNegotiated);
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                }
                ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                    protocol: stream,
                    ..
                }) => {
                    self.pending_out_events
                        .push_front(OutEvent::OutboundNegotiated);
                    self.outbound = Some(OutboundState::Idle(stream));
                }
                ConnectionEvent::DialUpgradeError(e) => {
                    self.on_dial_upgrade_error(e);
                }
                ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
            }
    }
}

type PendingVerf = BoxFuture<'static, Result<(NegotiatedSubstream, Vec<u8>), io::Error>>;
type PendingSend = BoxFuture<'static, Result<(NegotiatedSubstream, Duration), io::Error>>;

enum OutboundState {
    OpenStream,
    Idle(NegotiatedSubstream),
    Busy(PendingSend, oneshot::Sender<BehaviourOpResult>, Delay),
}
