use crate::net::p2p::swarm::EventSender;
use libp2p::PeerId;
pub use owlnest_advertise::*;
use owlnest_macro::{generate_handler_method, listen_event, with_timeout};
use std::sync::{atomic::AtomicU64, Arc};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Handle {
    sender: mpsc::Sender<InEvent>,
    event_tx: EventSender,
    counter: Arc<AtomicU64>,
}
impl Handle {
    pub(crate) fn new(buffer: usize, event_tx: &EventSender) -> (Self, mpsc::Receiver<InEvent>) {
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
    /// Send query to a remote for current advertisements.
    /// Will return `Err(Error::NotProviding)` for peers who don't support this protocol.
    pub async fn query_advertised_peer(
        &self,
        relay: PeerId,
    ) -> Result<Option<Box<[PeerId]>>, Error> {
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise,
            OutEvent::QueryAnswered { from, list } => {
                if *from == relay {
                    return Ok(list.clone());
                }
            }
            OutEvent::Error(Error::NotProviding(peer)) => {
                if *peer == relay{
                    return Err(Error::NotProviding(*peer))
                }
            }
        );
        let ev = InEvent::QueryAdvertisedPeer(relay);
        self.sender.send(ev).await.unwrap();
        match with_timeout!(fut, 10) {
            Ok(v) => v,
            Err(_) => Err(Error::Timeout),
        }
    }
    /// Set advertisement on a remote peer.
    /// ## Silent failure
    /// This function will return immediately, the effect is not guaranteed:
    /// - peers that are not connected
    /// - peers that don't support this protocol
    /// - peers that are not providing
    pub async fn set_remote_advertisement(&self, remote: PeerId, state: bool) {
        let ev = InEvent::SetRemoteAdvertisement {
            remote,
            state,
            id: self.next_id(),
        };
        self.sender.send(ev).await.unwrap();
    }
    /// Set provider state of local peer.
    /// Will return a recent(not immediate) state change.
    pub async fn set_provider_state(&self, state: bool) -> bool {
        let op_id = self.next_id();
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(
            listener for Advertise,
            OutEvent::ProviderState(state, id)=> {
                if *id == op_id {
                    return *state;
                }
            }
        );
        let ev = InEvent::SetProviderState(state, op_id);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    /// Get provider state of local peer.
    /// Will return a immediate state report, e.g. only changes caused by this operation.
    pub async fn provider_state(&self) -> bool {
        let op_id = self.next_id();
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise, OutEvent::ProviderState(state, id)=> {
            if *id == op_id {
                return *state;
            }
        });
        let ev = InEvent::GetProviderState(op_id);
        self.sender.send(ev).await.expect("send to succeed");
        with_timeout!(fut, 10).expect("future to finish in 10s")
    }
    /// Remove advertisement on local peer.
    /// Will return a recent(not immediate) state change.
    pub async fn remove_advertised(&self, peer_id: &PeerId) -> bool {
        let ev = InEvent::RemoveAdvertised(*peer_id);
        let mut listener = self.event_tx.subscribe();
        let fut = listen_event!(listener for Advertise,
            OutEvent::AdvertisedPeerChanged(target,state)=>{
                if *target == *peer_id{
                    return *state
                }
        });
        self.sender.send(ev).await.expect("Send to succeed");
        with_timeout!(fut, 10).expect("Future to finish in 10s")
    }
    generate_handler_method!(
        /// List all peers that supports and connected to this peer.
        /// This call cannot fail.
        ListConnected:list_connected()->Box<[PeerId]>;
        /// List all advertisement on local peer.
        ListAdvertised:list_advertised()->Box<[PeerId]>;
    );
    generate_handler_method!(
        /// Clear all advertisements on local peer.
        /// This call cannot fail.
        ClearAdvertised:clear_advertised();
    );
    fn next_id(&self) -> u64 {
        use std::sync::atomic::Ordering;
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

pub(crate) mod cli {
    use super::*;
    use crate::net::p2p::swarm::Manager;
    use clap::Subcommand;
    use libp2p::PeerId;
    use tokio::runtime::Handle as RtHandle;

    #[derive(Debug, Subcommand)]
    pub enum Advertise {
        #[command(subcommand)]
        Provider(provider::Provider),
        SetRemoteAdvertisement {
            remote: PeerId,
            state: bool,
        },
        QueryAdvertised {
            remote: PeerId,
        },
    }

    pub fn handle_advertise(manager: &Manager, command: Advertise) {
        let handle = manager.advertise();
        let executor = manager.executor();
        use Advertise::*;
        match command {
            Provider(command) => provider::handle_provider(handle, executor, command),
            SetRemoteAdvertisement { remote, state } => {
                executor.block_on(handle.set_remote_advertisement(remote, state));
                println!("OK")
            }
            QueryAdvertised { remote } => {
                let result = executor.block_on(handle.query_advertised_peer(remote));
                println!("Query result:{:?}", result)
            }
        }
    }

    mod provider {
        use clap::{arg, Subcommand};

        #[derive(Debug, Subcommand)]
        pub enum Provider {
            Start,
            Stop,
            State,
            ListAdvertised,
            RemoveAdvertise {
                #[arg(required = true)]
                peer: PeerId,
            },
            ClearAdvertised,
        }

        use super::*;
        pub fn handle_provider(handle: &Handle, executor: &RtHandle, command: Provider) {
            use Provider::*;
            match command {
                Start => {
                    executor.block_on(handle.set_provider_state(true));
                    println!("Local provider is now providing advertisement");
                }
                Stop => {
                    executor.block_on(handle.set_provider_state(false));
                    println!("Local provider has stopped providing advertisement")
                }
                State => {
                    let state = executor.block_on(handle.provider_state());
                    println!("isProviding:{}", state)
                }
                ListAdvertised => {
                    let list = executor.block_on(handle.list_advertised());
                    println!("Advertising: \n{:?}", list);
                }
                RemoveAdvertise { peer } => {
                    executor.block_on(handle.remove_advertised(&peer));
                    println!("Advertisement for peer {} is removed", peer)
                }
                ClearAdvertised => {
                    executor.block_on(handle.clear_advertised());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::net::p2p::test_suit::setup_default;
    use libp2p::Multiaddr;
    use std::{thread, time::Duration};
    use tracing_log::log::trace;

    #[test]
    fn test() {
        setup_logging();
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        peer1_m
            .swarm()
            .listen_blocking(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
            .unwrap();
        trace!("peer 1 is listening");
        thread::sleep(Duration::from_millis(100));
        let peer1_id = peer1_m.identity().get_peer_id();
        let peer2_id = peer2_m.identity().get_peer_id();
        peer2_m
            .swarm()
            .dial_blocking(&peer1_m.swarm().list_listeners_blocking()[0])
            .unwrap();
        trace!("peer 1 dialed");
        thread::sleep(Duration::from_millis(200));
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.advertise().set_provider_state(true)));
        trace!("provider state set");
        thread::sleep(Duration::from_millis(200));
        peer2_m
            .executor()
            .block_on(peer2_m.advertise().set_remote_advertisement(peer1_id, true));
        assert!(peer2_m.swarm().is_connected_blocking(peer1_id));
        trace!("peer 1 connected and advertisement set");
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m
            .executor()
            .block_on(peer2_m.advertise().query_advertised_peer(peer1_id))
            .unwrap()
            .unwrap()
            .contains(&peer2_id));
        trace!("found advertisement for peer2 on peer1");
        assert!(!peer1_m
            .executor()
            .block_on(peer1_m.advertise().set_provider_state(false)));
        thread::sleep(Duration::from_millis(200));
        trace!("provider state of peer1 set to false");
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.advertise().query_advertised_peer(peer1_id))
                .unwrap()
                == None
        );
        trace!("advertisement no longer available");
        peer2_m.executor().block_on(
            peer2_m
                .advertise()
                .set_remote_advertisement(peer1_id, false),
        );
        trace!("removed advertisement on peer1(testing presistence)");
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.advertise().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        trace!("turned peer1 provider back on");
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.advertise().query_advertised_peer(peer1_id))
                .unwrap()
                .unwrap()
                .len()
                == 0
        );
    }

    fn setup_logging() {
        use std::sync::Mutex;
        use tracing::Level;
        use tracing_log::LogTracer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Layer;
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("owlnest", Level::INFO)
            .with_target("owlnest::net::p2p::protocols::advertise", Level::TRACE)
            .with_target("owlnest_advertise", Level::TRACE)
            .with_target("", Level::WARN);
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_ansi(false)
            .with_writer(Mutex::new(std::io::stdout()))
            .with_filter(filter);
        let reg = tracing_subscriber::registry().with(layer);
        tracing::subscriber::set_global_default(reg).expect("you can only set global default once");
        LogTracer::init().unwrap()
    }
}
