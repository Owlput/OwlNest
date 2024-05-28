use std::net::{SocketAddr, ToSocketAddrs};

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Utils {
    DnsLookup { domian_name: String },
}

pub fn handle_utils(command: Utils) {
    use Utils::*;
    match command {
        DnsLookup { domian_name } => {
            let addresses = match domian_name.to_socket_addrs() {
                Ok(addr) => addr.collect::<Box<[SocketAddr]>>(),
                Err(e) => {
                    println!("Failed to perform lookup: {:?}", e);
                    println!("Hint: You may have missed the port number. You can try out port 0.");
                    return;
                }
            };
            println!(r"Lookup result: {:?}", addresses);
        }
    }
}
