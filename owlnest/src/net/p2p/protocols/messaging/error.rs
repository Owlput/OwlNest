use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    ConnectionClosed,
    VerifierMismatch,
    PeerNotFound(PeerId),
    Timeout,
    UnrecognizedMessage(String), // Serialzied not available on the original type
    IO(String),                  // Serialize not available on the original type
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionClosed => f.write_str("Connection Closed"),
            Self::VerifierMismatch => f.write_str("Message verifier mismatch"),
            Self::Timeout => f.write_str("Message timed out"),
            Self::PeerNotFound(peer) => f.write_str(&format!("Peer {} not connected", peer)),
            Self::UnrecognizedMessage(msg) => f.write_str(msg),
            Self::IO(msg) => f.write_str(msg),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
