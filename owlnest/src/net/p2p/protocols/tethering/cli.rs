use crate::net::p2p::swarm::manager::Manager;
use libp2p::PeerId;
use std::str::FromStr;

pub fn handle_tethering(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Missing subcommands. Please type \"tethering help\" for more information");
        return;
    }
    match command[1] {
        "trust" => handle_trust_peer(manager, command),
        "untrust" => handle_untrust_peer(manager, command),
        "help" => println!("{}", TOP_HELP_MESSAGE),
        _ => println!("Unrecognized command. Please type \"tethering help\" for more information"),
    }
}

fn handle_trust_peer(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Error: Missing required argument <peer ID>, syntax: `tethering trust <peer ID>`");
        return;
    };
    let peer_to_trust = match PeerId::from_str(command[1]) {
        Ok(id) => id,
        Err(e) => {
            println!(
                "Error: Failed parsing peer id from `{}`: {:?}.",
                command[1], e
            );
            return;
        }
    };

    let result = manager
        .executor()
        .block_on(manager.tethering().trust(peer_to_trust));
    if result.is_ok() {
        println!("Successfully trusted peer {}", peer_to_trust)
    } else {
        println!("Peer {} is already in trust list", peer_to_trust)
    }
}

fn handle_untrust_peer(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!(
            "Error: Missing required argument <peer ID>, syntax: `tethering untrust <peer ID>`"
        );
        return;
    };
    let peer_to_untrust = match PeerId::from_str(command[1]) {
        Ok(id) => id,
        Err(e) => {
            println!(
                "Error: Failed parsing peer id from `{}`: {:?}.",
                command[1], e
            );
            return;
        }
    };
    let result = manager
        .executor()
        .block_on(manager.tethering().untrust(peer_to_untrust));
    if result.is_ok() {
        println!(
            "Successfully removed peer {} from trust list",
            peer_to_untrust
        )
    } else {
        println!(
            "Error: Failed to untrust peer {}: Peer not found.",
            peer_to_untrust
        )
    }
}

const TOP_HELP_MESSAGE: &str = r#"
Owlnest tethering protocol 0.0.1

Available Subcommands
    trust <peer ID>         
                Trust the given peer to allow remote command execution.

    untrust <peer ID>       
                Remove the peer from trust list.
"#;
