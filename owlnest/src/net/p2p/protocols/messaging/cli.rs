use super::{Message, Op, OpResult, PROTOCOL_NAME};
use crate::net::p2p::identity::IdentityUnion;
use crate::net::p2p::swarm::Manager;

pub fn handle_messaging(manager: &Manager, ident: &IdentityUnion, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Failed to execute: missing subcommands.");
        println!("{}", TOP_HELP_MESSAGE);
        return;
    }
    match command[1] {
        "send" => handle_message_send(manager, ident, command),
        "help" => println!("Protocol {}/n{}", PROTOCOL_NAME, TOP_HELP_MESSAGE),
        _ => println!(
            "Failed to execute: unrecognized subcommand.\n{}",
            TOP_HELP_MESSAGE
        ),
    }
}

pub fn handle_message_send(manager: &Manager, ident: &IdentityUnion, command: Vec<&str>) {
    if command.len() < 4 {
        println!(
            "Error: Missing required argument(s), syntax: `messaging send <peer id> <message>`"
        );
        return;
    }
    let target_peer: libp2p::PeerId = match std::str::FromStr::from_str(command[2]) {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[2], e);
            return;
        }
    };
    let op = Op::SendMessage(
        target_peer,
        Message::new(
            ident.get_peer_id(),
            target_peer,
            command.split_at(3).1.join(" "),
        ),
    );
    let result = manager.blocking_behaviour_exec(op.into()).try_into();
    if let Ok(result) = result {
        match result {
            OpResult::SuccessfulPost(rtt) => println!(
                "Message successfully sent, estimated round trip time {}ms",
                rtt.as_millis()
            ),
            OpResult::Error(e) => println!("Failed to send message: {}", e),
        }
    }
}

const TOP_HELP_MESSAGE: &str = r#"
Sending messages to the given peer.

Available subcommands:
    send <peer ID> <message>
                Send the message to the given peer.

    help
                Show this help message.
"#;
