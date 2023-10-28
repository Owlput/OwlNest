use crate::net::p2p::swarm::op::SwarmHandle;
use libp2p::{Multiaddr, PeerId, TransportError};

pub fn handle_swarm(handle: &SwarmHandle, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Error: Missing subcommands. Type \"swarm help\" for more information");
        return;
    }
    match command[1] {
        "help" => println!("{}", TOP_HELP_MESSAGE),
        "dial" => {
            if command.len() < 3 {
                println!(
                    "Error: Missing required argument <address>, syntax: `swarm dial <address>`"
                );
                return;
            }
            handle_swarm_dial(handle, command[2])
        }
        "listen" => {
            if command.len() < 3 {
                println!(
                    "Error: Missing required argument <address>, syntax: `swarm listen <address>`"
                );
                return;
            }
            handle_swarm_listen(handle, command[2])
        }
        "listener" => handle_swarm_listener(handle, command),
        "isconnected" => handle_swarm_isconnected(handle, command),
        _ => println!(
            "Failed to execute: unrecognized subcommand. Type \"swarm help\" for more information."
        ),
    }
}

pub fn handle_swarm_dial(handle: &SwarmHandle, addr: &str) {
    let addr = match addr.parse::<Multiaddr>() {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing address `{}`: {}", addr, e);
            return;
        }
    };
    if let Err(e) = handle.dial(&addr) {
        println!("Failed to initiate dial {} with error: {:?}", addr, e);
    } else {
        println!("Dialing {}", addr);
    }
}

pub fn handle_swarm_listen(handle: &SwarmHandle, addr: &str) {
    let addr = match addr.parse::<Multiaddr>() {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing address `{}`: {}", addr, e);
            return;
        }
    };
    match handle.listen(&addr) {
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

fn handle_swarm_listener(handle: &SwarmHandle, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing subcommands. Type \"swarm listener help\" for more information");
        return;
    }
    match command[2]{
        "ls" => println!("Active listeners: {:?}",handle.list_listeners()),
        "help" => println!("{}",LISTENER_SUBCOMMAND_HELP),
        _=>println!("Failed to execute: unrecognized subcommand. Type \"swarm listener help\" for more information.")
    }
}

fn handle_swarm_isconnected(handle: &SwarmHandle, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing argument <PeerId>. Syntax: swarm isconnected <PeerId>");
        return;
    }
    let peer_id = match command[2].parse::<PeerId>() {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to parse peer ID for input {}: {}", command[2], e);
            return;
        }
    };
    println!("{}", handle.is_connected(&peer_id))
}

fn format_transport_error(e: TransportError<std::io::Error>) -> String {
    match e {
        TransportError::MultiaddrNotSupported(addr) => {
            format!("Requested address {} is not supported.", addr)
        }
        TransportError::Other(e) => {
            let error_string = format!("{:?}", e);
            if error_string.contains("AddrNotAvailable") {
                return "Local interface associated with the given address does not exist".into();
            }
            error_string
        }
    }
}

const TOP_HELP_MESSAGE: &str = r#"
swarm: Subcommand for managing libp2p swarm.

Available subcommands:
    dial <address>          
                Dial the given address in multiaddr format.
                Dial result may not shown directly after
                command issued.

    listen <address>        
                Listen on the given address in multiaddr
                format.

    listener <subcommand>
                Managing connection listeners.

    external-addr <subcommand>           
                Managing external addresses.

    disconnect <peer ID>
                Terminate connections from the given peer.

    isconnected <peer ID>
                Check whether the swarm has connected to 
                the given peer.

    ban <peer ID>
                Ban the given peer, refuse further 
                connections from that peer.
                
    unban <peer ID>
                Unban the given peer.
"#;

const LISTENER_SUBCOMMAND_HELP: &str = r#""#;
