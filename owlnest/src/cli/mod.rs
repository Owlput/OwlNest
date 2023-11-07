mod utils;

use std::io::stdout;
use std::sync::Arc;

use crate::net::p2p::protocols::*;
use crate::net::p2p::swarm::manager::Manager;
use crate::net::p2p::{identity::IdentityUnion, swarm};
use crossterm::style::Stylize;
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;
use rustyline::{error::ReadlineError, DefaultEditor};
use tokio::sync::Notify;

use self::utils::handle_utils;

/// Make current terminal interactive
pub fn setup_interactive_shell(
    ident: IdentityUnion,
    manager: Manager,
    shutdown_notifier: Arc<Notify>,
) {
    std::thread::spawn(move || {
        stdout().execute(Clear(ClearType::All)).unwrap();
        println!("OwlNest is now running in interactive mode, type \"help\" for more information.");
        let mut rl = DefaultEditor::new().unwrap();
        let mut retry_times = 0u32;
        loop {
            let line_read = rl.readline(&">> ".stylize().dark_cyan().to_string());
            match line_read {
                Ok(line) => {
                    rl.add_history_entry(line.as_str()).unwrap();
                    handle_command(line, &manager, &ident)
                }
                Err(e) => {
                    if handle_err(e, &mut retry_times) {
                        break;
                    }
                }
            }
        }
        println!("shutdown criteria reached, shutting down this peer...");
        shutdown_notifier.notify_one()
    });
}

fn handle_command(line: String, manager: &Manager, ident: &IdentityUnion) {
    let command: Vec<&str> = line.split(' ').collect();
    match command[0] {
        "help" => {
            println!("{}", HELP_MESSAGE)
        }
        "clear" => drop(stdout().execute(Clear(ClearType::FromCursorUp))),
        "id" => println!("Local peer ID: {}", ident.get_peer_id()),
        "dial" => {
            if command.len() < 2 {
                println!("Error: Missing required argument <address>, syntax: `dial <address>`");
                return;
            }
            swarm::cli::handle_swarm_dial(&manager.swarm(), command[1])
        }
        "listen" => {
            if command.len() < 2 {
                println!("Error: Missing required argument <address>, syntax: `listen <address>`");
                return;
            }
            swarm::cli::handle_swarm_listen(&manager.swarm(), command[1])
        }
        "swarm" => swarm::cli::handle_swarm(&manager.swarm(), command),
        "tethering" => tethering::cli::handle_tethering(manager, command),
        "messaging" => messaging::handle_messaging(manager, ident, command),
        "kad" => kad::cli::handle_kad(manager, command),
        "mdns" => mdns::cli::handle_mdns(manager, command),
        "relay_ext" => relay_ext::cli::handle_relay_ext(manager, command),
        "utils" => handle_utils(command),
        "" => {}
        _ => println!("Unrecognized command"),
    }
}

fn handle_err(e: ReadlineError, retry_times: &mut u32) -> bool {
    match e {
        ReadlineError::Io(e) => {
            println!("Failed to read input with IO error: {:?}", e);
            should_exit(retry_times, 5)
        }
        ReadlineError::Eof => {
            println!("Failed to read input: Unexpected EOF");
            should_exit(retry_times, 5)
        }
        ReadlineError::Interrupted => {
            println!("Interrupt signal received.");
            true
        }
        ReadlineError::WindowResized => false,
        _ => {
            println!("Unknown error occurred");
            should_exit(retry_times, 5)
        }
    }
}

fn should_exit(retry_times: &mut u32, max_retry_times: u32) -> bool {
    *retry_times += 1;
    if *retry_times >= max_retry_times {
        println!("Maximum retry attempts reached, exiting interactive shell. To exit the process, send interrupt signal.");
        true
    } else {
        false
    }
}

const HELP_MESSAGE: &str = r#"
OwlNest 0.0.1
Interactive shell version 0.0.1

Available commands:
    help                Show this help message.
    clear               Clear current screen.
    id                  Show the peer ID of this node.
    listen <address>    Listen on the given address, in Multiaddr format.
    dial <address>      Dial the given address, in Multiaddr format.
    swarm               Subcommand for accessing features on the swarm.
    kad                 Subcommand for `/ipfs/kad/1.0.0` protocol.
    messaging           Subcommand for `messaging` protocol.
    tethering           Subcommand for `tethering` protocol.
    mdns                Subcommand for `mdns` protocol.
    utils               Subcommand for various utilities.
"#;
