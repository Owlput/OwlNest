use super::*;
use owlnest_macro::handle_callback_sender;
use owlnest_prelude::behaviour_prelude::*;
use std::collections::{HashSet, VecDeque};
use tracing::info;

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
        self.in_events.push_back(msg)
    }
    pub fn on_disconnect(&mut self, info: &ConnectionClosed) {
        if info.remaining_established < 1 {
            self.connected_peers.remove(&info.peer_id);
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = handler::Handler;
    type ToSwarm = OutEvent;

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: <Self::ConnectionHandler as ConnectionHandler>::ToBehaviour,
    ) {
        self.on_handler_event(event, peer_id, connection_id)
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<super::OutEvent, handler::FromBehaviourEvent>> {
        if let Some(ev) = self.out_events.pop_front() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.handle_in_events() {
            return Poll::Ready(ev);
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        if let FromSwarm::ConnectionClosed(info) = event {
            self.on_disconnect(&info)
        }
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(handler::Handler::new(self.config.clone()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(handler::Handler::new(self.config.clone()))
    }
}

impl Behaviour {
    #[inline]
    fn on_handler_event(
        &mut self,
        event: handler::ToBehaviourEvent,
        peer_id: PeerId,
        connection_id: ConnectionId,
    ) {
        use handler::ToBehaviourEvent::*;
        match event {
            IncomingMessage(bytes) => match serde_json::from_slice::<Message>(&bytes) {
                Ok(msg) => {
                    trace!("Incoming message from {}: {}", peer_id, msg.msg);
                    self.out_events.push_back(OutEvent::IncomingMessage {
                        from: msg.from,
                        msg,
                    })
                }
                Err(e) => {
                    self.out_events
                        .push_back(OutEvent::Error(super::Error::UnrecognizedMessage(format!(
                            "Unrecognized message: {}, raw data: {}",
                            e,
                            String::from_utf8_lossy(&bytes)
                        ))))
                }
            },
            Error(e) => {
                info!(
                    "Error occurred on peer {}:{:?}: {:#?}",
                    peer_id, connection_id, e
                );
                self.out_events.push_back(OutEvent::Error(e));
            }
            InboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::InboundNegotiated(peer_id));
                trace!(
                    "Successfully negotiated inbound connection from peer {}",
                    peer_id
                )
            }
            OutboundNegotiated => {
                self.out_events
                    .push_back(OutEvent::OutboundNegotiated(peer_id));
                self.connected_peers.insert(peer_id);
                trace!(
                    "Successfully negotiated outbound connection from peer {}",
                    peer_id
                )
            }
            Unsupported => {
                self.out_events.push_back(OutEvent::Unsupported(peer_id));
                trace!("Peer {} doesn't support {}", peer_id, PROTOCOL_NAME)
            }
            SendResult(result, id) => self.out_events.push_back(OutEvent::SendResult(result, id)),
        }
    }
    #[inline]
    fn handle_in_events(&mut self) -> Option<ToSwarm<OutEvent, handler::FromBehaviourEvent>> {
        while let Some(ev) = self.in_events.pop_front() {
            trace!("Received event {:#?}", ev);
            use InEvent::*;
            match ev {
                SendMessage(target, msg, id) => {
                    let ev = if self.connected_peers.contains(&target) {
                        ToSwarm::NotifyHandler {
                            peer_id: target,
                            handler: NotifyHandler::Any,
                            event: handler::FromBehaviourEvent::PostMessage(msg, id),
                        }
                    } else {
                        ToSwarm::GenerateEvent(OutEvent::SendResult(
                            Err(error::SendError::PeerNotFound(target)),
                            id,
                        ))
                    };
                    return Some(ev);
                }
                ListConnected(callback) => {
                    handle_callback_sender!(self.connected_peers.iter().copied().collect() => callback);
                }
            }
        }
        None
    }
}
