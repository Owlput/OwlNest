use libp2p::{Multiaddr, TransportError};

use super::*;

pub fn handle_swarm(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Error: Missing subcommands. Type \"swarm help\" for more information");
        return;
    }
    match command[2] {
        "help" => println!("{}", TOP_HELP_MESSAGE),
        "dial" => handle_swarm_dial(manager, command),
        "listen" => handle_swarm_listen(manager, command),
        "listener" => handle_swarm_listener(manager, command),
        _ => println!(
            "Failed to execute: unrecognized subcommand. Type \"swarm help\" for more information."
        ),
    }
}

fn handle_swarm_dial(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Error: Missing required argument <address>, syntax: `swarm dial <address>`");
        return;
    }
    let addr = match command[1].parse::<Multiaddr>() {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing address `{}`: {}", command[1], e);
            return;
        }
    };
    let op = Op::Dial(addr.clone());
    if let OpResult::Dial(result) = manager.blocking_swarm_exec(op) {
        match result {
            Ok(_) => println!("Now dialing {}", addr),
            Err(e) => println!("Failed to initiate dial {} with error: {}", addr, e),
        }
    }
}

fn handle_swarm_listen(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Error: Missing required argument <address>, syntax: `swarm listen <address>`");
        return;
    }
    let addr = match command[1].parse::<Multiaddr>() {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing address `{}`: {}", command[1], e);
            return;
        }
    };
    let op = Op::Listen(addr.clone());
    if let OpResult::Listen(result) = manager.blocking_swarm_exec(op) {
        match result {
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

fn handle_swarm_listener(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing subcommands. Type \"swarm listener help\" for more information");
        return;
    }
    match command[2]{
        "ls" =>
            if let OpResult::ListListeners(list) = manager.blocking_swarm_exec(swarm::Op::ListListeners){
                println!("Active listeners: \n{:#?}",list);
            },
        _=>println!("Failed to execute: unrecognized subcommand. Type \"swarm listener help\" for more information.")
    }
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
                Check whether the swarm has connected to the
                given peer.
ban <peer ID>
                Ban the given peer, refuse further connections
                from that peer.
unban <peer ID>
                Unban the given peer.
"#;
