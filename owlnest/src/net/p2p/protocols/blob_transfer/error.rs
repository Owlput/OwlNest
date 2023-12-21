use std::fmt::Display;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    UnrecognizedMessage(String), // Serialzied not available on the original type
    IO(String),                  // Serialize not available on the original type
    Channel,
    SendIdNotFound(u64),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            UnrecognizedMessage(msg) => f.write_str(msg),
            IO(msg) => f.write_str(msg),
            Channel => f.write_str("Callback channel closed unexpectedly"),
            SendIdNotFound(id) => write!(f, "Send ID {} not found", id),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum SendError {
    ConnectionClosed,
    VerifierMismatch,
    PeerNotFound(PeerId),
    Timeout,
}

impl Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SendError::*;
        match self {
            ConnectionClosed => f.write_str("Connection Closed"),
            VerifierMismatch => f.write_str("Message verifier mismatch"),
            Timeout => f.write_str("Message timed out"),
            PeerNotFound(peer) => f.write_str(&format!("Peer {} not connected", peer)),
        }
    }
}

#[derive(Debug)]
pub enum FileSendReqError {
    IsDirectory,
    FileNotFound,
    PermissionDenied,
    OtherFsError(std::io::ErrorKind),
    Timeout,
    PeerNotFound,
}

#[derive(Debug)]
pub enum FileRecvError {
    IllegalFilePath,
    PendingRecvNotFound,
    Timeout,
    FsError(std::io::ErrorKind),
}
impl From<std::io::Error> for FileRecvError {
    fn from(value: std::io::Error) -> Self {
        Self::FsError(value.kind())
    }
}
