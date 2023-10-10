use crate::net::p2p::swarm::manager::Manager;

pub fn handle_relay_ext(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Missing sencond level command")
    }
    match command[1] {
        "provider" => handle_provider(manager, command),
        _ => println!("Unrecognized command. Type `relay_ext help` for more info."),
    }
}
fn handle_provider(manager: &Manager, command: Vec<&str>) {
    match command[2] {
        "start" => {
            manager
                .executor()
                .block_on(manager.relay_ext().start_providing());
            println!("relay_ext is set to provide advertised peers");
        }
        "stop" => {}
        "status" => {}
        _ => {}
    }
}
