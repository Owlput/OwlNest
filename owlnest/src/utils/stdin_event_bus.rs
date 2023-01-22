use libp2p::{Multiaddr, PeerId};
use tokio::sync::mpsc;
use crate::net::p2p::swarm;
#[cfg(feature="relay-client")]
use crate::net::p2p::relayed_swarm;

#[derive(Debug)]
pub enum StdinEvent {
    Dial(Multiaddr),
    Listen(Multiaddr),
    #[cfg(feature = "messaging")]
    Messaging(crate::net::p2p::messaging::InEvent),
    #[cfg(feature = "relay-server")]
    RelayServer(crate::net::p2p::relay_server::InEvent),
}

pub fn setup_bus(local_peer: PeerId) -> mpsc::UnboundedReceiver<StdinEvent> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let mut iter = std::io::stdin().lines();
        println!("OwlNest is now running in interactive mode. Type in `help` for more information.");
        loop {
            if let Some(Ok(line)) = iter.next() {
                let command: Vec<&str> = line.split(' ').collect();
                match command[0] {
                    "help"=>{
                        println!("{}",HELP_MESSAGE)
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
                        match tx.send(StdinEvent::Dial(addr)) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
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
                        match tx.send(StdinEvent::Listen(addr)) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                    }
                    #[cfg(feature="relay-server")]
                    "reserve" => {
                        if command.len() < 2 {
                            println!("Error: Missing required argument <address>, syntax: `reserve <address>`");
                            continue;
                        }

                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                continue;
                            }
                        };
                        match tx.send(StdinEvent::RelayServer(crate::net::p2p::protocols::relay_server::InEvent::AddExternalAddress(addr))) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                    }
                    #[cfg(feature = "messaging")]
                    "msg" => {
                        if command.len() < 3 {
                            println!("Error: Missing required argument(s), syntax: `msg <peer id> <message>`");
                            println!("Hint: You should use `/p2p/<peer id>` for the moment");
                            continue;
                        }
                        let addr = match command[1].parse::<Multiaddr>() {
                            Ok(addr) => addr,
                            Err(e) => {
                                println!("Error: Failed parsing address `{}`: {}", command[1], e);
                                println!("Hint: You should use `/p2p/<peer id>` for the moment");
                                continue;
                            }
                        };
                        let target_peer = match PeerId::try_from_multiaddr(&addr) {
                            Some(id) => id,
                            None => {
                                println!("Error: failed to extract peer id");
                                println!("Hint: You should use `/p2p/<peer id>` for the moment");
                                continue;
                            }
                        };
                        match tx.send(StdinEvent::Messaging(crate::net::p2p::messaging::InEvent::PostMessage(
                            crate::net::p2p::messaging::Message::new(&local_peer, &target_peer, command[2].into()),
                        ))) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Failed to send your command: {}", e)
                            }
                        }
                    }
                    "" => {}
                    _ => println!("Unrecognized command"),
                }
            };
        }
    });
    rx
}


pub fn setup_distributor(mut center: mpsc::UnboundedReceiver<StdinEvent>,swarm_mgr:swarm::Manager) {
    tokio::spawn(async move{
        while let Some(ev) = center.recv().await{
            let _ = match ev{
                StdinEvent::Dial(addr) => swarm_mgr.dial(addr).await,
                StdinEvent::Listen(addr) => swarm_mgr.listen(addr).await,
                #[cfg(feature="messaging")]
                StdinEvent::Messaging(ev) => swarm_mgr.execute(swarm::Op::Messaging(ev)).await,
                #[cfg(feature = "relay-server")]
                StdinEvent::RelayServer(ev)=>swarm_mgr.execute(swarm::Op::RelayServer(ev)).await
            };
        }
    });
}

#[cfg(feature="relay-client")]
pub fn setup_distributor_relayed(mut center: mpsc::UnboundedReceiver<StdinEvent>,swarm_mgr:relayed_swarm::Manager) {
    

    tokio::spawn(async move{
        while let Some(ev) = center.recv().await{
            let res = match ev{
                StdinEvent::Dial(addr) => swarm_mgr.dial(addr).await,
                StdinEvent::Listen(addr) => swarm_mgr.listen(addr).await,
                #[cfg(feature="messaging")]
                StdinEvent::Messaging(msg) => swarm_mgr.execute(relayed_swarm::Op::Messaging(msg)).await,
                #[cfg(feature = "relay-server")]
                StdinEvent::RelayServer(ev)=>swarm_mgr.execute(relayed_swarm::Op::RelayServer(ev)).await
            };
            println!("{:#?}",res)
        }
    });
}

const HELP_MESSAGE:&str = r#"
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
        reserve <address>   Reserve the given address, in Multiaddr format.       
"#;