/// A wrapper around reference implementation of libp2p identify protocol.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
pub mod identify;

/// A wrapper around reference implementation of libp2p kademila protocol.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
pub mod kad;

/// A wrapper around reference implementation of libp2p mdns protocol.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
pub mod mdns;

/// A behaviour that allows peers to directly exchange human-readable message,
/// similar to instant messaging(IM) service.
#[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
pub mod messaging;

// /// A behaviour that allows remote control of a node.
// pub mod tethering;

/// A wrapper around reference implementation of libp2p relay client.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-client"))]
pub mod relay_client;

/// A wrapper around reference implementation of libp2p relay server.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
pub mod relay_server;

/// An extension on top reference implementation of libp2p relay server and client.
#[cfg(any(feature = "owlnest-protocols", feature = "owlnest-advertise"))]
pub mod advertise;

// /// A mock up of reference implementation of libp2p allow-block list.
// pub mod allow_block_list;

/// A wrapper around reference implementation of libp2p Direct Connection Upgrade through Relay.
#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-dcutr"))]
pub mod dcutr;

/// A wrapper around reference implementation of libp2p Autonat
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

#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-upnp"))]
pub mod upnp;

#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-ping"))]
pub mod ping;

#[cfg(any(feature = "libp2p-protocols", feature = "libp2p-gossipsub"))]
pub mod gossipsub;

// pub mod hyper;

pub mod event_listener {}

#[allow(unused)]
const SUBSCRIBER_CONFLICT_ERROR_MESSAGE: &str =
    "You can only set global default once. Did you forget to remove some attached log subscribers?";
