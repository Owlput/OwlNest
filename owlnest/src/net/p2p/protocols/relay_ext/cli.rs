use libp2p::PeerId;

use crate::net::p2p::swarm::manager::Manager;

pub fn handle_relay_ext(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Missing subcommand. Type `relay-ext help` for more info.")
    }
    match command[1] {
        "provider" => provider::handle_provider(manager, command),
        "set-advertise-self" => handle_set_advertise_self(manager, command),
        "query-advertised" => handle_query_advertised(manager, command),
        "help" => println!("{}", HELP_MESSAGE),
        _ => println!("Unrecognized command. Type `relay-ext help` for more info."),
    }
}

mod provider {
    use super::*;
    pub fn handle_provider(manager: &Manager, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing subcommand. Type `relay-ext provider help` for more info.");
            return;
        }
        match command[2] {
            "start" => {
                manager
                    .executor()
                    .block_on(manager.relay_ext().set_provider_state(true));
                println!("relay_ext is set to provide advertised peers");
            }
            "stop" => {}
            "state" => {
                let state = manager
                    .executor()
                    .block_on(manager.relay_ext().provider_state());
                println!("isProviding:{}", state)
            }
            "remove-peer" => {
                let peer_id = match command[2].parse::<PeerId>() {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to parse peer ID for input {}: {}", command[2], e);
                        return;
                    }
                };
                let is_advertising = manager
                    .executor()
                    .block_on(manager.relay_ext().remove_advertised(&peer_id));
                println!(
                    "Advertisement for peer {} is set to {}",
                    peer_id, is_advertising
                )
            }
            "clear" => {
                manager
                    .executor()
                    .block_on(manager.relay_ext().clear_advertised());
            }
            "help" => println!("{}", HELP_MESSAGE),
            _ => {}
        }
    }

    const HELP_MESSAGE: &str = r#"
    Protocol `/owlnest/relay-ext/0.0.1`
    `relay-ext provider help`

    Available Subcommands:
        start
                    Start providing information about advertised 
                    peers.

        stop
                    Stop providing information about advertised 
                    peers.

        state
                    Get current provider state.
        
        remove-peer <peer id>
                    Remove the given peer from advertisement.

        clear
                    Remove all peers in the advertisement.
    "#;
}

fn handle_set_advertise_self(manager: &Manager, command: Vec<&str>) {
    if command.len() < 4 {
        println!("Missing argument(s). Syntax: relay-ext set-advertise-self <PeerId> <state:Bool>");
        return;
    }
    let peer_id: PeerId = match command[2].parse() {
        Ok(v) => v,
        Err(e) => {
            println!("Error: Failed parsing peer ID from `{}`: {}", command[2], e);
            return;
        }
    };
    let state = match command[3].parse() {
        Ok(v) => v,
        Err(_) => {
            println!(
                "Error: Failed parsing boolean from {}. Expecting `true` or `false`",
                command[3]
            );
            return;
        }
    };
    manager
        .executor()
        .block_on(manager.relay_ext().set_remote_advertisement(peer_id, state));
    println!("OK")
}

fn handle_query_advertised(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing argument. Syntax: relay-ext query-advertised <PeerId>");
    }
    let peer_id: PeerId = match command[2].parse() {
        Ok(v) => v,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[2], e);
            return;
        }
    };
    let result = manager
        .executor()
        .block_on(manager.relay_ext().query_advertised_peer(peer_id));
    println!("Query result:{:?}", result)
}

const HELP_MESSAGE: &str = r#"
Protocol `/owlnest/relay-ext/0.0.1`
Relay protocol extension version 0.0.1

Available subcommands:
    provider
                Subcommand for local provider.
    
    set-advertise-self <PeerId> <state:Bool>
                Set advertisement state on a remote peer.
                `true` to advertise, `false` to stop advertising.
    
    query-advertised <PeerId>
                Query all for advertised peers on a given peer.
"#;
