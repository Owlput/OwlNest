use crate::net::p2p::swarm::EventSender;
pub use config::Config;
pub use libp2p::autonat::Behaviour;
pub use libp2p::autonat::Event as OutEvent;
pub use libp2p::autonat::NatStatus;
use libp2p::{Multiaddr, PeerId};
use owlnest_core::alias::Callback;
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug)]
pub(crate) enum InEvent {
    AddServer {
        peer_id: PeerId,
        address: Option<Multiaddr>,
    },
    RemoveServer(PeerId),
    Probe(Multiaddr),
    GetNatStatus(Callback<(NatStatus, usize)>),
}

/// A handle that can communicate with the behaviour within the swarm.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Handle {
    swarm_source: EventSender,
    sender: mpsc::Sender<InEvent>,
}
impl Handle {
    pub(crate) fn new(
        _config: &Config,
        buffer_size: usize,
        swarm_event_source: &EventSender,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        (
            Self {
                swarm_source: swarm_event_source.clone(),
                sender: tx,
            },
            rx,
        )
    }
    generate_handler_method!(
        /// Remove a server(peer) the behaviour can use.
        RemoveServer:remove_server(peer:PeerId);
        /// Tell the behaivour to probe the endpoint now.
        Probe:probe(candidate:Multiaddr);
    );
    generate_handler_method!(GetNatStatus:get_nat_status()->(NatStatus,usize););
    generate_handler_method!(
        /// Add a server(peer) that the behaviour can use to probe
        /// for public reachability.
        AddServer:add_server{peer_id:PeerId,address:Option<Multiaddr>};
    );
}

pub(crate) fn map_in_event(behaviour: &mut Behaviour, ev: InEvent) {
    match ev {
        InEvent::AddServer { peer_id, address } => behaviour.add_server(peer_id, address),
        InEvent::RemoveServer(peer) => behaviour.remove_server(&peer),
        InEvent::GetNatStatus(callback) => {
            handle_callback_sender!((behaviour.nat_status(),behaviour.confidence())=>callback)
        }
        InEvent::Probe(candidate) => behaviour.probe_address(candidate),
    }
}

pub(crate) fn ev_dispatch(ev: &OutEvent) {
    match ev {
        OutEvent::InboundProbe(probe) => info!("Incoming probe: {:?}", probe),
        OutEvent::OutboundProbe(probe) => info!("Probe sent: {:?}", probe),
        OutEvent::StatusChanged { old, new } => {
            info!("Nat statue changed from {:?} to {:?}", old, new)
        }
    }
}

/// Adapter for the intergeated command line interface.
pub mod cli {
    use clap::Subcommand;
    use libp2p::{Multiaddr, PeerId};

    use super::Handle;

    /// Command for managing `libp2p-autonat` protocol.
    #[derive(Debug, Subcommand)]
    pub enum AutoNat {
        /// Add a server to be periodically probed to check for NAT status.
        AddServer {
            /// The peer to act as the server.
            #[arg(required = true)]
            peer_id: PeerId,
            /// The address to probe, in multiaddr format.
            #[arg(required = false)]
            address: Option<Multiaddr>,
        },
        /// Remove a server that will be periodcally probed.
        RemoveServer {
            /// The server to remove.
            #[arg(required = true)]
            peer_id: PeerId,
        },
        /// Manually oneshot probing the address.
        /// This command will return immediately without reporting the result.
        /// Use `autonat get-nat-status` to get the updated NAT status.
        Probe {
            /// The address to probe, in multiaddr format.
            #[arg(required = true)]
            address: Multiaddr,
        },
        /// Get current NAT status and it confidence score.  
        /// `Private` for behind NAT walls, `Public` for publicly available,
        /// `Unknown` for unknown status.
        /// The confidence score is simply the number of servers
        /// that report the current status.
        GetNatStatus,
    }

    /// The function used to handle the first level of `autonat` command.
    pub async fn handle_autonat(handle: &Handle, command: AutoNat) {
        match command {
            AutoNat::AddServer { peer_id, address } => {
                handle.add_server(peer_id, address).await;
                println!("OK.");
            }
            AutoNat::RemoveServer { peer_id } => {
                handle.remove_server(peer_id).await;
                println!("OK.");
            }
            AutoNat::Probe { address } => {
                handle.probe(address).await;
                println!("OK.");
            }
            AutoNat::GetNatStatus => {
                let (status, confidence) = handle.get_nat_status().await;
                use super::NatStatus::*;
                match status {
                    Private => println!("NAT status: Private; Confidence: {}", confidence),
                    Public(addr) => println!(
                        "NAT status: Public; Public address: {}, Confidence: {}",
                        addr, confidence
                    ),
                    Unknown => println!("NAT status: Unknown"),
                }
            }
        }
    }
}

pub mod config {
    use serde::{Deserialize, Serialize};

    /// Config for the [`Behaviour`].
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Config {
        /// Timeout for requests.
        pub timeout_sec: u64,

        // Client Config
        /// Delay on init before starting the fist probe.
        pub boot_delay_sec: u64,
        /// Interval in which the NAT should be tested again if max confidence was reached in a status.
        pub refresh_interval_sec: u64,
        /// Interval in which the NAT status should be re-tried if it is currently unknown
        /// or max confidence was not reached yet.
        pub retry_interval_sec: u64,
        /// Throttle period for re-using a peer as server for a dial-request.
        pub throttle_server_period_sec: u64,
        /// Use connected peers as servers for probes.
        pub use_connected: bool,
        /// Max confidence that can be reached in a public / private NAT status.
        /// Note: for [`NatStatus::Unknown`] the confidence is always 0.
        pub confidence_max: usize,

        // Server Config
        /// Max addresses that are tried per peer.
        pub max_peer_addresses: usize,
        /// Max total dial requests done in `[Config::throttle_clients_period`].
        pub throttle_clients_global_max: usize,
        /// Max dial requests done in `[Config::throttle_clients_period`] for a peer.
        pub throttle_clients_peer_max: usize,
        /// Period for throttling clients requests.
        pub throttle_clients_period_sec: u64,
        /// As a server reject probes for clients that are observed at a non-global ip address.
        /// Correspondingly as a client only pick peers as server that are not observed at a
        /// private ip address. Note that this does not apply for servers that are added via
        /// [`Behaviour::add_server`].
        pub only_global_ips: bool,
    }

    impl Default for Config {
        fn default() -> Self {
            Config {
                timeout_sec: 30,                // 30 sec
                boot_delay_sec: 15,             // 15 sec
                retry_interval_sec: 90,         // 90 sec
                refresh_interval_sec: 15 * 60,  // 15 min
                throttle_server_period_sec: 90, // 90 sec
                use_connected: true,
                confidence_max: 3,
                max_peer_addresses: 16,
                throttle_clients_global_max: 30,
                throttle_clients_peer_max: 3,
                throttle_clients_period_sec: 1, // 1 sec
                only_global_ips: true,
            }
        }
    }

    impl From<libp2p::autonat::Config> for Config {
        fn from(value: libp2p::autonat::Config) -> Self {
            let libp2p::autonat::Config {
                timeout,
                boot_delay,
                refresh_interval,
                retry_interval,
                throttle_server_period,
                use_connected,
                confidence_max,
                max_peer_addresses,
                throttle_clients_global_max,
                throttle_clients_peer_max,
                throttle_clients_period,
                only_global_ips,
            } = value;
            Self {
                timeout_sec: timeout.as_secs(),
                boot_delay_sec: boot_delay.as_secs(),
                refresh_interval_sec: refresh_interval.as_secs(),
                retry_interval_sec: retry_interval.as_secs(),
                throttle_server_period_sec: throttle_server_period.as_secs(),
                use_connected,
                confidence_max,
                max_peer_addresses,
                throttle_clients_global_max,
                throttle_clients_peer_max,
                throttle_clients_period_sec: throttle_clients_period.as_secs(),
                only_global_ips,
            }
        }
    }
}
