use futures::future::BoxFuture;
use libp2p::swarm::{
    ConnectionHandler, ConnectionHandlerUpgrErr, KeepAlive, NegotiatedSubstream, SubstreamProtocol,
};
use std::collections::{HashMap, VecDeque};
use tokio::sync::oneshot;

use super::*;

pub struct ExecHandler {
    in_event: VecDeque<InEvent>,
    out_event: VecDeque<OutEvent>,
    inbound: Option,
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
            ConnectionHandlerUpgrErr::Upgrade(UpgradeError::Select(NegotiationError::Failed)) => {
                self.state = State::Inactive { reported: false };
                return;
            }
            e => {
                warn!(
                    "Error occurred when negotiating protocol {}: {}",
                    String::from_utf8(PROTOCOL_NAME.to_vec()).unwrap(),
                    e
                )
            }
        }
    }
}

pub struct InEvent {
    op: Op,
    callback: oneshot::Sender<Result>,
}

pub enum OutEvent {
    IncomingOp(Op),
}

pub enum Error {}

impl ConnectionHandler for ExecHandler {
    type InEvent = InEvent;
    type OutEvent = OutEvent;
    type Error = Error;
    type InboundProtocol = inbound_exec::Upgrade;
    type OutboundProtocol = outbound_exec::Upgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = ();

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(inbound_exec::Upgrade, ())
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
            Self::OutEvent,
            Self::Error,
        >,
    > {
        todo!()
    }
}

type PendingVerf = BoxFuture<'static, Result<(NegotiatedSubstream, Vec<u8>), io::Error>>;
type PendingSend = BoxFuture<'static, Result<(NegotiatedSubstream, Duration), io::Error>>;

enum OutboundState {
    OpenStream,
    Idle(NegotiatedSubstream),
    Busy,
}
