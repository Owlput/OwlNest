use libp2p::{PeerId,Multiaddr};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use std::fmt::Display;
use std::time::Duration;

#[cfg(feature = "messaging")]
pub mod messaging;

#[cfg(feature = "tethering")]
pub mod tethering;

#[cfg(feature="relay-client")]
pub mod relay_client;

#[cfg(feature="relay-server")]
pub mod relay_server;

