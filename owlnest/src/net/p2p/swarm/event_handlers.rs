use super::{behaviour::BehaviourEvent, manager::Rx, InEvent, Swarm};
use crate::net::p2p::protocols;
use libp2p::Multiaddr;
use owlnest_macro::handle_callback_sender;
use tracing::{debug, info, trace};

#[inline]
pub async fn handle_swarm_event(ev: &super::SwarmEvent, swarm: &mut Swarm) {
    #[cfg(feature = "libp2p-protocols")]
    use crate::net::p2p::kad::swarm_hooks::*;
    use libp2p::swarm::SwarmEvent::*;
    #[allow(unused)]
    match ev {
        Behaviour(event) => handle_behaviour_event(swarm, event),
        ConnectionEstablished {
            peer_id, endpoint, ..
        } => {
            #[cfg(feature = "libp2p-protocols")]
            kad_add(swarm, *peer_id, endpoint.clone());
        }
        ConnectionClosed {
            peer_id, endpoint, ..
        } =>
        {
            #[cfg(feature = "libp2p-protocols")]
            kad_remove(swarm, *peer_id, endpoint.clone())
        }
        IncomingConnection {
            send_back_addr,
            local_addr,
            ..
        } => debug!(
            "Incoming connection from {} on local address {}",
            send_back_addr, local_addr
        ),
        IncomingConnectionError {
            local_addr,
            send_back_addr,
            error,
            ..
        } => info!(
            "Incoming connection error from {} on local address {}, error: {:?}",
            send_back_addr, local_addr, error
        ),
        OutgoingConnectionError { peer_id, error, .. } => {
            use libp2p::TransportError;
            if let libp2p_swarm::DialError::Transport(transport_err) = error {
                let closure =
                    |err: &(Multiaddr, libp2p::TransportError<std::io::Error>)| match &err.1 {
                        TransportError::MultiaddrNotSupported(addr) => {
                            (addr.clone(), "MultiaddrNotSupported".to_string())
                        }
                        TransportError::Other(e) => (err.0.clone(), e.kind().to_string()),
                    };
                let info = transport_err
                    .iter()
                    .map(closure)
                    .collect::<Vec<(Multiaddr, String)>>();
                info!("Outgoing connection error: {:?}", info);
                return;
            }
            info!(
                "Outgoing connection error to peer {:?}: {:?}",
                peer_id, error
            );
        }
        NewListenAddr { address, .. } => info!("Listening on {:?}", address),
        ExpiredListenAddr { address, .. } => info!("Expired listen address: {}", address),

        ListenerClosed {
            addresses, reason, ..
        } => trace!("Listener on address {:?} closed: {:?}", addresses, reason),
        ListenerError { listener_id, error } => {
            info!("Listener {:?} reported an error {}", listener_id, error)
        }
        Dialing { peer_id, .. } => trace!("Dailing peer? {:?}", peer_id),
        NewExternalAddrCandidate { address } => {
            info!(
                "A possible external address has been discovered: {}",
                address
            );
        }
        ExternalAddrConfirmed { address } => {
            info!(
                "A possible external address has been confirmed: {}",
                address
            );
        }
        ExternalAddrExpired { address } => {
            debug!("A possible external address has expired: {}", address)
        }
        NewExternalAddrOfPeer { .. } => {}
        uncovered => unimplemented!("New branch {:?} not covered", uncovered),
    }
}

#[inline]
pub fn handle_incoming_event(ev: Rx, swarm: &mut Swarm) {
    #[allow(unused)]
    use crate::net::p2p::protocols::*;
    use Rx::*;
    trace!("Receive incoming event {:?}", ev);
    match ev {
        Swarm(ev) => swarm_op_exec(swarm, ev),
        #[cfg(feature = "owlnest-protocols")]
        Advertise(ev) => swarm.behaviour_mut().advertise.push_event(ev),
        #[cfg(feature = "owlnest-protocols")]
        Blob(ev) => swarm.behaviour_mut().blob.push_event(ev),
        #[cfg(feature = "owlnest-protocols")]
        Messaging(ev) => swarm.behaviour_mut().messaging.push_event(ev),
        #[cfg(feature = "libp2p-protocols")]
        Kad(ev) => kad::map_in_event(ev, &mut swarm.behaviour_mut().kad),
        #[cfg(feature = "libp2p-protocols")]
        Mdns(ev) => mdns::map_in_event(ev, &mut swarm.behaviour_mut().mdns),
        #[cfg(feature = "libp2p-protocols")]
        AutoNat(ev) => autonat::map_in_event(&mut swarm.behaviour_mut().autonat, ev),
    }
}

#[inline]
pub fn swarm_op_exec(swarm: &mut Swarm, ev: InEvent) {
    use InEvent::*;
    match ev {
        Dial(addr, callback) => {
            handle_callback_sender!(swarm.dial(addr) => callback)
        }
        Listen(addr, callback) => {
            handle_callback_sender!(swarm.listen_on(addr) => callback)
        }
        AddExternalAddress(addr, callback) => {
            handle_callback_sender!(swarm.add_external_address(addr) => callback)
        }
        RemoveExternalAddress(addr, callback) => {
            handle_callback_sender!(swarm.remove_external_address(&addr) => callback)
        }
        DisconnectFromPeerId(peer_id, callback) => {
            handle_callback_sender!(swarm.disconnect_peer_id(peer_id) => callback)
        }
        ListExternalAddresses(callback) => {
            let addr_list = swarm
                .external_addresses()
                .cloned()
                .collect::<Vec<Multiaddr>>();
            handle_callback_sender!(addr_list => callback)
        }
        ListListeners(callback) => {
            let listener_list = swarm.listeners().cloned().collect::<Vec<Multiaddr>>();
            handle_callback_sender!(listener_list => callback)
        }
        IsConnectedToPeerId(peer_id, callback) => {
            handle_callback_sender!(swarm.is_connected(&peer_id) => callback)
        }
    }
}

/// Use for external protocol that doesn't expose callback communication only
#[allow(unused)]
#[inline]
fn handle_behaviour_event(swarm: &mut Swarm, ev: &BehaviourEvent) {
    use super::behaviour::BehaviourEvent::*;
    use protocols::*;
    match ev {
        #[cfg(feature = "libp2p-protocols")]
        Kad(ev) => kad::ev_dispatch(ev),
        #[cfg(feature = "libp2p-protocols")]
        Identify(ev) => identify::ev_dispatch(ev),
        #[cfg(feature = "libp2p-protocols")]
        Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        #[cfg(feature = "libp2p-protocols")]
        RelayServer(ev) => relay_server::ev_dispatch(ev),
        #[cfg(feature = "libp2p-protocols")]
        Dcutr(ev) => dcutr::ev_dispatch(ev),
        #[cfg(feature = "libp2p-protocols")]
        AutoNat(ev) => autonat::ev_dispatch(ev),
        #[cfg(feature = "libp2p-protocols")]
        Upnp(ev) => upnp::ev_dispatch(ev),
        #[cfg(feature = "libp2p-protocols")]
        Ping(ev) => ping::ev_dispatch(ev),
        _ => {}
    }
}
