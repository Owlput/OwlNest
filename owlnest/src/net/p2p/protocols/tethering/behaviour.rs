use super::*;
use libp2p::swarm::{
    derive_prelude::Either, ConnectionHandler, ConnectionHandlerSelect, ConnectionId,
    NetworkBehaviour, NotifyHandler, ToSwarm,
};
use std::{
    collections::{HashSet, VecDeque},
    task::Poll,
};

pub struct Behaviour {
    #[allow(unused)]
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
    pub fn push_event(&mut self, ev: InEvent) {
        self.in_events.push_front(ev)
    }
    pub fn trust(&mut self, peer_id: PeerId) -> Result<(), HandleError> {
        if self.trusted_peer.insert(peer_id) {
            Ok(())
        } else {
            Err(HandleError::LocalExec(TetheringOpError::AlreadyTrusted))
        }
    }
    pub fn untrust(&mut self, peer_id: PeerId) -> Result<(), HandleError> {
        if self.trusted_peer.remove(&peer_id) {
            Ok(())
        } else {
            Err(HandleError::LocalExec(TetheringOpError::NotFound))
        }
    }
}

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler =
        ConnectionHandlerSelect<push::handler::PushHandler, exec::handler::ExecHandler>;
    type ToSwarm = OutEvent;

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::ToBehaviour,
    ) {
        let out_event = match event {
            Either::Right(ev) => map_exec_out_event(peer_id, ev),
            Either::Left(ev) => map_push_out_event(peer_id, ev),
        };
        self.out_events.push_front(out_event);
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters,
    ) -> Poll<
        ToSwarm<
            OutEvent,
            Either<subprotocols::push::handler::InEvent,subprotocols::exec::handler::InEvent, >,
        >,
    > {
        if let Some(ev) = self.out_events.pop_back() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_back() {
            let (op, handle_callback) = ev.into_inner();
            match op {
                Op::RemoteExec(peer_id, op, result_callback) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: Either::Right(exec::InEvent::new_exec(
                            op,
                            handle_callback,
                            result_callback,
                        )),
                    })
                }
                Op::RemoteCallback(peer_id, stamp, result) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: Either::Right(exec::InEvent::new_callback(
                            stamp,
                            result,
                            handle_callback,
                        )),
                    })
                }
                Op::Push(to, push_type) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: Either::Left(push::InEvent::new(push_type, handle_callback)),
                    })
                }
                Op::LocalExec(op) => {
                    let result = match op {
                        TetheringOp::Trust(peer_id) => self.trust(peer_id),
                        TetheringOp::Untrust(peer_id) => self.untrust(peer_id),
                    };
                    handle_callback
                        .send(BehaviourOpResult::Tethering(
                            result.map(|_| HandleOk::LocalExec(())),
                        ))
                        .unwrap();
                }
            }
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm<Self::ConnectionHandler>) {}

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(push::handler::PushHandler::default().select(exec::handler::ExecHandler::default()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(push::handler::PushHandler::default().select(exec::handler::ExecHandler::default()))
    }
}

fn map_exec_out_event(peer_id: PeerId, ev: exec::OutEvent) -> behaviour::OutEvent {
    match ev {
        exec::OutEvent::Exec(op, stamp) => behaviour::OutEvent::Exec(op, stamp),
        exec::OutEvent::Error(e) => behaviour::OutEvent::ExecError(e),
        exec::OutEvent::Unsupported => {
            println!("unsupported: exec");
            behaviour::OutEvent::Unsupported(peer_id, behaviour::Subprotocol::Exec)
        }
    }
}
fn map_push_out_event(peer_id: PeerId, ev: push::OutEvent) -> behaviour::OutEvent {
    match ev {
        push::OutEvent::Message(msg) => behaviour::OutEvent::IncomingNotification(msg),
        push::OutEvent::Error(e) => behaviour::OutEvent::PushError(e),
        push::OutEvent::Unsupported => {
            println!("unsupported: push");
            behaviour::OutEvent::Unsupported(peer_id, behaviour::Subprotocol::Push)
        }
    }
}
