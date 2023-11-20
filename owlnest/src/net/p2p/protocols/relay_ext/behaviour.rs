use super::*;
use libp2p::swarm::{ConnectionId, NetworkBehaviour, NotifyHandler, ToSwarm};
use libp2p::PeerId;
use std::collections::HashSet;
use std::{collections::VecDeque, task::Poll};
use tracing::{debug, trace};

pub struct Behaviour {
    /// Pending events to emit to `Swarm`
    pending_out_events: VecDeque<OutEvent>,
    /// Pending events to be processed by this `Behaviour`.
    in_events: VecDeque<InEvent>,
    /// A set for all connected peers.
    advertised_peers: HashSet<PeerId>,
    pending_query_answer: VecDeque<(PeerId, ConnectionId)>,
    is_providing: bool,
}

impl Behaviour {
    pub fn new() -> Self {
        Self {
            pending_out_events: VecDeque::new(),
            in_events: VecDeque::new(),
            advertised_peers: HashSet::new(),
            pending_query_answer: VecDeque::new(),
            is_providing: false,
        }
    }
    pub fn push_event(&mut self, msg: InEvent) {
        self.in_events.push_front(msg)
    }
    pub fn is_advertising(&self, peer: &PeerId) -> bool {
        self.advertised_peers.contains(peer)
    }
    pub fn advertised_peers(&self) -> &HashSet<PeerId> {
        &self.advertised_peers
    }
    pub fn set_provider_status(&mut self, status: bool) {
        self.is_providing = status
    }
    pub fn get_provider_status(&self) -> bool {
        self.is_providing
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = handler::Handler;
    type ToSwarm = OutEvent;

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::ToBehaviour,
    ) {
        use handler::ToBehaviourEvent::*;
        match event {
            IncomingQuery => {
                self.pending_query_answer
                    .push_back((peer_id, connection_id));
            }
            IncomingAdvertiseReq(bool) => {
                if bool {
                    if self.advertised_peers.insert(peer_id) {
                        debug!("Now advertising peer {}", peer_id);
                    }
                } else {
                    if self.advertised_peers.remove(&peer_id) {
                        debug!("Stopped advertising peer {}", peer_id);
                    }
                };
            }
            QueryAnswered(result) => self.pending_out_events.push_back(OutEvent::QueryAnswered {
                from: peer_id,
                list: result,
            }),
            Error(e) => self.pending_out_events.push_front(OutEvent::Error(e)),
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<super::OutEvent, handler::FromBehaviourEvent>> {
        if let Some((peer_id, connection_id)) = self.pending_query_answer.pop_front() {
            if self.is_providing {
                return Poll::Ready(ToSwarm::NotifyHandler {
                    peer_id,
                    handler: NotifyHandler::One(connection_id),
                    event: handler::FromBehaviourEvent::AnswerAdvertisedPeer(
                        self.advertised_peers.iter().cloned().collect(),
                    ),
                });
            }
        }
        if let Some(ev) = self.in_events.pop_front() {
            trace!("Received event {:#?}", ev);
            use InEvent::*;
            match ev {
                QueryAdvertisedPeer(relay) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id: relay,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourEvent::QueryAdvertisedPeer,
                    })
                }
                QueryProviderState =>{
                    return Poll::Ready(ToSwarm::GenerateEvent(OutEvent::ProviderState(self.is_providing))
                    )
                }
                StartAdvertiseSelf(relay) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id: relay,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourEvent::StartAdvertiseSelf,
                    })
                }
                StopAdvertiseSelf(relay) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id: relay,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourEvent::StopAdvertiseSelf,
                    });
                }
                SetProviderState(status) => {
                    self.set_provider_status(status);
                    return Poll::Ready(ToSwarm::GenerateEvent(OutEvent::ProviderState(
                        status,
                    )));
                }
            }
        }
        if let Some(ev) = self.pending_out_events.pop_front() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm) {}

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(handler::Handler::new())
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(handler::Handler::new())
    }
}
