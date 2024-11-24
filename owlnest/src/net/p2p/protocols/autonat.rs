use crate::net::p2p::swarm::SwarmEvent;
pub use libp2p::autonat::Behaviour;
pub use libp2p::autonat::Config;
pub use libp2p::autonat::Event as OutEvent;
pub use libp2p::autonat::NatStatus;
use libp2p::{Multiaddr, PeerId};
use owlnest_macro::generate_handler_method;
use owlnest_macro::handle_callback_sender;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::info;

#[derive(Debug)]
pub(crate) enum InEvent {
    AddServer(PeerId, Option<Multiaddr>),
    RemoveServer(PeerId),
    Probe(Multiaddr),
    GetNatStatus(oneshot::Sender<(NatStatus, usize)>),
}

/// A handle that can communicate with the behaviour within the swarm.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Handle {
    swarm_source: broadcast::Sender<Arc<SwarmEvent>>,
    sender: mpsc::Sender<InEvent>,
}
impl Handle {
    pub(crate) fn new(
        buffer: usize,
        swarm_source: &broadcast::Sender<Arc<SwarmEvent>>,
    ) -> (Self, mpsc::Receiver<InEvent>) {
        let (tx, rx) = mpsc::channel(buffer);
        (
            Self {
                swarm_source: swarm_source.clone(),
                sender: tx,
            },
            rx,
        )
    }
    generate_handler_method!(
        /// Add a server(endpoint) that the behaviour can use to probe(test)
        /// for public reachability.
        AddServer:add_server(peer:PeerId,address:Option<Multiaddr>,);
        /// Remove a server(endpoint) the behaviour can use.
        RemoveServer:remove_server(peer:PeerId);
        /// Tell the behaivour to probe the endpoint now.
        Probe:probe(candidate:Multiaddr);
    );
    generate_handler_method!(GetNatStatus:get_nat_status()->(NatStatus,usize););
}

pub(crate) fn map_in_event(behaviour: &mut Behaviour, ev: InEvent) {
    match ev {
        InEvent::AddServer(peer, address) => behaviour.add_server(peer, address),
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
