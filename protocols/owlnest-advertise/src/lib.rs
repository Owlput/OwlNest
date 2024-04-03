use owlnest_prelude::lib_prelude::*;
use serde::{Deserialize, Serialize};

pub mod behaviour;
mod handler;
pub mod in_event;

pub use behaviour::Behaviour;

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
            NotProviding => f.write_str(&format!("Relay is not providing")),
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
    SetAdvertisingSelf {
        remote: PeerId,
        state: bool,
        id: u64,
    },
    /// Remove a advertised peer from local provider.
    RemoveAdvertised(PeerId),
    /// Remove all advertised peers from local provider.
    ClearAdvertised,
}

#[cfg(feature = "disabled")]
#[cfg(test)]
mod test {
    use crate::net::p2p::setup_default;
    use libp2p::Multiaddr;
    use std::{thread, time::Duration};

    #[test]
    fn test() {
        let (peer1_m, _) = setup_default();
        let (peer2_m, _) = setup_default();
        peer1_m
            .swarm()
            .listen_blocking(&"/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        let peer1_id = peer1_m.identity().get_peer_id();
        let peer2_id = peer2_m.identity().get_peer_id();
        peer2_m
            .swarm()
            .dial_blocking(&peer1_m.swarm().list_listeners_blocking()[0])
            .unwrap();
        thread::sleep(Duration::from_millis(200));
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.relay_ext().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        peer2_m
            .executor()
            .block_on(peer2_m.relay_ext().set_remote_advertisement(peer1_id, true));
        assert!(peer2_m.swarm().is_connected_blocking(peer1_id));
        thread::sleep(Duration::from_millis(200));
        assert!(peer2_m
            .executor()
            .block_on(peer2_m.relay_ext().query_advertised_peer(peer1_id))
            .unwrap()
            .contains(&peer2_id));
        assert!(!peer1_m
            .executor()
            .block_on(peer1_m.relay_ext().set_provider_state(false)));
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.relay_ext().query_advertised_peer(peer1_id))
                .unwrap()
                .len()
                == 0
        );
        peer2_m.executor().block_on(
            peer2_m
                .relay_ext()
                .set_remote_advertisement(peer1_id, false),
        );
        assert!(peer1_m
            .executor()
            .block_on(peer1_m.relay_ext().set_provider_state(true)));
        thread::sleep(Duration::from_millis(200));
        assert!(
            peer2_m
                .executor()
                .block_on(peer2_m.relay_ext().query_advertised_peer(peer1_id))
                .unwrap()
                .len()
                == 0
        );
    }
}
