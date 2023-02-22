use libp2p::{Multiaddr, TransportError};

pub fn handle_dial(manager:&super::Manager,command:Vec<&str>){
    if command.len() < 2 {
        println!("Error: Missing required argument <address>, syntax: `dial <address>`");
        return;
    }

    let addr = match command[1].parse::<Multiaddr>() {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing address `{}`: {}", command[1], e);
            return;
        }
    };
    match manager.blocking_dial(&addr) {
        Ok(_) => println!("Now dialing {}", addr),
        Err(e) => println!("Failed to initiate dial {} with error: {}", addr, e),
    }
}

pub fn handle_listen(manager:&super::Manager,command:Vec<&str>){
    if command.len() < 2 {
        println!("Error: Missing required argument <address>, syntax: `listen <address>`");
        return;
    }

    let addr = match command[1].parse::<Multiaddr>() {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing address `{}`: {}", command[1], e);
            return;
        }
    };
    match manager.blocking_listen(&addr) {
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