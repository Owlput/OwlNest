use super::error::SendError;
use super::{protocol, Config, Error, Message, PROTOCOL_NAME};
use futures_timer::Delay;
use owlnest_core::alias::Callback;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::handler_prelude::*;
use std::task::Context;
use std::{collections::VecDeque, time::Duration};
use tracing::{debug, trace};

#[derive(Debug)]
pub enum FromBehaviourEvent {
    PostMessage(Message, Callback<Result<Duration, SendError>>),
}
#[derive(Debug)]
pub enum ToBehaviourEvent {
    IncomingMessage(Vec<u8>),
    Error(Error),
    InboundNegotiated,
    OutboundNegotiated,
    Unsupported,
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
        trace!("Received event {:#?}", event);
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
        match self.state {
            State::Inactive { reported: true } => return Poll::Pending,
            State::Inactive { reported: false } => {
                self.state = State::Inactive { reported: true };
                trace!("Reporting inactivity");
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
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
        }
        if let Some(OutboundState::OpenStream) = self.outbound {
            return Poll::Pending;
        }
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
            ConnectionEvent::AddressChange(_) => {}
            ConnectionEvent::DialUpgradeError(e) => {
                self.on_dial_upgrade_error(e);
            }
            // ConnectionEvent::ListenUpgradeError(_) => {}
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
    Busy(PendingSend, Callback<Result<Duration, SendError>>, Delay),
}

type PollResult = ConnectionHandlerEvent<
    <Handler as ConnectionHandler>::OutboundProtocol,
    <Handler as ConnectionHandler>::OutboundOpenInfo,
    <Handler as ConnectionHandler>::ToBehaviour,
>;

impl Handler {
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
                let e = format!("{e:?}");
                if !e.contains("Timeout") {
                    debug!("Error occurred when negotiating protocol {PROTOCOL_NAME}: {e:?}",)
                }
            }
        }
    }
    #[inline]
    fn poll_inbound(&mut self, cx: &mut Context<'_>) -> Option<PollResult> {
        if let Some(fut) = self.inbound.as_mut() {
            let poll_result = fut.poll_unpin(cx);
            if let Poll::Ready(Ok((stream, bytes))) = poll_result {
                self.inbound = Some(super::protocol::recv(stream).boxed());
                let event = ConnectionHandlerEvent::NotifyBehaviour(
                    ToBehaviourEvent::IncomingMessage(bytes),
                );
                return Some(event);
            }
            if let Poll::Ready(Err(e)) = poll_result {
                let error = Error::IO(format!("IO Error: {e:?}"));
                self.pending_out_events
                    .push_back(ToBehaviourEvent::Error(error));
                self.inbound = None;
            }
        }
        None
    }
    #[inline]
    fn poll_outbound(&mut self, cx: &mut Context<'_>) -> Option<PollResult> {
        loop {
            match self.outbound.take() {
                Some(OutboundState::Busy(mut task, callback, mut timer)) => {
                    let poll_result = task.poll_unpin(cx);
                    if poll_result.is_pending() {
                        trace!("outbound pending");
                        if timer.poll_unpin(cx).is_ready() {
                            handle_callback_sender!(Err(SendError::Timeout)=>callback);
                            break;
                            // exit and drop the task(with negotiated stream)
                        }
                        self.outbound = Some(OutboundState::Busy(task, callback, timer));
                        break; // exit loop because of pending future, checking pending out events
                    }
                    if let Poll::Ready(Err(e)) = poll_result {
                        return Some(ConnectionHandlerEvent::NotifyBehaviour(
                            ToBehaviourEvent::Error(Error::IO(e.to_string())),
                        )); // exit because of error
                    }
                    if let Poll::Ready(Ok((stream, rtt))) = poll_result {
                        handle_callback_sender!(Ok(rtt)=>callback);
                        // Free the outbound
                        self.outbound = Some(OutboundState::Idle(stream));
                        // continue to see if there is any pending activity
                    }
                }
                Some(OutboundState::Idle(stream)) => {
                    if self.pending_in_events.is_empty() {
                        self.outbound = Some(OutboundState::Idle(stream));
                        break; // exit because of no pending activity
                    }
                    self.progress_in_events(stream)
                } // come back to poll the newly created future for wake-up
                Some(OutboundState::OpenStream) => {
                    self.outbound = Some(OutboundState::OpenStream);
                    break;
                }
                None => {
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
    /// Can panic if not guarded with checking if the queue is empty
    #[inline]
    fn progress_in_events(&mut self, stream: Stream) {
        let ev = self.pending_in_events.pop_front().expect("already handled");
        match ev {
            FromBehaviourEvent::PostMessage(msg, id) => {
                trace!("sending message: {}", msg.msg);
                // Put Outbound into send state
                self.outbound = Some(OutboundState::Busy(
                    protocol::send(stream, msg.as_bytes()).boxed(),
                    id,
                    Delay::new(self.timeout),
                ))
            }
        }
    }
}
