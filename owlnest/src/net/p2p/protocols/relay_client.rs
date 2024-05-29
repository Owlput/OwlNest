use libp2p::relay::client;

/// `Behaviour` of libp2p's `relay` protocol.
pub use client::Behaviour;
pub use client::Event as OutEvent;

#[allow(unused)]
pub(crate) mod cli {
    use super::*;
    use crate::net::p2p::swarm::{cli::format_transport_error, manager::Manager};
    use clap::Subcommand;
    use futures::TryFutureExt;
    use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};
    use tracing::{error, warn};

    /// Subcommand for interacting relay client.
    #[derive(Debug, Subcommand)]
    pub enum RelayClient {
        /// Listen on the given relay
        Listen {
            /// The address of the relay, in multiaddr format.
            #[arg(required = true)]
            address: Multiaddr,
            /// The peer ID of the relay.
            /// The reason behind is that there can be multiple peers behind the same address.
            #[arg(required = true)]
            peer_id: PeerId,
        },
    }

    pub fn setup(manager: &Manager) {
        let mut listener = manager.event_subscriber().subscribe();
        use crate::net::p2p::swarm::{behaviour::BehaviourEvent, SwarmEvent};
        use tracing::debug;
        manager.executor().spawn(async move {
            while let Ok(ev) = listener.recv().await {
                if let SwarmEvent::Behaviour(BehaviourEvent::RelayClient(ev)) = ev.as_ref() {
                    use client::Event::*;
                    match ev {
                        ReservationReqAccepted {
                            relay_peer_id,
                            renewal,
                            limit,
                        } => {
                            if !renewal {
                                println!(
                                    "Reservation sent to relay {} has been accepted. Limit:{:?}",
                                    relay_peer_id, limit
                                );
                            }
                            debug!(
                                "Reservation on relay {} has been renewed. limit:{:?}",
                                relay_peer_id, limit
                            )
                        }
                        OutboundCircuitEstablished {
                            relay_peer_id,
                            limit,
                        } => debug!(
                            "Outbound circuit to relay {} established, limit:{:?}",
                            relay_peer_id, limit
                        ),
                        InboundCircuitEstablished { src_peer_id, limit } => debug!(
                            "Inbound circuit from source peer {} established, limit:{:?}",
                            src_peer_id, limit
                        ),
                    }
                }
            }
        });
    }

    pub fn handle_relay_client(manager: &Manager, command: RelayClient) {
        use RelayClient::*;
        match command {
            Listen { address, peer_id } => {
                let addr: Multiaddr = address
                    .with(Protocol::P2p(peer_id))
                    .with(Protocol::P2pCircuit);
                match manager.swarm().listen_blocking(&addr) {
                    Ok(listener_id) => println!(
                        "Successfully listening on {} with listener ID {:?}",
                        addr, listener_id
                    ),

                    Err(e) => println!(
                        "Failed to listen on {} with error: {}",
                        addr,
                        format_transport_error(e)
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::net::p2p::{swarm::Manager, test_suit::setup_default};
    use libp2p::{multiaddr::Protocol, Multiaddr};
    use owlnest_macro::listen_event;
    use std::{
        io::stdout,
        sync::{atomic::AtomicU8, Arc},
        thread,
        time::Duration,
    };

    #[test]
    fn test() {
        setup_logging();
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        let (peer3_m, _) = setup_default();
        println!("server:{}", peer1_m.identity().get_peer_id());
        println!("listener:{}", peer2_m.identity().get_peer_id());
        println!("dialer:{}", peer3_m.identity().get_peer_id());
        assert!(peer1_m
            .swarm()
            .listen_blocking(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap()) // Pick a random port that is available
            .is_ok());
        thread::sleep(Duration::from_millis(100));
        let server_address = peer1_m.swarm().list_listeners_blocking();
        let mut addr_filtered = server_address
            .iter()
            .filter(|addr| addr.to_string().contains("127.0.0.1"));
        let server_address: Multiaddr = addr_filtered.next().cloned().unwrap();
        peer1_m
            .swarm()
            .add_external_address_blocking(server_address.clone()); // The address is on local network
        thread::sleep(Duration::from_millis(20000));
        assert!(peer2_m.swarm().dial_blocking(&server_address).is_ok());
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m
            .swarm()
            .listen_blocking(
                &server_address
                    .clone()
                    .with(Protocol::P2p(peer1_m.identity().get_peer_id()))
                    .with(Protocol::P2pCircuit)
            )
            .is_ok());
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m.swarm().list_listeners_blocking().len() > 0);
        assert!(peer3_m
            .swarm()
            .dial_blocking(
                &server_address
                    .clone()
                    .with(Protocol::P2p(peer1_m.identity().get_peer_id()))
                    .with(Protocol::P2pCircuit)
                    .with(Protocol::P2p(peer2_m.identity().get_peer_id()))
            )
            .is_ok());
        thread::sleep(Duration::from_millis(1000));
        assert!(peer3_m
            .swarm()
            .is_connected_blocking(peer2_m.identity().get_peer_id()));
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
        with_messaging(&peer1_m, &peer2_m, &peer3_m);
    }

    #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
    fn with_messaging(peer1_m: &Manager, peer2_m: &Manager, peer3_m: &Manager) {
        use owlnest_messaging::Message;
        use std::sync::atomic::Ordering;
        println!("testing with messaging");
        let counter = Arc::new(AtomicU8::new(0));
        listen_message(peer1_m, counter.clone());
        listen_message(peer2_m, counter.clone());
        listen_message(peer3_m, counter.clone());
        let _ = peer2_m
            .executor()
            .block_on(peer2_m.messaging().send_message(
                peer1_m.identity().get_peer_id(),
                Message::new(
                    peer2_m.identity().get_peer_id(),
                    peer1_m.identity().get_peer_id(),
                    "Hi",
                ),
            ));
        let _ = peer2_m
            .executor()
            .block_on(peer2_m.messaging().send_message(
                peer3_m.identity().get_peer_id(),
                Message::new(
                    peer2_m.identity().get_peer_id(),
                    peer3_m.identity().get_peer_id(),
                    "Hi",
                ),
            ));
        let _ = peer1_m
            .executor()
            .block_on(peer1_m.messaging().send_message(
                peer3_m.identity().get_peer_id(),
                Message::new(
                    peer1_m.identity().get_peer_id(),
                    peer3_m.identity().get_peer_id(),
                    "Hi",
                ),
            ));
        let _ = peer3_m
            .executor()
            .block_on(peer3_m.messaging().send_message(
                peer1_m.identity().get_peer_id(),
                Message::new(
                    peer3_m.identity().get_peer_id(),
                    peer1_m.identity().get_peer_id(),
                    "Hi",
                ),
            ));
        let _ = peer3_m
            .executor()
            .block_on(peer3_m.messaging().send_message(
                peer2_m.identity().get_peer_id(),
                Message::new(
                    peer3_m.identity().get_peer_id(),
                    peer2_m.identity().get_peer_id(),
                    "Hi",
                ),
            ));
        thread::sleep(Duration::from_millis(1000));
        assert_eq!(counter.fetch_add(0, Ordering::Relaxed), 5)
    }

    fn setup_logging() {
        use std::sync::Mutex;
        use tracing::Level;
        use tracing_log::LogTracer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Layer;
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("owlnest", Level::DEBUG)
            .with_target("owlnest::net::p2p::protocols::kad", Level::WARN)
            .with_target("libp2p_core::transport::choice", Level::WARN)
            .with_target("yamux", Level::WARN)
            .with_target("hickory_proto", Level::WARN)
            .with_target("libp2p_mdns", Level::WARN)
            .with_target("libp2p_noise", Level::WARN)
            .with_target("libp2p_kad", Level::WARN)
            .with_target("libp2p_relay", Level::TRACE)
            .with_target("libp2p_identify", Level::WARN)
            .with_target("libp2p", Level::TRACE)
            .with_target("multistream_select", Level::INFO)
            .with_target("owlnest_messaging", Level::TRACE)
            .with_target("", Level::TRACE);
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_ansi(false)
            .with_writer(Mutex::new(stdout()))
            .with_filter(filter);
        let reg = tracing_subscriber::registry().with(layer);
        tracing::subscriber::set_global_default(reg).expect("you can only set global default once");
        LogTracer::init().unwrap()
    }

    fn listen_message(mgr: &Manager, counter: Arc<AtomicU8>) {
        use owlnest_messaging::OutEvent;
        use std::sync::atomic::Ordering;
        let mut listener = mgr.event_subscriber().subscribe();
        mgr.executor().spawn(
            listen_event!( listener for Messaging, OutEvent::IncomingMessage { .. }=>{
                counter.fetch_add(1, Ordering::Relaxed);
            } ),
        );
    }
}
