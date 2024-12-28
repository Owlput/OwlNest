/// # Libp2p Identify
/// Protocol name: `/ipfs/id/1.0.0`  
/// Support status: Tier 1
/// ## About this protocol
/// Identify facilitates identity(public key and agent) and ability(supported protocols)
/// information exchange.  
/// Peers will exchange those information periodically(configurable).
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
pub mod identify;

/// # Libp2p Kadelima
/// Protocol name: `/ipfs/kad/1.0.0`  
/// Support status: Tier 1
/// ## About this protocol
/// Kadelima facilitates routing information exchange between peers.
/// Only peers that are considered publicly reachable will be shared between
/// peers connected to the local node.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
pub mod kad;

/// # Libp2p mDNS
/// Support status: Tier 2
/// ## About this protocol
/// mDNS(multicast DNS) facilitates peer discovery in local network.
/// Using multicasting, peers can broadcast their hostname and IP addresses
/// automatically without any human intervention.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
pub mod mdns;

/// # Owlnest Messaging  
/// Protocol name: `/owlnest/messaging/0.0.1`
/// Support status: Tier 1  
/// ## About this protocol
/// Messaging facilitates direct messaging service between connected peers
/// with flexibility and reliability.
#[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
pub mod messaging;

// /// A behaviour that allows remote control of a node.
// pub mod tethering;

/// # Libp2p Relay(client)
/// Supoort status: Tier 1  
/// ## About this protocol  
/// Allow local peer to connect to a relay(lighthouse) to extend connectivity.
/// ## Security & Privacy
/// The relay cannot decrypt messages(bytes) send through the relay connection.
/// But the IP addresses are known for both ends.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-client"))]
pub mod relay_client;

/// # Libp2p Relay(server)  
/// Support status: Tier 1  
/// ## About this protocol  
/// Allow local peer to function as a relay(lighthouse) to provide
/// extended connectivity for other peers.
/// ## Security & Privacy
/// The relay cannot decrypt messages(bytes) send through the relay connection.
/// But the IP addresses are known for both ends.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
pub mod relay_server;

/// A behaviour that allows automated information sharing between peers.
#[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
pub mod advertise;

// /// A mock up of reference implementation of libp2p allow-block list.
// pub mod allow_block_list;

/// An adapter for reference implementation of libp2p Direct Connection Upgrade through Relay.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-dcutr"))]
pub mod dcutr;

/// An adapter for reference implementation of libp2p Autonat
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-autonat"))]
pub mod autonat;

/// A protocol for sending large binary data.
///
/// A send request will be cancelled under these senarios:
/// - Sender called `Behaviour::cancel_send()`;
/// - Error occurred when the sender is trying to read the file;
/// - Receiver called `Behaviour::cancel_recv()`;
#[cfg(any(feature = "owlnest-protocols", feature = "owlnest-blob"))]
pub mod blob;

/// UPnP - Universal Plug and Play
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-upnp"))]
pub mod upnp;

/// An adapter for reference implememtation of libp2p ping
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-ping"))]
pub mod ping;

/// # Gossipsub
/// Protocol name: `/meshsub/1.1.0`
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"))]
pub mod gossipsub;

// pub mod hyper;

#[allow(unused)]
const SUBSCRIBER_CONFLICT_ERROR_MESSAGE: &str =
    "You can only set global default once. Did you forget to remove some attached log subscribers?";

use crate::net::p2p::swarm::EventSender;
use crate::{future_timeout, handle_callback, send_swarm};
use libp2p::{Multiaddr, PeerId};
use owlnest_core::alias::Callback;
use owlnest_macro::{generate_handler_method, handle_callback_sender, listen_event};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
