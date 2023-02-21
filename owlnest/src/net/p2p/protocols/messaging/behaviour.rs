use crate::net::p2p::protocols::messaging::OpResult;

use super::{handler, Config, Error, Message,InEvent,OutEvent, Op};
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler};
use libp2p::PeerId;
use tracing::debug;
use std::collections::HashSet;
use std::{collections::VecDeque, task::Poll};

pub struct Behaviour {
    config: Config,
    out_events: VecDeque<OutEvent>,
    in_events: VecDeque<InEvent>,
    connected_peers:HashSet<PeerId>,
}

impl Behaviour {
    #[inline]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            out_events: VecDeque::new(),
            in_events: VecDeque::new(),
            connected_peers:HashSet::new(),
        }
    }
    #[inline]
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
        _connection_id: libp2p::core::connection::ConnectionId,
        event: <<Self::ConnectionHandler as libp2p::swarm::IntoConnectionHandler>::Handler as
            libp2p::swarm::ConnectionHandler>::OutEvent,
    ) {
        match event {
            handler::OutEvent::IncomingMessage(bytes) => {
                match serde_json::from_slice::<Message>(&bytes) {
                    Ok(msg) => self
                        .out_events
                        .push_front(OutEvent::IncomingMessage {
                            from: msg.from,
                            msg,
                        }),
                    Err(e) => self.out_events.push_front(OutEvent::Error(
                        Error::UnrecognizedMessage(format!("Unrecognized message: {}, raw data: {}",e,String::from_utf8_lossy(&bytes)),
                    ))),
                }
            }
            handler::OutEvent::Error(e) => self.out_events.push_front(OutEvent::Error(e)),
            handler::OutEvent::Unsupported => self
                .out_events
                .push_front(OutEvent::Unsupported(peer_id)),
            handler::OutEvent::InboundNegotiated => self
                .out_events
                .push_front(OutEvent::InboundNegotiated(peer_id)),
            handler::OutEvent::OutboundNegotiated => {self
                .out_events
                .push_front(OutEvent::OutboundNegotiated(peer_id.clone()));
                self.connected_peers.insert(peer_id);
            }
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
            debug!("Received event {:#?}",ev);
            let InEvent{op,callback} = ev;
            
            match op {
                Op::SendMessage(target,msg)=> {
                    if self.connected_peers.contains(&target){
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id: target,
                        handler: NotifyHandler::Any,
                        event: handler::InEvent::PostMessage(msg,callback),
                    })}else{
                        callback.send(OpResult::Error(Error::PeerNotFound(target))).unwrap();
                    }
                }
            }
        }
        Poll::Pending
    }
}
