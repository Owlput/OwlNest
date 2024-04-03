/// A wrapper around reference implementation of libp2p identify protocol.
#[cfg(feature = "libp2p-protocols")]
pub mod identify;

/// A wrapper around reference implementation of libp2p kademila protocol.
#[cfg(feature = "libp2p-protocols")]
pub mod kad;

/// A wrapper around reference implementation of libp2p mdns protocol.
#[cfg(feature = "libp2p-protocols")]
pub mod mdns;

/// A behaviour that allows peers to directly exchange human-readable message,
/// similar to instant messaging(IM) service.
#[cfg(feature = "owlnest-protocols")]
pub mod messaging;

/// A behaviour that allows remote control of a node. 
// pub mod tethering;

/// A wrapper around reference implementation of libp2p relay client.
#[cfg(feature = "libp2p-protocols")]
pub mod relay_client;

/// A wrapper around reference implementation of libp2p relay server.
#[cfg(feature = "libp2p-protocols")]
pub mod relay_server;

/// An extension on top reference implementation of libp2p relay server and client.
#[cfg(feature = "owlnest-protocols")]
pub mod advertise;

// /// A mock up of reference implementation of libp2p allow-block list. 
// pub mod allow_block_list;

/// A wrapper around reference implementation of libp2p Direct Connection Upgrade through Relay.
#[cfg(feature = "libp2p-protocols")]
pub mod dcutr;

/// A wrapper around reference implementation of libp2p Autonat
#[cfg(feature = "libp2p-protocols")]
pub mod autonat;

/// A protocol for sending large binary data.
/// 
/// A send request will be cancelled under these senarios:
/// - Sender called `Behaviour::cancel_send()`;
/// - Error occurred when the sender is trying to read the file;
/// - Receiver called `Behaviour::cancel_recv()`;
#[cfg(feature = "owlnest-protocols")]
pub mod blob;

#[cfg(feature = "libp2p-protocols")]
pub mod upnp;

#[cfg(feature = "libp2p-protocols")]
pub mod ping;

// pub mod hyper;

pub mod event_listener {}
