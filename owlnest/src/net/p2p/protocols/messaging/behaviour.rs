use crate::net::p2p::protocols::messaging::OpResult;

use super::*;
use libp2p::swarm::{ConnectionId, NetworkBehaviour, NetworkBehaviourAction, NotifyHandler};
use libp2p::PeerId;
use std::collections::HashSet;
use std::{collections::VecDeque, task::Poll};
use tracing::debug;

pub struct Behaviour {
    config: Config,
    /// Pending events to emit to `Swarm`
    out_events: VecDeque<OutEvent>,
    /// Pending events to be processed by this `Behaviour`.
    in_events: VecDeque<InEvent>,
    /// A set for all connected peers.
    connected_peers: HashSet<PeerId>,
}

impl Behaviour {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            out_events: VecDeque::new(),
            in_events: VecDeque::new(),
            connected_peers: HashSet::new(),
        }
    }
    pub fn push_event(&mut self, msg: InEvent) {
        self.in_events.push_front(msg)
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = handler::Handler;
    type OutEvent = OutEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        handler::Handler::new(self.config.clone())
    }
    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::OutEvent,
    ) {
        match event {
            handler::OutEvent::IncomingMessage(bytes) => {
                match serde_json::from_slice::<Message>(&bytes) {
                    Ok(msg) => self.out_events.push_front(OutEvent::IncomingMessage {
                        from: msg.from,
                        msg,
                    }),
                    Err(e) => {
                        self.out_events
                            .push_front(OutEvent::Error(Error::UnrecognizedMessage(format!(
                                "Unrecognized message: {}, raw data: {}",
                                e,
                                String::from_utf8_lossy(&bytes)
                            ))))
                    }
                }
            }
            handler::OutEvent::Error(e) => self.out_events.push_front(OutEvent::Error(e)),
            handler::OutEvent::Unsupported => {
                self.out_events.push_front(OutEvent::Unsupported(peer_id))
            }
            handler::OutEvent::InboundNegotiated => self
                .out_events
                .push_front(OutEvent::InboundNegotiated(peer_id)),
            handler::OutEvent::OutboundNegotiated => {
                self.out_events
                    .push_front(OutEvent::OutboundNegotiated(peer_id.clone()));
                self.connected_peers.insert(peer_id);
            }
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters,
    ) -> Poll<NetworkBehaviourAction<super::OutEvent, handler::InEvent>> {
        if let Some(ev) = self.out_events.pop_back() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_back() {
            debug!("Received event {:#?}", ev);
            let InEvent { op, callback } = ev;

            match op {
                Op::SendMessage(target, msg) => {
                    if self.connected_peers.contains(&target) {
                        return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                            peer_id: target,
                            handler: NotifyHandler::Any,
                            event: handler::InEvent::PostMessage(msg, callback),
                        });
                    } else {
                        let result = BehaviourOpResult::Messaging(OpResult::Error(
                            Error::PeerNotFound(target),
                        ));
                        callback.send(result).unwrap();
                    }
                }
            }
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm<Self::ConnectionHandler>) {
    }
}
