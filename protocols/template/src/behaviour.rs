use super::*;
use owlnest_prelude::behaviour_prelude::*;
use std::collections::{HashSet, VecDeque};

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
        use handler::ToBehaviourEvent::*;
        match event {
            // Error(e) => {
            //     info!(
            //         "Error occurred on peer {}:{:?}: {:#?}",
            //         peer_id, connection_id, e
            //     );
            //     self.out_events.push_back(OutEvent::Error(e));
            // }
            // InboundNegotiated => {
            //     self.out_events
            //         .push_back(OutEvent::InboundNegotiated(peer_id));
            //     trace!(
            //         "Successfully negotiated inbound connection from peer {}",
            //         peer_id
            //     )
            // }
            // OutboundNegotiated => {
            //     self.out_events
            //         .push_back(OutEvent::OutboundNegotiated(peer_id));
            //     trace!(
            //         "Successfully negotiated outbound connection from peer {}",
            //         peer_id
            //     )
            // }
            // Unsupported => {
            //     self.out_events.push_back(OutEvent::Unsupported(peer_id));
            //     trace!("Peer {} doesn't support {}", peer_id, PROTOCOL_NAME)
            // }
            _ => todo!(),
        }
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<super::OutEvent, handler::FromBehaviourEvent>> {
        if let Some(ev) = self.out_events.pop_front() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_front() {
            // trace!("Received event {:#?}", ev);
            use InEvent::*;
            match ev {
                _ => todo!(),
            }
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        match &event {
            FromSwarm::ConnectionClosed(info) => {
                if info.remaining_established < 1 {
                    self.connected_peers.remove(&info.peer_id);
                }
            }
            _ => {}
        }
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        peer: PeerId,
        _addr: &Multiaddr,
        _role_override: Endpoint,
        _port_use: PortUse,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        self.connected_peers.insert(peer);
        Ok(handler::Handler::new(self.config.clone()))
    }
}
