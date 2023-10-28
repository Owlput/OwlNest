use super::*;
use libp2p::swarm::{derive_prelude::*, NotifyHandler};
use owlnest_macro::connection_handler_select;
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
    pub(crate) fn push_event(&mut self, ev: InEvent) {
        self.in_events.push_front(ev)
    }
    pub fn trust(&mut self, peer_id: PeerId) -> Result<(), ()> {
        if self.trusted_peer.insert(peer_id) {
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn untrust(&mut self, peer_id: PeerId) -> Result<(), ()> {
        if self.trusted_peer.remove(&peer_id) {
            Ok(())
        } else {
            Err(())
        }
    }
}

connection_handler_select!(
    push=>Push:crate::net::p2p::protocols::tethering::subprotocols::push::handler::PushHandler,
    exec=>Exec:crate::net::p2p::protocols::tethering::subprotocols::exec::handler::ExecHandler,
);

impl NetworkBehaviour for Behaviour {
    type ConnectionHandler = handler::ConnectionHandlerSelect;
    type ToSwarm = OutEvent;

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        _connection_id: ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::ToBehaviour,
    ) {
        let out_event = match event {
            handler::ToBehaviourSelect::Exec(ev) => map_exec_out_event(peer_id, ev),
            handler::ToBehaviourSelect::Push(ev) => map_push_out_event(peer_id, ev),
        };
        self.out_events.push_front(out_event);
    }
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<OutEvent, handler::FromBehaviourSelect>> {
        if let Some(ev) = self.out_events.pop_back() {
            return Poll::Ready(ToSwarm::GenerateEvent(ev));
        }
        if let Some(ev) = self.in_events.pop_back() {
            use InEvent::*;
            match ev {
                RemoteExec(peer_id, op, id) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourSelect::Exec(exec::InEvent::new_exec(op, id)),
                    })
                }
                RemoteCallback(peer_id, stamp, result) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourSelect::Exec(exec::InEvent::new_callback(
                            stamp, result,
                        )),
                    })
                }
                Push(to, push_type, id) => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id: to,
                        handler: NotifyHandler::Any,
                        event: handler::FromBehaviourSelect::Push(push::InEvent::new(
                            push_type, id,
                        )),
                    })
                }
                LocalExec(op, id) => {
                    let result = match op {
                        TetheringOp::Trust(peer_id) => self.trust(peer_id),
                        TetheringOp::Untrust(peer_id) => self.untrust(peer_id),
                    };
                    self.out_events.push_back(OutEvent::LocalExec(result, id));
                }
            }
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
        Ok(handler::ConnectionHandlerSelect {
            push: push::PushHandler::default(),
            exec: exec::handler::ExecHandler::default(),
        })
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(handler::ConnectionHandlerSelect {
            push: push::PushHandler::default(),
            exec: exec::handler::ExecHandler::default(),
        })
    }
}

fn map_exec_out_event(peer_id: PeerId, ev: exec::OutEvent) -> behaviour::OutEvent {
    use exec::OutEvent::*;
    match ev {
        RemoteExecReq(op, stamp) => behaviour::OutEvent::Exec(op, stamp),
        Error(e) => behaviour::OutEvent::ExecError(e),
        Unsupported => {
            println!("unsupported: exec");
            behaviour::OutEvent::Unsupported(peer_id, behaviour::Subprotocol::Exec)
        }
        HandleResult(result, id) => behaviour::OutEvent::LocalExec(result, id),
        CallbackResult(result, id) => behaviour::OutEvent::RemoteExecResult(result, id),
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
