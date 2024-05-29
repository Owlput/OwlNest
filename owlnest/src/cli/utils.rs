use std::net::{SocketAddr, ToSocketAddrs};

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Utils {
    /// Performs a DNS lookup for the given domain name.
    /// This calls `domain_name.to_socket_addre()`(from `std::net`) internally.
    DnsLookup {
        /// The domain name to look up with.
        #[arg(required = true)]
        domian_name: String,
    },
}

pub fn handle_utils(command: Utils) {
    use Utils::*;
    match command {
        DnsLookup { domian_name } => {
            let domian_name_with_port = format!("{}:0", domian_name);
            let addresses = match domian_name_with_port.to_socket_addrs() {
                Ok(addr) => addr.collect::<Box<[SocketAddr]>>(),
                Err(e) => {
                    println!("Failed to perform lookup with {}: {:?}", domian_name, e);
                    return;
                }
            };
            println!(r"Lookup result: {:?}", addresses);
        }
    }
}
