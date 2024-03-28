use std::{fmt::Display, string::FromUtf8Error};

#[derive(Debug)]
pub enum Error {
    ConnectionClosed,
    StampMismatch,
    Timeout,
    UnrecognizedOp(serde_json::Error, Result<String, FromUtf8Error>),
    IO(std::io::Error),
    NotApplicableOnLocalNode,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionClosed => f.write_str("Connection Closed"),
            Self::StampMismatch => f.write_str("Message verifier mismatch"),
            Self::Timeout => f.write_str("Message timeout"),
            Self::UnrecognizedOp(e, broken_op) => f.write_str(&format!(
                "Failed to deserialize message with error: {}, possible raw data: {:?}",
                e, broken_op
            )),
            Self::IO(e) => f.write_str(&format!("IO error: {}", e)),
            Self::NotApplicableOnLocalNode => f.write_str("Not an action applicable to local node"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
