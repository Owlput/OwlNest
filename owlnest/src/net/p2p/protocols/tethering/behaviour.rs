use crate::net::p2p::swarm;

use super::*;
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler};
use std::{
    collections::{HashSet, VecDeque},
    task::Poll,
};

pub struct Behaviour {
    config: Config,
    trusted_peer: HashSet<PeerId>,
    out_events: VecDeque<OutEvent>,
    in_events: VecDeque<InEvent>,
}

impl Behaviour {
    #[inline]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            trusted_peer: HashSet::new(),
            out_events: VecDeque::new(),
            in_events: VecDeque::new(),
        }
    }
    #[inline]
    pub fn push_op(&mut self, op: InEvent) {
        self.in_events.push_front(op)
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
            handler::OutEvent::IncomingOp(op) => {
                    if let Some(_peer) = self.trusted_peer.get(&peer_id) {
                        self.out_events.push_front(OutEvent::IncomingOp(op))
                    }
                }
            handler::OutEvent::IncomingPush(ev) => {
                if let Some(_peer) = self.trusted_peer.get(&peer_id) {
                    self.out_events.push_front(OutEvent::IncomingPush(ev))
                }
            }
            handler::OutEvent::Callback(msg) => {
                if let Some(_peer) = self.trusted_peer.get(&peer_id) {
                    self.out_events.push_front(OutEvent::IncomingPush(ev))
                }
            }
            handler::OutEvent::Error(e) => self.out_events.push_front(OutEvent::Error(e)),
            handler::OutEvent::Unsupported => {
                self.out_events.push_front(OutEvent::Unsupported(peer_id))
            }
            handler::OutEvent::InboundNegotiated => self
                .out_events
                .push_front(OutEvent::InboundNegotiated(peer_id)),
            handler::OutEvent::OutboundNegotiated => self
                .out_events
                .push_front(OutEvent::OutboundNegotiated(peer_id)),
            handler::OutEvent::Dummy => {}
            
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
            match ev {
                InEvent::LocalExec(op, callback) => match op {
                    Op::Trust(peer) => match self.trusted_peer.insert(peer) {
                        true => {
                            let send_res = callback.send(OpResult::Local(LocalOpResult::Ok));
                            handle_send_res(send_res);
                        }
                        false => {
                            let send_res =
                                callback.send(OpResult::Local(LocalOpResult::AlreadyTrusted));
                            handle_send_res(send_res);
                        }
                    },
                    Op::Untrust(peer) => match self.trusted_peer.remove(&peer) {
                        true => {
                            let send_res = callback.send(OpResult::Local(LocalOpResult::Ok));
                            handle_send_res(send_res);
                        }
                        false => {
                            let send_res = callback.send(OpResult::Local(LocalOpResult::NotFound));
                            handle_send_res(send_res);
                        }
                    },
                },
                InEvent::RemoteExec(to, op, callback) => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: handler::InEvent{
                            kind: handler::InEventKind::Op(op),
                            callback,
                        },
                    })
                }
                InEvent::Push(to,ev,callback)=>{
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: handler::InEvent{
                            kind: handler::InEventKind::Push(ev),
                            callback,
                        },
                    })
                }
            }
        }
        Poll::Pending
    }
}
#[inline]
fn handle_send_res(send_res: Result<(), OpResult>) {
    match send_res {
        Ok(_) => {}
        Err(op_res) => warn!("Failed to send callback {:?}", op_res),
    }
}
