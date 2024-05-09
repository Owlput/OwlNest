use super::error::SendError;
use super::{protocol, Config, Error, Message, PROTOCOL_NAME};
use futures_timer::Delay;
use owlnest_prelude::handler_prelude::*;
use std::{collections::VecDeque, time::Duration};
use tracing::{debug, trace};

#[derive(Debug)]
pub enum FromBehaviourEvent {
    PostMessage(Message, u64),
}
#[derive(Debug)]
pub enum ToBehaviourEvent {
    IncomingMessage(Vec<u8>),
    SendResult(Result<Duration, SendError>, u64),
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
            StreamUpgradeError::NegotiationFailed => {
                self.state = State::Inactive { reported: false };
            }
            e => {
                let e = format!("{:?}", e);
                if !e.contains("Timeout") {
                    debug!(
                        "Error occurred when negotiating protocol {}: {:?}",
                        PROTOCOL_NAME, e
                    )
                }
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
        if let Some(fut) = self.inbound.as_mut() {
            let poll_result = fut.poll_unpin(cx);
            if let Poll::Ready(Ok((stream, bytes))) = poll_result {
                self.inbound = Some(super::protocol::recv(stream).boxed());
                let event = ConnectionHandlerEvent::NotifyBehaviour(
                    ToBehaviourEvent::IncomingMessage(bytes),
                );
                return Poll::Ready(event);
            }
            if let Poll::Ready(Err(e)) = poll_result {
                let error = Error::IO(format!("IO Error: {:?}", e));
                self.pending_out_events
                    .push_back(ToBehaviourEvent::Error(error));
                self.inbound = None;
            }
        }
        loop {
            if let Some(OutboundState::Busy(task, id, timer)) = self.outbound.as_mut() {
                let poll_result = task.poll_unpin(cx);
                if let Poll::Pending = poll_result {
                    trace!("outbound pending");
                    if timer.poll_unpin(cx).is_ready() {
                        return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                            ToBehaviourEvent::SendResult(Err(SendError::Timeout), *id),
                        )); // exit and drop the task(with negotiated stream)
                    }
                    break; // exit loop because of pending future, checking pending out events
                }
                if let Poll::Ready(Err(e)) = poll_result {
                    return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                        ToBehaviourEvent::Error(Error::IO(e.to_string())),
                    )); // exit because of error
                }
                if let Poll::Ready(Ok((stream, rtt))) = poll_result {
                    self.pending_out_events
                        .push_back(ToBehaviourEvent::SendResult(Ok(rtt), *id));
                    // Free the outbound
                    self.outbound = Some(OutboundState::Idle(stream));
                    // continue to see if there is any pending activity
                }
            }
            trace!("pending in events: {}", self.pending_in_events.len());
            if self.pending_in_events.is_empty() {
                break; // exit because of no pending activity
            }
            if let Some(OutboundState::Idle(stream)) = self.outbound.take() {
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
            // come back to poll the newly created future for wake-up
        }
        if let Some(ev) = self.pending_out_events.pop_front() {
            return Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(ev));
        }
        if let Some(OutboundState::OpenStream) = self.outbound {
            return Poll::Pending;
        }
        if let None = self.outbound {
            self.outbound = Some(OutboundState::OpenStream);
            let protocol = SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ());
            let event = ConnectionHandlerEvent::OutboundSubstreamRequest { protocol };
            return Poll::Ready(event);
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
    Busy(PendingSend, u64, Delay),
}
