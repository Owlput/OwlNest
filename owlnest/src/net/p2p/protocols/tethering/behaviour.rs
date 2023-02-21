use super::*;
use libp2p::swarm::{
    derive_prelude::EitherOutput, ConnectionHandler, ConnectionHandlerSelect, NetworkBehaviour,
    NetworkBehaviourAction, NotifyHandler,
};
use std::{
    collections::{HashSet, VecDeque},
    task::Poll,
};

pub struct Behaviour {
    #[allow(unused)]
    config:Config,
    trusted_peer: HashSet<PeerId>,
    out_events: VecDeque<OutEvent>,
    in_events: VecDeque<InEvent>,
}

impl Behaviour {
    #[inline]
    pub fn new(config:Config) -> Self {
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
    type ConnectionHandler =
        ConnectionHandlerSelect<exec::handler::ExecHandler, push::handler::PushHandler>;
    type OutEvent = OutEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        exec::handler::ExecHandler::default().select(push::handler::PushHandler::default())
    }
    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: libp2p::core::connection::ConnectionId,
        event: <<Self::ConnectionHandler as libp2p::swarm::IntoConnectionHandler>::Handler as
            libp2p::swarm::ConnectionHandler>::OutEvent,
    ) {
        let out_event = match event {
            EitherOutput::First(ev) => map_exec_out_event(peer_id, ev),
            EitherOutput::Second(ev) => map_push_out_event(peer_id, ev),
        };
        self.out_events.push_front(out_event);
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
                    TetheringOp::Trust(peer) => {
                        if self.trusted_peer.insert(peer) {
                            callback.send(TetheringOpResult::Ok).unwrap();
                        } else {
                            callback.send(TetheringOpResult::AlreadyTrusted).unwrap()
                        }
                    }
                    TetheringOp::Untrust(peer) => {
                        if self.trusted_peer.remove(&peer) {
                            callback.send(TetheringOpResult::Ok).unwrap()
                        } else {
                            callback
                                .send(TetheringOpResult::Err(TetheringOpError::NotFound))
                                .unwrap()
                        }
                    }
                },
                InEvent::RemoteExec(peer_id, op, handle_callback, result_callback) => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: EitherOutput::First(exec::InEvent::new_exec(
                            op,
                            handle_callback,
                            result_callback,
                        )),
                    })
                }
                InEvent::RemoteCallback(peer_id, stamp, result, handle_callback) => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: EitherOutput::First(exec::InEvent::new_callback(
                            stamp,
                            result,
                            handle_callback,
                        )),
                    })
                }
                InEvent::Push(to, push_type, callback) => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: EitherOutput::Second(push::InEvent::new(push_type, callback)),
                    })
                }
            }
        }
        Poll::Pending
    }
}

fn map_exec_out_event(peer_id: PeerId, ev: exec::OutEvent) -> behaviour::OutEvent {
    match ev {
        exec::OutEvent::Exec(op, stamp) => behaviour::OutEvent::Exec(op, stamp),
        exec::OutEvent::Error(e) => behaviour::OutEvent::ExecError(e),
        exec::OutEvent::Unsupported => {
            behaviour::OutEvent::Unsupported(peer_id, behaviour::Subprotocol::Exec)
        }
    }
}
fn map_push_out_event(peer_id: PeerId, ev: push::OutEvent) -> behaviour::OutEvent {
    match ev {
        push::OutEvent::Message(msg) => behaviour::OutEvent::IncomingNotification(msg),
        push::OutEvent::Error(e) => behaviour::OutEvent::PushError(e),
        push::OutEvent::Unsupported => {
            behaviour::OutEvent::Unsupported(peer_id, behaviour::Subprotocol::Push)
        }
    }
}
