use std::fmt::Debug;

#[cfg(feature = "messaging")]
use crate::net::p2p::protocols::messaging;
#[cfg(feature = "tethering")]
use crate::net::p2p::protocols::tethering;
use crate::net::p2p::swarm;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::{mpsc, oneshot};

type SwarmEvent = swarm::in_event::swarm::InEvent;

#[derive(Debug)]
pub enum StdinEvent {
    Swarm(SwarmEvent),
    #[cfg(feature = "tethering")]
    Tethering(tethering::InEvent),
    #[cfg(feature = "messaging")]
    Messaging(messaging::InEvent),
}

pub fn setup_bus(local_peer: PeerId) -> mpsc::UnboundedReceiver<StdinEvent> {
    let (tx, rx) = mpsc::unbounded_channel();
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
                        let (callback_tx, callback_rx) = oneshot::channel();
                        match tx.send(StdinEvent::Swarm(SwarmEvent::Dial { addr, callback_tx })) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                        handle_callback_rx(callback_rx);
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
                        let (callback_tx, callback_rx) = oneshot::channel();
                        match tx.send(StdinEvent::Swarm(SwarmEvent::Listen { addr, callback_tx })) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                        handle_callback_rx(callback_rx);
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
                        let (callback_tx, callback_rx) = oneshot::channel();
                        match tx.send(StdinEvent::Tethering(tethering::InEvent::new(
                            None,
                            tethering::TetherOps::Trust(peer_to_trust),
                            callback_tx,
                        ))) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                        match callback_rx.blocking_recv() {
                            Ok(res) => println!("{:?}", res),
                            Err(e) => println!("Failed to receive callback: {}", e),
                        }
                    }
                    #[cfg(feature = "relay-server")]
                    "reserve" => {
                        if command.len() < 3 {
                            println!("Error: Missing required argument(s), syntax: `reserve <address> <score>`");
                            continue;
                        }
                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                continue;
                            }
                        };
                        let score = match command[2] {
                            "INF" => libp2p::swarm::AddressScore::Infinite,
                            v => match v.parse::<u32>() {
                                Ok(num) => libp2p::swarm::AddressScore::Finite(num),
                                Err(_) => {
                                    println!("Failed to parse score from {}, accepting `INF` or a valid 32-bit integer.",command[2]);
                                    continue;
                                }
                            },
                        };
                        let (callback_tx, callback_rx) = oneshot::channel();
                        match tx.send(StdinEvent::Swarm(SwarmEvent::AddExternalAddress {
                            addr,
                            score,
                            callback_tx,
                        })) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                        handle_callback_rx(callback_rx);
                    }
                    #[cfg(feature = "messaging")]
                    "msg" => {
                        let (callback_tx, callback_rx) = oneshot::channel();
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
                        let message =
                            messaging::Message::new(&local_peer, &target_peer, command[2].into());
                        match tx.send(StdinEvent::Messaging(messaging::InEvent::PostMessage(
                            target_peer,
                            message,
                            callback_tx,
                        ))) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                        handle_callback_rx(callback_rx);
                    }
                    "" => {}
                    _ => println!("Unrecognized command"),
                }
            };
        }
    });
    rx
}

pub fn setup_distributor(
    mut center: mpsc::UnboundedReceiver<StdinEvent>,
    swarm_mgr: swarm::Manager,
) {
    tokio::spawn(async move {
        while let Some(ev) = center.recv().await {
            let _ = match ev {
                StdinEvent::Swarm(ev) => swarm_mgr.execute(swarm::InEvent::Swarm(ev)).await,
                #[cfg(feature = "tethering")]
                StdinEvent::Tethering(ev) => swarm_mgr.execute(swarm::InEvent::Tethering(ev)).await,
                #[cfg(feature = "messaging")]
                StdinEvent::Messaging(ev) => swarm_mgr.execute(swarm::InEvent::Messaging(ev)).await,
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
        reserve <address>   Reserve the given address, in Multiaddr format.
                            Note that this command can't be used to make relay
                            reservations, but to occupy the supplied address.   
"#;

fn handle_callback_rx<T>(callback_rx: oneshot::Receiver<T>)
where
    T: Debug,
{
    match callback_rx.blocking_recv() {
        Ok(res) => println!("{:?}", res),
        Err(e) => println!("Failed to receive from callback: {}", e),
    }
}
