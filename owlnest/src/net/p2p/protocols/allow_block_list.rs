use libp2p::{swarm::ConnectionDenied, PeerId};

pub mod behaviour {
    use super::{policy_peer_id::AllowPeerId, *};
    use libp2p::swarm::{NetworkBehaviour, ToSwarm};
    use std::{
        collections::VecDeque,
        task::{Poll, Waker},
    };

    pub struct Behaviour<P> {
        policy: P,
        pending_disconnect: VecDeque<PeerId>,
        waker: Option<Waker>,
    }
    impl<P> Behaviour<P> {
        pub fn new(policy: P) -> Behaviour<P> {
            Behaviour {
                policy,
                pending_disconnect: VecDeque::new(),
                waker: None,
            }
        }
    }
    impl<P> NetworkBehaviour for Behaviour<P>
    where
        P: policy_peer_id::Policy + 'static,
    {
        type ConnectionHandler = libp2p::swarm::dummy::ConnectionHandler;

        type ToSwarm = super::OutEvent;

        fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm) {}

        fn on_connection_handler_event(
            &mut self,
            _peer_id: libp2p::PeerId,
            _connection_id: libp2p::swarm::ConnectionId,
            _event: libp2p::swarm::THandlerOutEvent<Self>,
        ) {
            unreachable!()
        }

        fn handle_pending_outbound_connection(
            &mut self,
            _connection_id: libp2p::swarm::ConnectionId,
            maybe_peer: Option<PeerId>,
            _addresses: &[libp2p::Multiaddr],
            _effective_role: libp2p::core::Endpoint,
        ) -> Result<Vec<libp2p::Multiaddr>, ConnectionDenied> {
            if let Some(peer) = maybe_peer {
                self.policy.enforce(&peer)?;
            }
            Ok(Vec::new())
        }

        fn poll(
            &mut self,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<
            libp2p::swarm::ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>,
        > {
            if let Some(v) = self.pending_disconnect.pop_back() {
                return Poll::Ready(ToSwarm::CloseConnection {
                    peer_id: v,
                    connection: libp2p::swarm::CloseConnection::All,
                });
            }
            self.waker = Some(cx.waker().clone());
            Poll::Pending
        }

        fn handle_established_inbound_connection(
            &mut self,
            _connection_id: libp2p::swarm::ConnectionId,
            peer: PeerId,
            _local_addr: &libp2p::Multiaddr,
            _remote_addr: &libp2p::Multiaddr,
        ) -> Result<libp2p::swarm::THandler<Self>, ConnectionDenied> {
            self.policy.enforce(&peer)?;
            Ok(libp2p::swarm::dummy::ConnectionHandler)
        }

        fn handle_established_outbound_connection(
            &mut self,
            _connection_id: libp2p::swarm::ConnectionId,
            peer: PeerId,
            _addr: &libp2p::Multiaddr,
            _role_override: libp2p::core::Endpoint,
        ) -> Result<libp2p::swarm::THandler<Self>, ConnectionDenied> {
            self.policy.enforce(&peer)?;
            Ok(libp2p::swarm::dummy::ConnectionHandler)
        }
    }

    impl Behaviour<AllowPeerId> {
        pub fn add_peer(&mut self, peer: PeerId) -> bool {
            self.policy.add_peer(peer)
        }
        pub fn remove_peer(&mut self, peer: &PeerId) -> bool {
            if !self.policy.remove_peer(peer) {
                return false;
            }
            self.pending_disconnect.push_front(*peer);
            if let Some(waker) = self.waker.take() {
                waker.wake()
            }
            true
        }
    }
}
pub enum OutEvent {
    BannedPeer(PeerId),
}

pub mod policy_peer_id {
    use super::*;
    use std::{collections::HashSet, error::Error, fmt::Display};

    pub trait Policy {
        fn enforce(&self, peer: &PeerId) -> Result<(), ConnectionDenied>;
    }

    #[derive(Debug, Default)]
    pub struct AllowPeerId(HashSet<PeerId>);
    impl AllowPeerId {
        pub fn add_peer(&mut self, peer: PeerId) -> bool {
            self.0.insert(peer)
        }
        pub fn remove_peer(&mut self, peer: &PeerId) -> bool {
            self.0.remove(peer)
        }
        pub fn is_allowed(&self, peer: &PeerId) -> bool {
            self.0.contains(peer)
        }
    }
    impl Policy for AllowPeerId {
        fn enforce(&self, peer: &PeerId) -> Result<(), ConnectionDenied> {
            if self.is_allowed(peer) {
                Ok(())
            } else {
                Err(ConnectionDenied::new(NotAllowed))
            }
        }
    }
    #[derive(Debug)]
    pub struct NotAllowed;
    impl Display for NotAllowed {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("NotAllowed")
        }
    }
    impl Error for NotAllowed {}
}

pub mod policy_ip {
    use std::{collections::HashSet, net::IpAddr};

    use libp2p::swarm::ConnectionDenied;

    pub trait Policy {
        fn enforce(&self, addr: &IpAddr) -> Result<(), ConnectionDenied>;
    }

    pub struct AllowIp(HashSet<std::net::IpAddr>);
}
