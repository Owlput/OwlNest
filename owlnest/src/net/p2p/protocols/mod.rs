/// A wrapper around reference implementation of libp2p identify protocol.
pub mod identify;

/// A wrapper around reference implementation of libp2p kademila protocol.
pub mod kad;

/// A wrapper around reference implementation of libp2p mdns protocol.
pub mod mdns;

/// A behaviour that allows peers to directly exchange human-readable message,
/// similar to instant messaging(IM) service.
pub mod messaging;

/// A behaviour that allows remote control of a node. 
// pub mod tethering;

/// A wrapper around reference implementation of libp2p relay client.
pub mod relay_client;

/// A wrapper around reference implementation of libp2p relay server.
pub mod relay_server;

/// An extension on top reference implementation of libp2p relay server and client.
pub mod relay_ext;

// /// A mock up of reference implementation of libp2p allow-block list. 
// pub mod allow_block_list;

/// A wrapper around reference implementation of libp2p Direct Connection Upgrade through Relay.
pub mod dcutr;

/// A wrapper around reference implementation of libp2p Autonat
pub mod autonat;

/// A protocol for sending large binary data.
/// 
/// A send request will be cancelled under these senarios:
/// - Sender called `Behaviour::cancel_send()`;
/// - Error occurred when the sender is trying to read the file;
/// - Receiver called `Behaviour::cancel_recv()`;
pub mod blob_transfer;

pub mod upnp;

pub mod ping;

// pub mod hyper;

pub mod event_listener {}
#[allow(unused)]
mod universal;
