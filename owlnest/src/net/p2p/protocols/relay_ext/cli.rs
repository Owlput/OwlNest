use libp2p::PeerId;

use crate::net::p2p::swarm::manager::Manager;

pub fn handle_relay_ext(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Missing sencond level command")
    }
    match command[1] {
        "provider" => handle_provider(manager, command),
        "advertise-self" => handle_advertise_self(manager, command),
        "query-advertised"=> handle_query_advertised(manager, command),
        _ => println!("Unrecognized command. Type `relay_ext help` for more info."),
    }
}
fn handle_provider(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing subcommand");
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
            let state = manager.executor().block_on(manager.relay_ext().provider_state());
            println!("isProviding:{}",state)
        }
        _ => {}
    }
}
fn handle_advertise_self(manager: &Manager, command: Vec<&str>) {
    if command.len() < 4 {
        println!("Missing argument(s). Syntax: relay-ext advertise-self <PeerId> <state:Bool>");
        return;
    }
    let peer_id: PeerId = match command[2].parse() {
        Ok(v) => v,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[2], e);
            return;
        }
    };
    let state = match command[3].parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: Failed parsing boolean from {}. Expected `true` or `false`",command[3]);
            return;
        }
    };
    manager.executor().block_on(manager.relay_ext().set_remote_advertisement(peer_id, state));
    println!("OK")
}

fn handle_query_advertised(manager: &Manager,command: Vec<&str>){
    if command.len() < 3{
        println!("Missing argument. Syntax: relay-ext query-advertised <PeerId>");
    }
    let peer_id: PeerId = match command[2].parse() {
        Ok(v) => v,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[2], e);
            return;
        }
    };
    let result = manager.executor().block_on(manager.relay_ext().query_advertised_peer(peer_id));
    println!("Query result:{:?}",result)
}
