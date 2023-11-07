use crate::event_bus::listened_event::Listenable;
use libp2p::relay::client;
use tracing::{debug, info};

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
        ReservationReqFailed {
            relay_peer_id,
            renewal,
            error,
        } => {
            println!(
                "Reservation sent to relay {} failed, IsRenewal:{}, error:{:?}",
                relay_peer_id, renewal, error
            );
            info!(
                "Reservation sent to relay {} failed, IsRenewal:{}, error:{:?}",
                relay_peer_id, renewal, error
            )
        }
        OutboundCircuitEstablished {
            relay_peer_id,
            limit,
        } => debug!(
            "Outbound circuit to relay {} established, limit:{:?}",
            relay_peer_id, limit
        ),
        OutboundCircuitReqFailed {
            relay_peer_id,
            error,
        } => info!(
            "Outbound circuit request to relay {} failed, error:{:?}",
            relay_peer_id, error
        ),
        InboundCircuitEstablished { src_peer_id, limit } => debug!(
            "Inbound circuit from source peer {} established, limit:{:?}",
            src_peer_id, limit
        ),
        InboundCircuitReqDenied { src_peer_id } => {
            info!("An inbound circuit from {} was denied", src_peer_id)
        }
        InboundCircuitReqDenyFailed { src_peer_id, error } => info!(
            "Iutbound circuit from source peer {} can't be denied, error:{:?}",
            src_peer_id, error
        ),
    }
}

impl Listenable for OutEvent {
    fn as_event_identifier() -> String {
        "/libp2p/relay_client:OutEvent".into()
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
