#[cfg(feature = "messaging")]
use crate::net::p2p::protocols::messaging;
#[cfg(feature = "tethering")]
use crate::net::p2p::protocols::tethering;
use crate::net::p2p::{
    identity::IdentityUnion,
    swarm
};
use libp2p::{Multiaddr, PeerId};
use rustyline::{error::ReadlineError, DefaultEditor};

pub fn setup_interactive_shell(ident: IdentityUnion, manager: swarm::Manager) {
    std::thread::spawn(move || {
        println!("{}OwlNest is now running in interactive mode, type \"help\" for more information.",termion::clear::All);
        let manager = manager.clone();
        let mut rl = DefaultEditor::new().unwrap();
        let mut retry_times = 0u32;
        loop {
            let line_read = rl.readline(&format!("{}>> {}",termion::color::LightYellow.fg_str(),termion::color::Reset.fg_str()));
            match line_read {
                Ok(line) => {
                    rl.add_history_entry(line.as_str()).unwrap();
                    handle_command(line, &manager, &ident)
                }
                Err(e) => if handle_err(e, &mut retry_times){
                    break;
                },
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
        "clear"=>println!("{}",termion::clear::BeforeCursor),
        "dial" => swarm::cli::handle_dial(manager, command),
        "listen" => swarm::cli::handle_listen(manager, command),
        #[cfg(feature = "tethering")]
        "tethering" => tethering::cli::handle_tethering(manager, command),
        #[cfg(feature = "messaging")]
        "msg" => {
            if command.len() < 3 {
                println!("Error: Missing required argument(s), syntax: `msg <peer id> <message>`");
                return;
            }
            let addr = match command[1].parse::<Multiaddr>() {
                Ok(addr) => addr,
                Err(e) => {
                    println!("Error: Failed parsing address `{}`: {}", command[1], e);
                    println!(
                        "Hint: You should use `/p2p/<peer id>` for the moment to supply peer IDs."
                    );
                    return;
                }
            };
            let target_peer = match PeerId::try_from_multiaddr(&addr) {
                Some(id) => id,
                None => {
                    println!("Error: failed to extract peer id");
                    println!(
                        "Hint: You should use `/p2p/<peer id>` for the moment to supply peer IDs."
                    );
                    return;
                }
            };
            match manager.blocking_messaging_exec(messaging::Op::SendMessage(
                target_peer.clone(),
                messaging::Message::new(&ident.get_peer_id(), &target_peer, command[2].into()),
            )) {
                messaging::OpResult::SuccessfulPost(rtt) => println!(
                    "Message successfully sent, estimated round trip time {}ms",
                    rtt.as_millis()
                ),
                messaging::OpResult::Error(e) => println!("Failed to send message: {}", e),
            }
        }
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
            println!("Interrupt signal received, exiting interactive shell");
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
        println!("Maximum retry attempts reached, exiting interactive shell");
        true
    } else {
        false
    }
}

const HELP_MESSAGE: &str = r#"
    OwlNest 0.0.1 Interactive Mode
    Interactive shell version 0.0.0

    Available commands:
        help    Show this help message.
        listen <address>    Listen on the given address, in Multiaddr format.
        dial <address>      Dial the given address, in Multiaddr format.
        msg <peer id> <message>     
                            Send message using `/owlput/messaging/0.0.1` protocol
                            to the given peer, identified by peer ID.
                            Peer ID needs to be supplied in `/p2p/<peer ID>` format.
        trust <peer id>     Allow the specified peer to use tethering protocol
                            to control the behaviour of this peer.
        untrust <peer id>   Remove the specified peer from trust list.
"#;
