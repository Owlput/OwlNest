use super::*;
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler};
use std::{
    collections::{HashSet, VecDeque},
    task::Poll,
};

pub struct Behaviour {
    config: Config,
    tethered_peer: HashSet<PeerId>,
    out_events: VecDeque<OutEvent>,
    in_events: VecDeque<InEvent>,
}

impl Behaviour {
    #[inline]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tethered_peer: HashSet::new(),
            out_events: VecDeque::new(),
            in_events: VecDeque::new(),
        }
    }
    #[inline]
    pub fn push_op(&mut self, op: InEvent) {
        self.in_events.push_front(op)
    }
    #[inline]
    pub fn trust_peer(&mut self, peer: PeerId) {
        self.tethered_peer.insert(peer);
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
        _connection_id: libp2p::core::connection::ConnectionId,
        event: <<Self::ConnectionHandler as libp2p::swarm::IntoConnectionHandler>::Handler as
            libp2p::swarm::ConnectionHandler>::OutEvent,
    ) {
        match event {
            handler::OutEvent::IncomingOp(bytes) => {
                match serde_json::from_slice::<TetherOps>(&bytes) {
                    Ok(op) => {
                        if let Some(_peer) = self.tethered_peer.get(&peer_id) {
                            self.out_events.push_front(OutEvent::IncomingOp {
                                from: peer_id,
                                inner: op,
                            })
                        }
                    }
                    Err(e) => {
                        self.out_events
                            .push_front(OutEvent::Error(Error::UnrecognizedOp(
                                e,
                                String::from_utf8(bytes),
                            )))
                    }
                }
            }
            handler::OutEvent::SuccessPost(stamp, rtt) => self
                .out_events
                .push_front(OutEvent::SuccessPost(peer_id, stamp, rtt)),
            handler::OutEvent::Error(e) => self.out_events.push_front(OutEvent::Error(e)),
            handler::OutEvent::Unsupported => self
                .out_events
                .push_front(OutEvent::Unsupported(peer_id)),
            handler::OutEvent::InboundNegotiated => self
                .out_events
                .push_front(OutEvent::InboundNegotiated(peer_id)),
            handler::OutEvent::OutboundNegotiated => self
                .out_events
                .push_front(OutEvent::OutboundNegotiated(peer_id)),
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters,
    ) -> std::task::Poll<
        libp2p::swarm::NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>,
    > {
        if let Some(ev) = self.out_events.pop_back() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_back() {
            return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                peer_id: ev.to,
                handler: NotifyHandler::Any,
                event: ev.inner,
            });
        }

        Poll::Pending
    }
}
