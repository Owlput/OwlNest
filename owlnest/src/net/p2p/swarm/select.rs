use crate::connection_handler_select;
use crate::net::p2p::protocols::*;

connection_handler_select! {
    tethering=>Tethering:<tethering::Behaviour as NetworkBehaviour>::ConnectionHandler,
    messaging=>Messaging:<messaging::Behaviour as NetworkBehaviour>::ConnectionHandler,
}