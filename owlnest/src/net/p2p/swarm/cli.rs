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
        "listener" => listener::handle_swarm_listener(handle, command),
        "external-addr" => external_address::handle_swarm_externaladdress(handle, command),
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

mod listener {
    use super::SwarmHandle;

    pub fn handle_swarm_listener(handle: &SwarmHandle, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing subcommands. Type \"swarm listener help\" for more information");
            return;
        }
        match command[2]{
            "ls" => println!("Active listeners: {:?}",handle.list_listeners()),
            "help" => println!("{}",HELP_MESSAGE),
            _=>println!("Failed to execute: unrecognized subcommand. Type \"swarm listener help\" for more information.")
        }
    }

    const HELP_MESSAGE: &str = r"
    swarm listener: Subcommand for managing listeners of this swarm

    Available Subcommands:
        ls
            List all listeners of this swarm.

        help
            Show this help message.
    ";
}

mod external_address {
    use super::*;
    pub fn handle_swarm_externaladdress(handle: &SwarmHandle, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing subcommands. Type \"swarm external help\" for more information");
            return;
        }
        match command[2] {
            "add" => add(handle, command),
            "remove" => remove(handle, command),
            "ls" => ls(handle),
            "help" => println!("{}", HELP_MESSAGE),
            _ => {}
        }
    }
    fn add(handle: &SwarmHandle, command: Vec<&str>) {
        if command.len() < 4 {
            println!("Missing argument <address>. Syntax: swarm external-addr add <PeerId>");
            return;
        }
        let addr = match command[3].parse::<Multiaddr>() {
            Ok(addr) => addr,
            Err(e) => {
                println!("Error: Failed parsing address `{}`: {}", command[3], e);
                return;
            }
        };
        handle.add_external_address(&addr);
        println!("External address `{}` added", addr)
    }
    fn remove(handle: &SwarmHandle, command: Vec<&str>) {
        if command.len() < 4 {
            println!("Missing argument <address>. Syntax: swarm external-addr remove <PeerId>");
            return;
        }
        let addr = match command[3].parse::<Multiaddr>() {
            Ok(addr) => addr,
            Err(e) => {
                println!("Error: Failed parsing address `{}`: {}", command[3], e);
                return;
            }
        };
        handle.remove_external_address(&addr);
        println!("External address `{}` removed", addr)
    }
    fn ls(handle: &SwarmHandle) {
        let addresses = handle.list_external_addresses();
        println!("External addresses: {:?}", addresses)
    }

    const HELP_MESSAGE: &str = r"
    swarm external-addr: Subcommand for managing external addresses of the swarm

    Available subcommands:
        add <address>
            Add the <address> to external address(bind).
        
        remove <address>
            Remove the <address> from external address.

        ls
            List all external address of this swarm.
        
        help
            Show this help message.
    ";
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

const TOP_HELP_MESSAGE: &str = r#"
swarm: Subcommand for managing libp2p swarm.

Available subcommands:
    dial <address>          
        Dial <address>, in multiaddr format.
        Dial result may not shown directly after command issued.

    listen <address>        
        Listen on <address>, in multiaddr format.

    listener <subcommand>
        Managing connection listeners.

    external-addr <subcommand>           
        Managing external addresses.

    disconnect <peer ID>
        Terminate connections from the given peer.

    isconnected <peer ID>
        Check whether the swarm has connected to the given peer.

    ban <peer ID>
        Ban the given peer, refuse further connections from that peer.
                
    unban <peer ID>
        Unban the given peer.
"#;

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
