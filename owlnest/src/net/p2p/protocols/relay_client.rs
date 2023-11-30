use libp2p::relay::client;
use tracing::debug;

/// `Behaviour` of libp2p's `relay` protocol.
pub use client::Behaviour;
pub use client::Event as OutEvent;

pub fn ev_dispatch(ev: &client::Event) {
    use client::Event::*;
    match ev {
        ReservationReqAccepted {
            relay_peer_id,
            renewal,
            limit,
        } => {
            if !renewal {
                println!(
                    "Reservation sent to relay {} has been accepted. Limit:{:?}",
                    relay_peer_id, limit
                );
            }
            debug!(
                "Reservation sent to relay {} has been accepted. IsRenewal:{}, limit:{:?}",
                relay_peer_id, renewal, limit
            )
        }
        OutboundCircuitEstablished {
            relay_peer_id,
            limit,
        } => debug!(
            "Outbound circuit to relay {} established, limit:{:?}",
            relay_peer_id, limit
        ),
        InboundCircuitEstablished { src_peer_id, limit } => debug!(
            "Inbound circuit from source peer {} established, limit:{:?}",
            src_peer_id, limit
        ),
    }
}

#[allow(unused)]
pub(crate) mod cli {
    use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};

    use crate::net::p2p::swarm::{cli::format_transport_error, manager::Manager};

    pub fn handle_relayclient(manager: &Manager, command: Vec<&str>) {
        if command.len() < 2 {
            println!("Missing subcommand. Type `relay-client help` for more information");
            return;
        }
        match command[1] {
            "listen" => handle_listen(manager, command),
            _ => {}
        }
    }

    fn handle_listen(manager: &Manager, command: Vec<&str>) {
        if command.len() < 4 {
            println!("Missing argument. Syntax relay-client connect <relay-server-address> <relay-server-peer-id>")
        }
        let addr = match command[2].parse::<Multiaddr>() {
            Ok(addr) => addr,
            Err(e) => {
                println!("Error: Failed parsing address `{}`: {}", command[2], e);
                return;
            }
        };

        let peer_id = match command[3].parse::<PeerId>() {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to parse peer ID for input {}: {}", command[2], e);
                return;
            }
        };
        let addr = addr.with(Protocol::P2p(peer_id)).with(Protocol::P2pCircuit);
        match manager.swarm().listen(&addr) {
            Ok(listener_id) => println!(
                "Successfully listening on {} with listener ID {:?}",
                addr, listener_id
            ),

            Err(e) => println!(
                "Failed to listen on {} with error: {}",
                addr,
                format_transport_error(e)
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::net::p2p::setup_default;
    use libp2p::{multiaddr::Protocol, Multiaddr};
    use std::{thread, time::Duration};

    #[test]
    fn test() {
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        let (peer3_m, _) = setup_default();
        assert!(peer1_m
            .swarm()
            .listen(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()) // Pick a random port that is available
            .is_ok());
        peer1_m
            .swarm()
            .add_external_address(&peer1_m.swarm().list_listeners()[0]); // The address is on local network
        assert!(peer2_m
            .swarm()
            .dial(&peer1_m.swarm().list_listeners()[0])
            .is_ok());
        thread::sleep(Duration::from_secs(1));
        assert!(peer2_m
            .swarm()
            .listen(
                &peer1_m.swarm().list_listeners()[0]
                    .clone()
                    .with(Protocol::P2p(peer1_m.identity().get_peer_id()))
                    .with(Protocol::P2pCircuit)
            )
            .is_ok());
        thread::sleep(Duration::from_secs(1));
        assert!(peer2_m.swarm().list_listeners().len() > 0);
        assert!(peer3_m
            .swarm()
            .dial(
                &peer1_m.swarm().list_listeners()[0]
                    .clone()
                    .with(Protocol::P2p(peer1_m.identity().get_peer_id()))
                    .with(Protocol::P2pCircuit)
                    .with(Protocol::P2p(peer2_m.identity().get_peer_id()))
            )
            .is_ok());
        thread::sleep(Duration::from_secs(1));
        assert!(peer3_m
            .swarm()
            .is_connected(&peer2_m.identity().get_peer_id()));
    }
}
