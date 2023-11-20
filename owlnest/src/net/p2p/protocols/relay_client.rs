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
    use crate::net::p2p::swarm::manager::Manager;

    pub fn handle_relayclient(manager: &Manager, command: Vec<&str>) {
        if command.len() < 2 {
            println!("Missing subcommand. Type `relay-client help` for more information");
            return;
        }
        match command[1] {
            "listen" => {}
            _ => {}
        }
    }

    fn handle_listen(manager: &Manager, command: Vec<&str>) {
        if command.len() < 4 {
            println!("Missing argument. Syntax relay-client connect <relay-server-address> <relay-server-peer-id>")
        }
    }
}
