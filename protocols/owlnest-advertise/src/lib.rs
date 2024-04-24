use owlnest_prelude::lib_prelude::*;
use serde::{Deserialize, Serialize};

pub mod behaviour;
mod handler;

pub use behaviour::Behaviour;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutEvent {
    /// A query sent to a remote peer is answered.
    QueryAnswered {
        from: PeerId,
        list: Vec<PeerId>,
    },
    /// A advertisement result from remote peer arrived.
    RemoteAdvertisementResult {
        from: PeerId,
        result: Result<(), ()>,
    },
    /// Local provider state.
    ProviderState(bool, u64),
    AdvertisedPeerChanged(PeerId, bool),
    Error(Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    ConnectionClosed,
    VerifierMismatch,
    NotProviding,
    Timeout,
    UnrecognizedMessage(String), // Serialzied not available on the original type
    IO(String),                  // Serialize not available on the original type
    Channel,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            ConnectionClosed => f.write_str("Connection Closed"),
            VerifierMismatch => f.write_str("Message verifier mismatch"),
            Timeout => f.write_str("Message timed out"),
            NotProviding => f.write_str("Relay is not providing"),
            UnrecognizedMessage(msg) => f.write_str(msg),
            IO(msg) => f.write_str(msg),
            Channel => f.write_str("Callback channel closed unexpectedly"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

mod protocol {
    pub const PROTOCOL_NAME: &str = "/owlnest/advertise/0.0.1";
    pub use owlnest_prelude::utils::protocol::universal::*;
}

#[derive(Debug)]
pub enum InEvent {
    /// Set local provider state.
    SetProviderState(bool, u64),
    /// Get local provider state.
    GetProviderState(u64),
    /// Send a query to a remote peer for advertised peers.
    QueryAdvertisedPeer(PeerId),
    /// Set remote provider state to advertise or stop advertising local peer.
    SetRemoteAdvertisement {
        remote: PeerId,
        state: bool,
        id: u64,
    },
    /// Remove a advertised peer from local provider.
    RemoveAdvertised(PeerId),
    /// Remove all advertised peers from local provider.
    ClearAdvertised(),
    ListConnected(oneshot::Sender<Vec<PeerId>>),
}
