use crate::net::p2p::swarm::handle::SwarmHandle;
use clap::Subcommand;
use libp2p::{Multiaddr, PeerId, TransportError};

#[derive(Debug, Subcommand)]
pub enum Swarm {
    Dial {
        address: Multiaddr,
    },
    Listen {
        address: Multiaddr,
    },
    #[command(subcommand)]
    Listener(listener::Listener),
    #[command(subcommand)]
    ExternalAddr(external_address::ExternalAddr),
    IsConnected {
        peer_id: PeerId,
    },
}

pub(crate) fn handle_swarm(handle: &SwarmHandle, command: Swarm) {
    use Swarm::*;
    match command {
        Dial { address } => {
            if let Err(e) = handle.dial_blocking(&address) {
                println!("Failed to initiate dial {} with error: {:?}", address, e);
            } else {
                println!("Dialing {}", address);
            }
        }
        Listen { address } => match handle.listen_blocking(&address) {
            Ok(listener_id) => println!(
                "Successfully listening on {} with listener ID {:?}",
                address, listener_id
            ),

            Err(e) => println!(
                "Failed to listen on {} with error: {}",
                address,
                format_transport_error(e)
            ),
        },
        Listener(command) => listener::handle_swarm_listener(handle, command),
        ExternalAddr(command) => external_address::handle_swarm_externaladdress(handle, command),
        IsConnected { peer_id } => println!("{}", handle.is_connected_blocking(peer_id)),
    }
}

pub mod listener {
    use super::*;
    #[derive(Debug, Subcommand)]
    pub enum Listener {
        Ls,
    }

    pub fn handle_swarm_listener(handle: &SwarmHandle, command: Listener) {
        use Listener::*;
        match command {
            Ls => println!("Active listeners: {:?}", handle.list_listeners_blocking()),
        }
    }
}

pub mod external_address {
    use super::*;
    #[derive(Debug, Subcommand)]
    pub enum ExternalAddr {
        Ls,
        Add { address: Multiaddr },
        Remove { address: Multiaddr },
    }
    pub fn handle_swarm_externaladdress(handle: &SwarmHandle, command: ExternalAddr) {
        use ExternalAddr::*;
        match command {
            Add { address } => {
                handle.add_external_address_blocking(address.clone());
                println!("External address `{}` added", address)
            }
            Remove { address } => {
                handle.remove_external_address_blocking(address.clone());
                println!("External address `{}` removed", address)
            }
            Ls => {
                let addresses = handle.list_external_addresses_blocking();
                println!("External addresses: {:?}", addresses)
            }
        }
    }
}

pub(crate) fn format_transport_error(e: TransportError<std::io::Error>) -> String {
    match e {
        TransportError::MultiaddrNotSupported(addr) => {
            format!("Requested address {} is not supported.", addr)
        }
        TransportError::Other(e) => {
            let error_string = format!("{:?}", e);
            if error_string.contains("AddrNotAvailable") {
                return "Local interface associated with the given address does not exist".into();
            }
            error_string
        }
    }
}
