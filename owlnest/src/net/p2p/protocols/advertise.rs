use crate::net::p2p::swarm::EventSender;
use libp2p::PeerId;
pub use owlnest_advertise::*;
use owlnest_macro::{listen_event, with_timeout};
use std::sync::{atomic::AtomicU64, Arc};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    counter: Arc<AtomicU64>,
}
impl Handle {
    pub fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                sender: tx,
                event_tx: event_tx.clone(),
                counter: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }
    pub async fn query_advertised_peer(&self, relay: PeerId) -> Result<Vec<PeerId>, Error> {
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise, OutEvent::QueryAnswered { from, list }=> {
            if *from == relay {
                return list.clone();
            }
        });

        let ev = InEvent::QueryAdvertisedPeer(relay);
        self.sender.send(ev).await.unwrap();
        match with_timeout!(fut, 10) {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::Timeout),
        }
    }
    pub async fn set_provider_state(&self, state: bool) -> bool {
        let op_id = self.next_id();
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise, OutEvent::ProviderState(state, id)=> {
            if *id != op_id {
                continue;
            }
            return *state;
        });
        let ev = InEvent::SetProviderState(state, op_id);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    pub async fn provider_state(&self) -> bool {
        let op_id = self.next_id();
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise, OutEvent::ProviderState(state, id)=> {
            if *id != op_id {
                continue;
            }
            return *state;
        });
        let ev = InEvent::GetProviderState(op_id);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    pub async fn set_remote_advertisement(&self, remote: PeerId, state: bool) {
        let op_id = self.next_id();
        let ev = InEvent::SetAdvertisingSelf {
            remote,
            state,
            id: op_id,
        };
        self.sender.send(ev).await.expect("Send to succeed");
    }
    pub async fn remove_advertised(&self, peer_id: &PeerId) -> bool {
        let ev = InEvent::RemoveAdvertised(*peer_id);
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise, OutEvent::AdvertisedPeerChanged(target,state)=>{
            if *target == *peer_id{
                return *state
            }
        });
        self.sender.send(ev).await.expect("Send to succeed");
        with_timeout!(fut, 10).expect("Future to finish in 10s")
    }
    pub async fn clear_advertised(&self) {
        let ev = InEvent::ClearAdvertised;
        self.sender.send(ev).await.expect("Send to succeed")
    }
    fn next_id(&self) -> u64 {
        use std::sync::atomic::Ordering;
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

pub(crate) mod cli {
    use libp2p::PeerId;

    use crate::net::p2p::swarm::manager::Manager;

    pub fn handle_advertise(manager: &Manager, command: Vec<&str>) {
        if command.len() < 2 {
            println!("Missing subcommand. Type `advertise help` for more info.")
        }
        match command[1] {
            "provider" => provider::handle_provider(manager, command),
            "set-advertise-self" => handle_set_advertise_self(manager, command),
            "query-advertised" => handle_query_advertised(manager, command),
            "help" => println!("{}", HELP_MESSAGE),
            _ => println!("Unrecognized command. Type `relay-ext help` for more info."),
        }
    }

    mod provider {
        use super::*;
        pub fn handle_provider(manager: &Manager, command: Vec<&str>) {
            if command.len() < 3 {
                println!("Missing subcommand. Type `relay-ext provider help` for more info.");
                return;
            }
            match command[2] {
                "start" => {
                    manager
                        .executor()
                        .block_on(manager.advertise().set_provider_state(true));
                    println!("advertise is set to provide advertised peers");
                }
                "stop" => {}
                "state" => {
                    let state = manager
                        .executor()
                        .block_on(manager.advertise().provider_state());
                    println!("isProviding:{}", state)
                }
                "remove-peer" => {
                    let peer_id = match command[2].parse::<PeerId>() {
                        Ok(v) => v,
                        Err(e) => {
                            println!("Failed to parse peer ID for input {}: {}", command[2], e);
                            return;
                        }
                    };
                    let is_advertising = manager
                        .executor()
                        .block_on(manager.advertise().remove_advertised(&peer_id));
                    println!(
                        "Advertisement for peer {} is set to {}",
                        peer_id, is_advertising
                    )
                }
                "clear" => {
                    manager
                        .executor()
                        .block_on(manager.advertise().clear_advertised());
                }
                "help" => println!("{}", HELP_MESSAGE),
                _ => {}
            }
        }

        const HELP_MESSAGE: &str = r#"
    Protocol `/owlnest/relay-ext/0.0.1`
    `relay-ext provider help`

    Available Subcommands:
        start
                    Start providing information about advertised 
                    peers.

        stop
                    Stop providing information about advertised 
                    peers.

        state
                    Get current provider state.
        
        remove-peer <peer id>
                    Remove the given peer from advertisement.

        clear
                    Remove all peers in the advertisement.
    "#;
    }

    fn handle_set_advertise_self(manager: &Manager, command: Vec<&str>) {
        if command.len() < 4 {
            println!(
                "Missing argument(s). Syntax: relay-ext set-advertise-self <PeerId> <state:Bool>"
            );
            return;
        }
        let peer_id: PeerId = match command[2].parse() {
            Ok(v) => v,
            Err(e) => {
                println!("Error: Failed parsing peer ID from `{}`: {}", command[2], e);
                return;
            }
        };
        let state = match command[3].parse() {
            Ok(v) => v,
            Err(_) => {
                println!(
                    "Error: Failed parsing boolean from {}. Expecting `true` or `false`",
                    command[3]
                );
                return;
            }
        };
        manager
            .executor()
            .block_on(manager.advertise().set_remote_advertisement(peer_id, state));
        println!("OK")
    }

    fn handle_query_advertised(manager: &Manager, command: Vec<&str>) {
        if command.len() < 3 {
            println!("Missing argument. Syntax: relay-ext query-advertised <PeerId>");
        }
        let peer_id: PeerId = match command[2].parse() {
            Ok(v) => v,
            Err(e) => {
                println!("Error: Failed parsing peer ID `{}`: {}", command[2], e);
                return;
            }
        };
        let result = manager
            .executor()
            .block_on(manager.advertise().query_advertised_peer(peer_id));
        println!("Query result:{:?}", result)
    }

    const HELP_MESSAGE: &str = r#"
Protocol `/owlnest/relay-ext/0.0.1`
Relay protocol extension version 0.0.1

Available subcommands:
    provider
                Subcommand for local provider.
    
    set-advertise-self <PeerId> <state:Bool>
                Set advertisement state on a remote peer.
                `true` to advertise, `false` to stop advertising.
    
    query-advertised <PeerId>
                Query all for advertised peers on a given peer.
"#;
}

#[cfg(test)]
mod test {
    use crate::net::p2p::test_suit::setup_default;
    use libp2p::Multiaddr;
    use std::{thread, time::Duration};

    #[test]
    fn test() {
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        peer1_m
            .swarm()
            .listen_blocking(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        let peer1_id = peer1_m.identity().get_peer_id();
        let peer2_id = peer2_m.identity().get_peer_id();
        peer2_m
            .swarm()
            .dial_blocking(&peer1_m.swarm().list_listeners_blocking()[0])
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.advertise().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        peer2_m
            .executor()
            .block_on(peer2_m.advertise().set_remote_advertisement(peer1_id, true));
        assert!(peer2_m.swarm().is_connected_blocking(peer1_id));
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m
            .executor()
            .block_on(peer2_m.advertise().query_advertised_peer(peer1_id))
            .unwrap()
            .contains(&peer2_id));
        assert!(!peer1_m
            .executor()
            .block_on(peer1_m.advertise().set_provider_state(false)));
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.advertise().query_advertised_peer(peer1_id))
                .unwrap()
                .len()
                == 0
        );
        peer2_m.executor().block_on(
            peer2_m
                .advertise()
                .set_remote_advertisement(peer1_id, false),
        );
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.advertise().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.advertise().query_advertised_peer(peer1_id))
                .unwrap()
                .len()
                == 0
        );
    }
}
