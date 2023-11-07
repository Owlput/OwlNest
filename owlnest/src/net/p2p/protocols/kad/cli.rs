use super::*;
use crate::net::p2p::swarm;
use swarm::manager::Manager;

/// Top-level handler for `kad` command.
pub fn handle_kad(manager: &Manager, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Missing subcommands. Type \"kad help\" for more information");
        return;
    }
    match command[1] {
        "lookup" => kad_lookup(manager, command),
        "bootstrap" => kad_bootstrap(manager),
        "set-mode" => kad_setmode(manager, command),
        "help" => println!("{}", TOP_HELP_MESSAGE),
        _ => println!("Unrecoginzed subcommands. Type \"kad help\" for more information"),
    }
}

/// Handler for `kad lookup` command.
fn kad_lookup(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing required argument: <peer ID>");
        return;
    }
    let peer_id = match PeerId::from_str(command[2]) {
        Ok(peer_id) => peer_id,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[1], e);
            return;
        }
    };
    let result = manager.executor().block_on(manager.kad().lookup(peer_id));
    println!("{:?}", result)
}

fn kad_bootstrap(manager: &Manager) {
    let result = manager.executor().block_on(manager.kad().bootstrap());
    if let Err(_) = result {
        println!("No known peer in the DHT");
        return;
    }
    println!("Bootstrap started")
}

fn kad_setmode(manager: &Manager, command: Vec<&str>) {
    if command.len() < 3 {
        println!("Missing required argument: <mode>. Syntax: `kad set-mode <mode>`");
        return;
    }
    let mode = match command[2] {
        "client" => Some(Mode::Client),
        "server" => Some(Mode::Server),
        "default" => None,
        _ => {
            println!("Invalid mode, possible modes: `client`, `server`, `default`");
            return;
        }
    };
    if let Err(_) = manager.executor().block_on(manager.kad().set_mode(mode)) {
        println!("Timeout reached for setting kad mode");
        return;
    }
    println!("mode for kad has been set to {}", command[2])
}

/// Top-level help message for `kad` command.
const TOP_HELP_MESSAGE: &str = r#"
Protocol `/ipfs/kad/1.0.0`

Available Subcommands:
    lookup <peer ID>        
        Initiate a lookup for the given peer.

    bootstrap
        Start traversing the DHT network to get latest information
        about all peers participating the network.

    set-mode <mode>
        Set the local DHT manager to the given <mode>.
        Available modes are:
            `client`: Don't share local DHT to others.
            `server`: Broadcast local DHT to others.
            `default`: Restore to default mode, which is
                      automatically determined by local node.
"#;
