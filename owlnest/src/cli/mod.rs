mod utils;

use crate::net::p2p::protocols::*;
use crate::net::p2p::{identity::IdentityUnion, swarm};
use rustyline::{error::ReadlineError, DefaultEditor};

use self::utils::handle_utils;

/// Make current terminal interactive
pub fn setup_interactive_shell(ident: IdentityUnion, manager: swarm::Manager) {
    std::thread::spawn(move || {
        println!(
            "{}OwlNest is now running in interactive mode, type \"help\" for more information.",
            termion::clear::All
        );
        let manager = manager.clone();
        let mut rl = DefaultEditor::new().unwrap();
        let mut retry_times = 0u32;
        loop {
            let line_read = rl.readline(&format!(
                "{}>> {}",
                termion::color::LightYellow.fg_str(),
                termion::color::Reset.fg_str()
            ));
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
    });
}

fn handle_command(line: String, manager: &swarm::Manager, ident: &IdentityUnion) {
    let command: Vec<&str> = line.split(' ').collect();
    match command[0] {
        "help" => {
            println!("{}", HELP_MESSAGE)
        }
        "clear" => println!("{}", termion::clear::BeforeCursor),
        "id"=>println!("Local peer ID: {}",ident.get_peer_id()),
        "swarm" => swarm::cli::handle_swarm(manager, command),
        
        "tethering" => tethering::cli::handle_tethering(manager, command),
        
        "messaging" => messaging::cli::handle_messaging(manager,ident,command),
        "kad" => kad::cli::handle_kad(manager, command),
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
            println!("Interrupt signal received, exiting interactive shell. Repeat to exit the whole process.");
            true
        }
        ReadlineError::Errno(e) => {
            println!("Unix signal received: {:?}, exiting interactive shell", e);
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
Interactive shell version 0.0.0

Available commands:
    help                Show this help message.
    clear               Clear current screen.
    id                  Show the peer ID of this node.
    listen <address>    Listen on the given address, in Multiaddr format.
    dial <address>      Dial the given address, in Multiaddr format.
    kad                 Subcommand for `/ipfs/kad/1.0.0` protocol.
    messaging           Subcommand for `messaging` protocol.
    tethering           Subcommand for `tethering` protocol.
    utils               Subcommand for various utilities.
"#;
