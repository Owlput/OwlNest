use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

mod universal;

#[cfg(feature = "messaging")]
pub mod messaging;

#[cfg(feature = "tethering")]
pub mod tethering;

#[cfg(feature = "relay-client")]
pub mod relay_client;

#[cfg(feature = "relay-server")]
pub mod relay_server;
