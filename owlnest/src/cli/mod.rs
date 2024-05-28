mod utils;

use crate::net::p2p::protocols::*;
use crate::net::p2p::swarm::cli::format_transport_error;
use crate::net::p2p::swarm::manager::Manager;
use crate::net::p2p::{identity::IdentityUnion, swarm};
use clap::{Parser, Subcommand};
use crossterm::style::Stylize;
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;
use libp2p::Multiaddr;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::io::stdout;
use std::sync::Arc;
use tokio::sync::Notify;

use self::utils::handle_utils;

/// Make current terminal interactive
pub fn setup_interactive_shell(
    ident: IdentityUnion,
    manager: Manager,
    shutdown_notifier: Arc<Notify>,
) {
    std::thread::spawn(move || {
        stdout().execute(Clear(ClearType::All)).unwrap();
        println!("OwlNest is now running in interactive mode, type \"help\" for more information.");
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
        messaging::cli::setup(&manager);
        let mut rl = DefaultEditor::new().unwrap();
        let mut retry_times = 0u32;
        loop {
            let line_read = rl.readline(&">> ".stylize().dark_cyan().to_string());
            match line_read {
                Ok(line) => {
                    rl.add_history_entry(line.as_str()).unwrap();
                    handle_command(line, &manager, &ident, &shutdown_notifier)
                }
                Err(e) => {
                    if handle_err(e, &mut retry_times) {
                        break;
                    }
                }
            }
        }
        println!("shutdown criteria reached, shutting down this peer...");
        shutdown_notifier.notify_one()
    });
}

fn handle_command(
    line: String,
    manager: &Manager,
    ident: &IdentityUnion,
    shutdown_notifier: &Arc<Notify>,
) {
    let line_with_owlnest = format!("owlnest {}", line);
    let commands = match shlex::split(line_with_owlnest.trim()) {
        Some(v) => v,
        None => {
            println!(r#"Cannot properly split "{}": unclosed delimiters"#, line);
            return;
        }
    };
    let command = match Cli::try_parse_from(commands.iter()) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };
    let handle = manager.swarm();
    use Command::*;
    match command.command {
        Clear => drop(stdout().execute(crossterm::terminal::Clear(
            crossterm::terminal::ClearType::FromCursorUp,
        ))),
        Id => println!("Local peer ID: {}", ident.get_peer_id()),
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
        Shutdown => {
            println!("Shutting down...");
            shutdown_notifier.notify_one()
        }
        Swarm(command) => swarm::cli::handle_swarm(manager.swarm(), command),
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
        Messaging(command) => messaging::cli::handle_messaging(manager, ident, command),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
        Kad(command) => kad::cli::handle_kad(manager, command),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
        Mdns(command) => mdns::cli::handle_mdns(manager, command),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-client"))]
        RelayClient(command) => relay_client::cli::handle_relayclient(manager, command),
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
        Advertise(command) => advertise::cli::handle_advertise(manager, command),
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-blob"))]
        Blob(command) => blob::cli::handle_blob(manager, command),
        Utils(command) => handle_utils(command),
    }
}

fn handle_err(e: ReadlineError, retry_times: &mut u32) -> bool {
    match e {
        ReadlineError::Io(e) => {
            println!("Failed to read input with IO error: {:?}", e);
            should_exit(retry_times, 5)
        }
        ReadlineError::Eof => {
            println!("Failed to read input: Unexpected EOF");
            should_exit(retry_times, 5)
        }
        ReadlineError::Interrupted => {
            println!("Interrupt signal received.");
            true
        }
        ReadlineError::WindowResized => false,
        _ => {
            println!("Unknown error occurred");
            should_exit(retry_times, 5)
        }
    }
}

fn should_exit(retry_times: &mut u32, max_retry_times: u32) -> bool {
    *retry_times += 1;
    if *retry_times >= max_retry_times {
        println!("Maximum retry attempts reached, exiting interactive shell. To exit the process, send interrupt signal.");
        true
    } else {
        false
    }
}

#[derive(Parser)]
#[command(name = "owlnest")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Id,
    Clear,
    Shutdown,
    Dial {
        address: Multiaddr,
    },
    Listen {
        address: Multiaddr,
    },
    #[command(subcommand)]
    Swarm(swarm::cli::Swarm),
    #[command(subcommand)]
    Kad(kad::cli::Kad),
    #[command(subcommand)]
    RelayClient(relay_client::cli::RelayClient),
    #[command(subcommand)]
    Messaging(messaging::cli::Messaging),
    #[command(subcommand)]
    Mdns(mdns::cli::Mdns),
    #[command(subcommand)]
    Blob(blob::cli::Blob),
    #[command(subcommand)]
    Advertise(advertise::cli::Advertise),
    #[command(subcommand)]
    Utils(utils::Utils),
}
