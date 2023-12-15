use std::num::NonZeroU32;

use super::*;
use libp2p::{swarm::derive_prelude::ListenerId, Multiaddr};
use libp2p_swarm::ConnectionId;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// A connection to the given peer has been opened.
    ConnectionEstablished {
        /// Identity of the peer that we have connected to.
        peer_id: PeerId,
        /// Identifier of the connection.
        connection_id: ConnectionId,
        /// Endpoint of the connection that has been opened.
        endpoint: ConnectedPoint,
        /// Number of established connections to this peer, including the one that has just been
        /// opened.
        num_established: NonZeroU32,
        /// [`Some`] when the new connection is an outgoing connection.
        /// Addresses are dialed concurrently. Contains the addresses and errors
        /// of dial attempts that failed before the one successful dial.
        concurrent_dial_errors: String,
        /// How long it took to establish this connection
        established_in: std::time::Duration,
    },
    /// A connection with the given peer has been closed,
    /// possibly as a result of an error.
    ConnectionClosed {
        /// Identity of the peer that we have connected to.
        peer_id: PeerId,
        /// Identifier of the connection.
        connection_id: ConnectionId,
        /// Endpoint of the connection that has been closed.
        endpoint: ConnectedPoint,
        /// Number of other remaining connections to this same peer.
        num_established: u32,
        /// Reason for the disconnection, if it was not a successful
        /// active close.
        cause: String,
    },
    /// A new connection arrived on a listener and is in the process of protocol negotiation.
    ///
    /// A corresponding [`ConnectionEstablished`](SwarmEvent::ConnectionEstablished) or
    /// [`IncomingConnectionError`](SwarmEvent::IncomingConnectionError) event will later be
    /// generated for this connection.
    IncomingConnection {
        /// Identifier of the connection.
        connection_id: ConnectionId,
        /// Local connection address.
        /// This address has been earlier reported with a [`NewListenAddr`](SwarmEvent::NewListenAddr)
        /// event.
        local_addr: Multiaddr,
        /// Address used to send back data to the remote.
        send_back_addr: Multiaddr,
    },
    /// An error happened on an inbound connection during its initial handshake.
    ///
    /// This can include, for example, an error during the handshake of the encryption layer, or
    /// the connection unexpectedly closed.
    IncomingConnectionError {
        /// Identifier of the connection.
        connection_id: ConnectionId,
        /// Local connection address.
        /// This address has been earlier reported with a [`NewListenAddr`](SwarmEvent::NewListenAddr)
        /// event.
        local_addr: Multiaddr,
        /// Address used to send back data to the remote.
        send_back_addr: Multiaddr,
        /// The error that happened.
        error: String,
    },
    /// An error happened on an outbound connection.
    OutgoingConnectionError {
        /// Identifier of the connection.
        connection_id: ConnectionId,
        /// If known, [`PeerId`] of the peer we tried to reach.
        peer_id: Option<PeerId>,
        /// Error that has been encountered.
        error: String,
    },
    /// One of our listeners has reported a new local listening address.
    NewListenAddr {
        /// The listener that is listening on the new address.
        listener_id: ListenerId,
        /// The new address that is being listened on.
        address: Multiaddr,
    },
    /// One of our listeners has reported the expiration of a listening address.
    ExpiredListenAddr {
        /// The listener that is no longer listening on the address.
        listener_id: ListenerId,
        /// The expired address.
        address: Multiaddr,
    },
    /// One of the listeners gracefully closed.
    ListenerClosed {
        /// The listener that closed.
        listener_id: ListenerId,
        /// The addresses that the listener was listening on. These addresses are now considered
        /// expired, similar to if a [`ExpiredListenAddr`](SwarmEvent::ExpiredListenAddr) event
        /// has been generated for each of them.
        addresses: Vec<Multiaddr>,
        /// Reason for the closure. Contains `Ok(())` if the stream produced `None`, or `Err`
        /// if the stream produced an error.
        reason: String,
    },
    /// One of the listeners reported a non-fatal error.
    ListenerError {
        /// The listener that errored.
        listener_id: ListenerId,
        /// The listener error.
        error: String,
    },
    /// A new dialing attempt has been initiated by the [`NetworkBehaviour`]
    /// implementation.
    ///
    /// A [`ConnectionEstablished`](SwarmEvent::ConnectionEstablished) event is
    /// reported if the dialing attempt succeeds, otherwise a
    /// [`OutgoingConnectionError`](SwarmEvent::OutgoingConnectionError) event
    /// is reported.
    Dialing {
        /// Identity of the peer that we are connecting to.
        peer_id: Option<PeerId>,

        /// Identifier of the connection.
        connection_id: ConnectionId,
    },
    /// We have discovered a new candidate for an external address for us.
    NewExternalAddrCandidate { address: Multiaddr },
    /// An external address of the local node was confirmed.
    ExternalAddrConfirmed { address: Multiaddr },
    /// An external address of the local node expired, i.e. is no-longer confirmed.
    ExternalAddrExpired { address: Multiaddr },
}
impl SwarmEvent{
    pub fn try_from_ref_swarm_event(ev:&super::SwarmEvent)->Result<Self,()>{
        let ev = match ev {
            libp2p::swarm::SwarmEvent::Behaviour(_) => return Err(()),
            libp2p::swarm::SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
                connection_id,
            } => Self::ConnectionEstablished {
                peer_id:*peer_id,
                endpoint:endpoint.clone(),
                num_established:num_established.clone(),
                concurrent_dial_errors: format!("{:?}", concurrent_dial_errors),
                established_in:established_in.clone(),
                connection_id:*connection_id
            },
            libp2p::swarm::SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause,
                connection_id,
            } => Self::ConnectionClosed {
                peer_id:*peer_id,
                endpoint:endpoint.clone(),
                num_established:*num_established,
                cause: format!("{:?}", cause),
                connection_id:*connection_id,
            },
            libp2p::swarm::SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
                connection_id,
            } => Self::IncomingConnection {
                local_addr:local_addr.clone(),
                send_back_addr:send_back_addr.clone(),
                connection_id:*connection_id,
            },
            libp2p::swarm::SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
                connection_id,
            } => Self::IncomingConnectionError {
                local_addr:local_addr.clone(),
                send_back_addr:send_back_addr.clone(),
                error: format!("{:?}", error),
                connection_id:*connection_id,
            },
            libp2p::swarm::SwarmEvent::OutgoingConnectionError {
                peer_id,
                error,
                connection_id,
            } => Self::OutgoingConnectionError {
                peer_id:*peer_id,
                error: format!("{:?}", error),
                connection_id:*connection_id,
            },
            libp2p::swarm::SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => Self::NewListenAddr {
                listener_id:*listener_id,
                address:address.clone(),
            },
            libp2p::swarm::SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => Self::ExpiredListenAddr {
                listener_id:*listener_id,
                address:address.clone(),
            },
            libp2p::swarm::SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => Self::ListenerClosed {
                listener_id:*listener_id,
                addresses:addresses.clone(),
                reason: format!("{:?}", reason),
            },
            libp2p::swarm::SwarmEvent::ListenerError { listener_id, error } => {
                Self::ListenerError {
                    listener_id:*listener_id,
                    error: format!("{:?}", error),
                }
            }
            libp2p::swarm::SwarmEvent::Dialing {
                peer_id,
                connection_id,
            } => Self::Dialing {
                peer_id:*peer_id,
                connection_id:*connection_id,
            },
            _ => unimplemented!("New branch not covered"),
        };
        Ok(ev)
    }
}

impl TryFrom<super::SwarmEvent> for SwarmEvent {
    type Error = ();
    fn try_from(value: super::SwarmEvent) -> Result<Self, Self::Error> {
        let ev = match value {
            libp2p::swarm::SwarmEvent::Behaviour(_) => return Err(()),
            libp2p::swarm::SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in,
                connection_id,
            } => Self::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors: format!("{:?}", concurrent_dial_errors),
                established_in,
                connection_id,
            },
            libp2p::swarm::SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause,
                connection_id,
            } => Self::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause: format!("{:?}", cause),
                connection_id,
            },
            libp2p::swarm::SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
                connection_id,
            } => Self::IncomingConnection {
                local_addr,
                send_back_addr,
                connection_id,
            },
            libp2p::swarm::SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
                connection_id,
            } => Self::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error: format!("{:?}", error),
                connection_id,
            },
            libp2p::swarm::SwarmEvent::OutgoingConnectionError {
                peer_id,
                error,
                connection_id,
            } => Self::OutgoingConnectionError {
                peer_id,
                error: format!("{:?}", error),
                connection_id,
            },
            libp2p::swarm::SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => Self::NewListenAddr {
                listener_id,
                address,
            },
            libp2p::swarm::SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => Self::ExpiredListenAddr {
                listener_id,
                address,
            },
            libp2p::swarm::SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => Self::ListenerClosed {
                listener_id,
                addresses,
                reason: format!("{:?}", reason),
            },
            libp2p::swarm::SwarmEvent::ListenerError { listener_id, error } => {
                Self::ListenerError {
                    listener_id,
                    error: format!("{:?}", error),
                }
            }
            libp2p::swarm::SwarmEvent::Dialing {
                peer_id,
                connection_id,
            } => Self::Dialing {
                peer_id,
                connection_id,
            },
            _ => unimplemented!("New branch not covered"),
        };
        Ok(ev)
    }
}

use crate::net::p2p::protocols::*;
pub struct OutEventBundle {
    pub messaging_rx: mpsc::Receiver<messaging::OutEvent>,
    #[cfg(feature = "tethering")]
    pub tethering_rx: mpsc::Receiver<tethering::OutEvent>,
    pub relay_client_rx: mpsc::Receiver<relay_client::OutEvent>,
    pub relay_server_rx: mpsc::Receiver<relay_server::OutEvent>,
}
