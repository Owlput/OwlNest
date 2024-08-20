use crate::net::p2p::swarm::handle::SwarmHandle;
use clap::Subcommand;
use libp2p::{Multiaddr, PeerId, TransportError};
use prettytable::table;

/// Subcommand for managing the swarm.  
/// Swarm is libp2p way of managing raw connections(or transports, e.g. TCP, UDP, QUIC, WebSocket).
/// Every request to initate a connection to another peer must go through the swarm.
#[derive(Debug, Subcommand)]
pub enum Swarm {
    /// Dial the given address.
    /// This command will return immediately without confirming if the dial actually succeeded.
    Dial {
        /// The address to dial, in multiaddr format.
        /// e.g. /ip4/127.0.0.1/tcp/42
        /// Visit `https://github.com/libp2p/specs/blob/master/addressing/README.md#multiaddr-in-libp2p`
        /// for more info about multiaddress
        #[arg(required = true)]
        address: Multiaddr,
    },
    /// Listen on the given local address.
    /// For listening on a relay server, it is recommended to use relay-client subcommand instead.
    /// Will return the id of the first listener created.
    /// Note that the listener can be dropped afterwards.
    Listen {
        /// The address to listen on, in multiaddr format.  
        /// e.g. `/ip4/127.0.0.1/tcp/42`;  
        /// `/ip4/some_remote_ip/tcp/port_number/p2p/peer_id_of_relay/p2p-circuit`  
        /// Visit `https://github.com/libp2p/specs/blob/master/addressing/README.md#multiaddr-in-libp2p`
        /// for more info about multiaddress
        #[arg(required = true)]
        address: Multiaddr,
    },
    /// Subcommand for handling active listeners.
    /// To add new listeners, use `listen` instead.
    #[command(subcommand)]
    Listener(listener::Listener),
    /// Subcommand for managing confirmed external addresses
    /// e.g. addresses that can be reached by others.
    #[command(subcommand)]
    ExternalAddr(external_address::ExternalAddr),
    /// Check if the peer with the given ID is connected.
    IsConnected {
        /// The peer ID to check.
        #[arg(required = true)]
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
    use printable::iter::PrintableIter;

    use super::*;

    /// Subcommand for managing listeners.
    #[derive(Debug, Subcommand)]
    pub enum Listener {
        /// List all listeners
        Ls,
    }

    pub fn handle_swarm_listener(handle: &SwarmHandle, command: Listener) {
        use Listener::*;
        match command {
            Ls => {
                let list = handle.list_listeners_blocking();
                let printable_list = list.iter().printable().with_left_bound("").with_right_bound("").with_separator("\n");
                let table  = table!(["Active Listeners"], [printable_list]);
                table.printstd();
            }
        }
    }
}

pub mod external_address {
    use printable::iter::PrintableIter;

    use super::*;

    /// Subcommand for managing external addresses.
    /// Note that some protocol will use this information
    /// to determine whether the address is publicly reachable.  
    /// Local addresses(10.0.0.0,127.0.0.1,192.168.0.0) are not
    /// considered external addresses by default, you can manually
    /// add them to the list.
    #[derive(Debug, Subcommand)]
    pub enum ExternalAddr {
        /// List all addresses that are considered publicly reachable.
        Ls,
        /// Manually add an address to the list of external addresses.
        Add {
            /// The address to add.
            #[arg(required = true)]
            address: Multiaddr,
        },
        /// Manually remove an address to the list of external addresses.
        Remove {
            /// The address to remove.
            #[arg(required = true)]
            address: Multiaddr,
        },
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
                let list = handle.list_external_addresses_blocking();
                let table  = table!(["External Addresses"], [list.iter().printable()]);
                table.printstd();
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
