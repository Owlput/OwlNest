use super::{behaviour::BehaviourEvent, manager::Rx, InEvent, Swarm};
use crate::net::p2p::protocols;
use libp2p::Multiaddr;
use owlnest_macro::handle_callback_sender;
use tracing::{debug, info, trace};

#[inline]
pub async fn handle_swarm_event(ev: &super::SwarmEvent, swarm: &mut Swarm) {
    #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
    use crate::net::p2p::kad::swarm_hooks::*;
    use libp2p::swarm::SwarmEvent::*;
    #[allow(unused)]
    match ev {
        Behaviour(event) => handle_behaviour_event(swarm, event),
        ConnectionEstablished {
            peer_id, endpoint, ..
        } => {
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
            kad_add(swarm, *peer_id, endpoint.clone());
        }
        ConnectionClosed {
            peer_id, endpoint, ..
        } =>
        {
            #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
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
            if let libp2p::swarm::DialError::Transport(transport_err) = error {
                let closure =
                    |err: &(Multiaddr, libp2p::TransportError<std::io::Error>)| match &err.1 {
                        TransportError::MultiaddrNotSupported(addr) => {
                            (addr.clone(), "MultiaddrNotSupported".to_string())
                        }
                        TransportError::Other(e) => (err.0.clone(), e.to_string()),
                    };
                let info = transport_err
                    .iter()
                    .map(closure)
                    .collect::<Box<[(Multiaddr, String)]>>();
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
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
        Advertise(ev) => swarm.behaviour_mut().advertise.push_event(ev),
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-blob"))]
        Blob(ev) => swarm.behaviour_mut().blob.push_event(ev),
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
        Messaging(ev) => swarm.behaviour_mut().messaging.push_event(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
        Kad(ev) => kad::map_in_event(ev, &mut swarm.behaviour_mut().kad),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
        Mdns(ev) => mdns::map_in_event(ev, &mut swarm.behaviour_mut().mdns),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-autonat"))]
        AutoNat(ev) => autonat::map_in_event(&mut swarm.behaviour_mut().autonat, ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"))]
        Gossipsub(ev) => gossipsub::map_in_event(&mut swarm.behaviour_mut().gossipsub, ev),
    }
}

#[inline]
pub fn swarm_op_exec(swarm: &mut Swarm, ev: InEvent) {
    use InEvent::*;
    match ev {
        Dial { address, callback } => {
            handle_callback_sender!(swarm.dial(address) => callback)
        }
        Listen { address, callback } => {
            handle_callback_sender!(swarm.listen_on(address) => callback)
        }
        ListListeners { callback } => {
            let listener_list = swarm.listeners().cloned().collect();
            handle_callback_sender!(listener_list => callback)
        }
        RemoveListeners {
            listener_id,
            callback,
        } => {
            handle_callback_sender!(swarm.remove_listener(listener_id)=> callback)
        }
        AddExternalAddress { address, callback } => {
            handle_callback_sender!(swarm.add_external_address(address) => callback)
        }
        RemoveExternalAddress { address, callback } => {
            handle_callback_sender!(swarm.remove_external_address(&address) => callback)
        }
        DisconnectFromPeerId { peer_id, callback } => {
            handle_callback_sender!(swarm.disconnect_peer_id(peer_id) => callback)
        }
        ListExternalAddresses { callback } => {
            let addr_list = swarm.external_addresses().cloned().collect();
            handle_callback_sender!(addr_list => callback)
        }
        ListConnected { callback } => {
            handle_callback_sender!(swarm.connected_peers().copied().collect()=>callback)
        }
        IsConnectedToPeerId { peer_id, callback } => {
            let result = swarm.is_connected(&peer_id);
            trace!("is conneted to {}: {}", peer_id, result);
            handle_callback_sender!( result => callback)
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
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
        Kad(ev) => kad::ev_dispatch(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
        Identify(ev) => identify::ev_dispatch(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
        Mdns(ev) => mdns::ev_dispatch(ev, swarm),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
        RelayServer(ev) => relay_server::ev_dispatch(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-dcutr"))]
        Dcutr(ev) => dcutr::ev_dispatch(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-autonat"))]
        AutoNat(ev) => autonat::ev_dispatch(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-upnp"))]
        Upnp(ev) => upnp::ev_dispatch(ev),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-ping"))]
        Ping(ev) => ping::ev_dispatch(ev),
        _ => {}
    }
}
