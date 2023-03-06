use std::str::FromStr;

use super::*;
use libp2p::PeerId;

use crate::net::p2p::{
    identity::IdentityUnion,
    swarm::{self, BehaviourOp},
};

pub fn handle_messaging(manager: &swarm::Manager, ident: &IdentityUnion, command: Vec<&str>) {
    if command.len() < 2 {
        println!("Failed to execute: missing subcommands.");
        println!("{}", TOP_HELP_MESSAGE);
    }
    match command[1] {
        "send" => handle_message_send(manager, ident, command),
        "help" => println!(
            "Protocol {}/n{}",
            String::from_utf8(protocol::PROTOCOL_NAME.to_vec()).unwrap(),
            TOP_HELP_MESSAGE
        ),
        _ => println!(
            "Failed to execute: unrecognized subcommand.\n{}",
            TOP_HELP_MESSAGE
        ),
    }
}

pub fn handle_message_send(manager: &swarm::Manager, ident: &IdentityUnion, command: Vec<&str>) {
    if command.len() < 4 {
        println!("Error: Missing required argument(s), syntax: `msg <peer id> <message>`");
        return;
    }
    let target_peer = match PeerId::from_str(command[1]) {
        Ok(addr) => addr,
        Err(e) => {
            println!("Error: Failed parsing peer ID `{}`: {}", command[1], e);
            return;
        }
    };
    let op = BehaviourOp::Messaging(Op::SendMessage(
        target_peer.clone(),
        Message::new(
            &ident.get_peer_id(),
            &target_peer,
            command.split_at(3).1.join(" "),
        ),
    ));
    let result = manager.blocking_behaviour_exec(op);
    if let BehaviourOpResult::Messaging(op_result) = result {
        match op_result {
            OpResult::SuccessfulPost(rtt) => println!(
                "Message successfully sent, estimated round trip time {}ms",
                rtt.as_millis()
            ),
            OpResult::Error(e) => println!("Failed to send message: {}", e),
        }
    }
    todo!()
}

const TOP_HELP_MESSAGE: &str = r#"
Sending messages to the given peer.

Available subcommands:
    send <peer ID> <message>
                Send the message to the given peer.
    help
                Show this help message.
"#;
