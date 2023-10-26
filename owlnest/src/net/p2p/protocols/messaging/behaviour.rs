use super::*;
use libp2p::swarm::{ConnectionId, NetworkBehaviour, NotifyHandler, ToSwarm};
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
    type ToSwarm = OutEvent;

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::ToBehaviour,
    ) {
        use handler::ToBehaviourEvent::*;
        match event {
            IncomingMessage(bytes) => {
                match serde_json::from_slice::<Message>(&bytes) {
                    Ok(msg) => self.out_events.push_front(OutEvent::IncomingMessage {
                        from: msg.from,
                        msg,
                    }),
                    Err(e) => {
                        self.out_events
                            .push_front(OutEvent::Error(super::Error::UnrecognizedMessage(format!(
                                "Unrecognized message: {}, raw data: {}",
                                e,
                                String::from_utf8_lossy(&bytes)
                            ))))
                    }
                }
            }
            Error(e) => self.out_events.push_front(OutEvent::Error(e)),
            Unsupported => {
                self.out_events.push_back(OutEvent::Unsupported(peer_id))
            }
            InboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::InboundNegotiated(peer_id));
            }
            OutboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::OutboundNegotiated(peer_id));
            }
            SuccessfulSend(id)=>{
                self.out_events.push_back(OutEvent::SuccessfulSend(id))
            }
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<super::OutEvent, handler::FromBehaviourEvent>> {
        if let Some(ev) = self.out_events.pop_back() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_back() {
            debug!("Received event {:#?}", ev);
            use InEvent::*;
            match ev {
                SendMessage(target, msg, id) => {
                    if self.connected_peers.contains(&target) {
                        return Poll::Ready(ToSwarm::NotifyHandler {
                            peer_id: target,
                            handler: NotifyHandler::Any,
                            event: handler::FromBehaviourEvent::PostMessage(msg, id),
                        });
                    } else {
                        self.out_events.push_back(OutEvent::Error(Error::PeerNotFound(target)))
                    }
                }
            }
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm<Self::ConnectionHandler>) {}

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }
}
