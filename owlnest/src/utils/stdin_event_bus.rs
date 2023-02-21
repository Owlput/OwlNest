#[cfg(feature = "messaging")]
use crate::net::p2p::protocols::messaging;
#[cfg(feature = "tethering")]
use crate::net::p2p::protocols::tethering;
use crate::net::p2p::swarm;
use libp2p::{Multiaddr, PeerId};

pub fn setup_bus(local_peer: PeerId, manager: swarm::Manager) {
    std::thread::spawn(move || {
        let mut iter = std::io::stdin().lines();
        println!(
            "OwlNest is now running in interactive mode. Type in `help` for more information."
        );
        loop {
            if let Some(Ok(line)) = iter.next() {
                let command: Vec<&str> = line.split(' ').collect();
                match command[0] {
                    "help" => {
                        println!("{}", HELP_MESSAGE)
                    }
                    "dial" => {
                        if command.len() < 2 {
                            println!("Error: Missing required argument <address>, syntax: `dial <address>`");
                            continue;
                        }

                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                continue;
                            }
                        };
                        match manager.blocking_dial(&addr) {
                            Ok(_) => println!("Successfully dialed {}", addr),
                            Err(e) => println!("Failed to dial {} with error: {}", addr, e),
                        }
                    }
                    "listen" => {
                        if command.len() < 2 {
                            println!("Error: Missing required argument <address>, syntax: `listen <address>`");
                            continue;
                        }

                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                continue;
                            }
                        };
                        match manager.blocking_listen(&addr) {
                            Ok(listener_id) => println!(
                                "Successfully listening on {} with listener ID {:?}",
                                addr, listener_id
                            ),
                            Err(e) => println!("Failed to listen on {} with error: {}", addr, e),
                        }
                    }
                    #[cfg(feature = "tethering")]
                    "trust" => {
                        if command.len() < 2 {
                            println!("Error: Missing required argument <address>, syntax: `trust <peer id>`");
                            continue;
                        }
                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                println!("Hint: You should use `/p2p/<peer id>` for the moment to supply peer IDs.");
                                continue;
                            }
                        };
                        let peer_to_trust = match PeerId::try_from_multiaddr(&addr) {
                            Some(id) => id,
                            None => {
                                println!(
                                    "Error: Failed parsing peer id from `{}`: No peer ID present.",
                                    command[1]
                                );
                                println!("Hint: You should use `/p2p/<peer id>` for the moment to supply peer IDs.");
                                continue;
                            }
                        };
                        match manager.blocking_tethering_local_exec(tethering::TetheringOp::Trust(
                            peer_to_trust.clone(),
                        )){
                            tethering::TetheringOpResult::Ok => {
                                println!("Successfully trusted peer {}",peer_to_trust)
                            },
                            tethering::TetheringOpResult::AlreadyTrusted => {
                                println!("Peer {} is already in trust list",peer_to_trust)
                            },
                            tethering::TetheringOpResult::Err(e) => {
                                println!("Failed to trust peer {}: {:?}",peer_to_trust,e)
                            },
                        }
                    }
                    #[cfg(feature = "messaging")]
                    "msg" => {
                        if command.len() < 3 {
                            println!("Error: Missing required argument(s), syntax: `msg <peer id> <message>`");
                            continue;
                        }
                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                println!("Hint: You should use `/p2p/<peer id>` for the moment to supply peer IDs.");
                                continue;
                            }
                        };
                        let target_peer = match PeerId::try_from_multiaddr(&addr) {
                            Some(id) => id,
                            None => {
                                println!("Error: failed to extract peer id");
                                println!("Hint: You should use `/p2p/<peer id>` for the moment to supply peer IDs.");
                                continue;
                            }
                        };
                        match manager.blocking_messaging_exec(messaging::Op::SendMessage(target_peer.clone(), messaging::Message::new(&local_peer, &target_peer, command[2].into()))){
                            messaging::OpResult::SuccessfulPost(rtt) => println!("Message successfully sent, estimated round trip time {}ms",rtt.as_millis()),
                            messaging::OpResult::Error(e) => println!("Failed to send message: {}",e),
                        }
                    }
                    "" => {}
                    _ => println!("Unrecognized command"),
                }
            };
        }
    });
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