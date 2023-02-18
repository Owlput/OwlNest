use super::*;

use std::{collections::VecDeque, io, task::Poll, time::Duration};

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

use super::{protocol::PROTOCOL_NAME, Config, Message};

#[derive(Debug)]
pub enum InEvent {
    PostMessage(Message, oneshot::Sender<OpResult>),
}
#[derive(Debug)]
pub enum OutEvent {
    IncomingMessage(Vec<u8>),
    InboundNegotiated,
    OutboundNegotiated,
    Error(Error),
    Unsupported,
    Dummy,
}

pub enum State {
    Inactive { reported: bool },
    Active,
}

pub struct Handler {
    state: State,
    inbox: VecDeque<InEvent>,
    pending_events: VecDeque<OutEvent>,
    timeout: Duration,
    inbound: Option<PendingVerf>,
    outbound: Option<OutboundState>,
}

impl Handler {
    pub fn new(config: Config) -> Self {
        Self {
            state: State::Active,
            inbox: VecDeque::new(),
            pending_events: VecDeque::new(),
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
                    String::from_utf8(PROTOCOL_NAME.to_vec()).unwrap(),
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
    type InboundProtocol = ReadyUpgrade<&'static [u8]>;
    type OutboundProtocol = ReadyUpgrade<&'static [u8]>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();
    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ())
    }
    fn on_behaviour_event(&mut self, event: Self::InEvent) {
        self.inbox.push_front(event)
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
            State::Inactive { reported: true } => {
                // When user is notified with a failing handshake
                return Poll::Pending;
            }
            State::Inactive { reported: false } => {
                // When user is not notified with a failing handshake
                self.state = State::Inactive { reported: true };
                // Make it notified
                return Poll::Ready(ConnectionHandlerEvent::Custom(OutEvent::Unsupported));
            }
            State::Active => {
                // Handshake success, nothing to do, proceed
            }
        }
        // We've confirmed that the remote supports this protocol
        // Check for inbund messages
        if let Some(fut) = self.inbound.as_mut() {
            // Poll the incoming future to see if it's ready
            match fut.poll_unpin(cx) {
                // The incoming message is not ready
                Poll::Pending => {
                    //Nothing to do, proceed
                }
                // The incoming future resolves to an error
                Poll::Ready(Err(e)) => {
                    self.pending_events
                        .push_front(OutEvent::Error(Error::IO(format!("IO Error: {:?}",e))));
                    // Free the inbound because there's no stream present
                    self.inbound = None;
                }
                // The incoming message is ready and resolves to the stream and message sent by the remote
                Poll::Ready(Ok((stream, bytes))) => {
                    // Keep trying to receive from remote
                    self.inbound = Some(super::protocol::recv(stream).boxed());
                    // Resolve the handler to an incoming message
                    return Poll::Ready(ConnectionHandlerEvent::Custom(OutEvent::IncomingMessage(
                        bytes,
                    )));
                    // This poll is over, waiting for the next call
                }
            }
        }
        // Incoming message has been processed, now for outgoing message
        loop {
            // Flushing the error queue
            if let Some(ev) = self.pending_events.pop_back() {
                return Poll::Ready(ConnectionHandlerEvent::Custom(ev));
            }
            // Check whether the outbound is ready
            match self.outbound.take() {
                // Outbound is waiting for send operation
                Some(OutboundState::Busy(mut task, callback, mut timer)) => {
                    match task.poll_unpin(cx) {
                        // Not ready
                        Poll::Pending => {
                            if timer.poll_unpin(cx).is_ready() {
                                match callback.send(OpResult::Error(Error::Timeout)){
                                    Ok(_)=>{},
                                    Err(res)=> warn!("Failed to send callback {:?}",res)
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
                            // Free the outbound
                            self.outbound = Some(OutboundState::Idle(stream));
                            match callback.send(OpResult::SuccessfulPost(rtt)){
                                Ok(_)=>{},
                                Err(res)=> warn!("Failed to send callback {:?}",res)
                            };
                            return Poll::Ready(ConnectionHandlerEvent::Custom(OutEvent::Dummy));
                            // This poll is over, waiting for the next call
                        }
                        // Ready but resolved to an error
                        Poll::Ready(Err(e)) => {
                            match callback.send(OpResult::Error(messaging::Error::IO(format!("IO Error: {:?}",e)))){
                                Ok(_)=>{}
                                Err(res)=>warn!("Failed to send result to callback: {:?}",res)
                            }
                        }
                    }
                }
                // Outbound is free, get the next message sent
                Some(OutboundState::Idle(stream)) => match self.inbox.pop_back() {
                    // Unsent message found
                    Some(ev) => {
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
                    }
                    None => {
                        // Put Outbound into idle state
                        self.outbound = Some(OutboundState::Idle(stream));
                        break;
                    }
                },
                // Outbound is waiting for a stream to open
                Some(OutboundState::OpenStream) => {
                    self.outbound = Some(OutboundState::OpenStream);
                    break;
                }
                // Outbound has no stream available
                // Also the default state end up here
                None => {
                    // Put outbound into waiting state
                    self.outbound = Some(OutboundState::OpenStream);
                    // construct a handshake
                    let protocol = SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ());
                    // Send the handshake requesting for negotiation
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
                info
                
            }) => {
                self.inbound = Some(super::protocol::recv(stream).boxed());
            }
            ConnectionEvent::FullyNegotiatedOutbound(FullyNegotiatedOutbound {
                protocol: stream,
                ..
            }) => self.outbound = Some(OutboundState::Idle(stream)),
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            ConnectionEvent::AddressChange(_) | ConnectionEvent::ListenUpgradeError(_) => {}
        }
    }
}

type PendingVerf = BoxFuture<'static, Result<(NegotiatedSubstream, Vec<u8>), io::Error>>;
type PendingSend = BoxFuture<
    'static,
    Result<
        (
            NegotiatedSubstream,
            Duration,
        ),
        io::Error,
    >,
>;

enum OutboundState {
    /// A new substream is being negotiated for the messaging protocol.
    OpenStream,
    /// The substream is idle, waiting for next message.
    Idle(NegotiatedSubstream),
    /// A message is being sent and the response awaited.
    Busy(PendingSend, oneshot::Sender<OpResult>, Delay),
}
