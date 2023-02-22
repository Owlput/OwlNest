use std::str::FromStr;
use super::*;
use libp2p::PeerId;

use crate::net::p2p::swarm;

pub fn handle_tethering(manager: &swarm::Manager, command: Vec<&str>) {
    if command.len() < 2{
        println!("Missing subcommands. Please type \"tethering help\" for more information")
    }
    match command[1]{
        "trust"=>handle_trust_peer(manager, command),
        "untrust"=>handle_untrust_peer(manager, command),
        "help"=>println!("{}",TOP_HELP_MESSAGE),
        _ => println!("Unrecognized command. Please type \"tethering help\" for more information")
    }
}

fn handle_trust_peer(manager: &swarm::Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Error: Missing required argument <peer ID>, syntax: `tethering trust <peer ID>`");
        return;
    };
    let peer_to_trust = match PeerId::from_str(command[1])
    {
        Ok(id) => id,
        Err(e) => {
            println!(
                "Error: Failed parsing peer id from `{}`: {:?}.",
                command[1],e
            );
            return;
        }
    };
    match manager
        .blocking_tethering_local_exec(TetheringOp::Trust(peer_to_trust.clone()))
    {
        TetheringOpResult::Ok => {
            println!("Successfully trusted peer {}", peer_to_trust)
        }
        TetheringOpResult::AlreadyTrusted => {
            println!("Peer {} is already in trust list", peer_to_trust)
        }
        TetheringOpResult::Err(e) => {
            println!("Failed to trust peer {}: {:?}", peer_to_trust, e)
        }
    }
}

fn handle_untrust_peer(manager: &swarm::Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Error: Missing required argument <peer ID>, syntax: `tethering untrust <peer ID>`");
        return;
    };
    let peer_to_trust = match PeerId::from_str(command[1])
    {
        Ok(id) => id,
        Err(e) => {
            println!(
                "Error: Failed parsing peer id from `{}`: {:?}.",
                command[1],e
            );
            return;
        }
    };
    match manager
        .blocking_tethering_local_exec(TetheringOp::Untrust(peer_to_trust.clone()))
    {
        TetheringOpResult::Ok => {
            println!("Successfully trusted peer {}", peer_to_trust)
        }
        TetheringOpResult::Err(e) => {
            println!("Failed to untrust peer {}: {:?}", peer_to_trust, e)
        }
        _ => unreachable!()
    }
}

const TOP_HELP_MESSAGE:&str = r#"
Owlnest tethering protocol 0.0.1

Available Subcommands
    trust <peer ID>         Trust the given peer to allow
                            remote command execution.
    untrust <peer ID>       Remove the peer from trust list.
"#;