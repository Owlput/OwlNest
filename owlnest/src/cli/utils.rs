use std::net::{SocketAddr, ToSocketAddrs};

pub fn handle_utils(command: Vec<&str>) {
    if command.len() < 2 {
        println!("No subcommands supplied. Type \"utils help\" for more information.")
    }
    match command[1] {
        "dns" => handle_utils_dns(command),
        "help" => println!("{}", TOP_HELP_MESSAGE),
        _ => println!("Unrecognized command. Type \"utils help\" for more information."),
    }
}

fn handle_utils_dns(command: Vec<&str>) {
    if command.len() < 3 {
        println!("No subcommands supplied. Type \"utils help\" for more information.")
    }
    match command[2] {
        "lookup" => {
            if command.len() < 4{
                println!("Failed to perform lookup: missing required argument <domain name>.")
            }
            let addresses = match command[3].to_socket_addrs() {
                Ok(addr) => addr.collect::<Box<[SocketAddr]>>(),
                Err(e) => {
                    println!("Failed to perform lookup: {:?}", e);
                    println!("Hint: You may have missed port number");
                    return;
                }
            };
            println!(r"Lookup result: {:?}", addresses);
        }
        "help" => println!("{}", DNS_HELP_MESSAGE),
        _ => println!("Unrecognized command. Type \"utils dns help\" for more information"),
    }
}

const TOP_HELP_MESSAGE: &str = r#"
utils: command line utilities

Available subcommands:
    dns     Perform DNS operations for the given domain name.

"#;

const DNS_HELP_MESSAGE: &str = r#"

Available subcommands:
    lookup <domain-name:port>
                            Lookup the given domain name using
                            Domain Name System.

"#;
