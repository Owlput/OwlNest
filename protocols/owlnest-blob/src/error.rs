use std::{fmt::Display, io::ErrorKind};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    UnrecognizedMessage(String), // Serialzied not available on the original type
    IO(String),                  // Serialize not available on the original type
    Channel,
    SendIdNotFound(u64),
    UnexpectedEOF(u64),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            UnrecognizedMessage(msg) => f.write_str(msg),
            IO(msg) => f.write_str(msg),
            Channel => f.write_str("Callback channel closed unexpectedly"),
            SendIdNotFound(id) => write!(f, "Send ID {} not found", id),
            UnexpectedEOF(recv_id) => {
                write!(f, "The file of recv ID {} meets an unexpected EOF", recv_id)
            }
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub enum FileSendError {
    IsDirectory,
    FileNotFound,
    PermissionDenied,
    OtherFsError(std::io::ErrorKind),
    Timeout,
    PeerNotFound,
}
impl std::error::Error for FileSendError {}
impl Display for FileSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FileSendError::*;
        match self {
            IsDirectory => write!(f, "The target to send to is a directory"),
            FileNotFound => write!(f, "The target file is not found"),
            PermissionDenied => write!(f, "Permission denied"),
            OtherFsError(error_kind) => {
                write!(f, "Other file system error: {}", error_kind.to_string())
            }
            Timeout => write!(f, "Operation has timed out"),
            PeerNotFound => write!(f, "Target peer is not found"),
        }
    }
}

#[derive(Debug)]
pub enum FileRecvError {
    PendingRecvNotFound(u64),
    Timeout,
    FsError {
        path: String,
        error: std::io::ErrorKind,
    },
}
impl std::error::Error for FileRecvError {}
impl Display for FileRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            FileRecvError::PendingRecvNotFound(id) => write!(f,"Cannot find operation associated with recv ID {}, is the request already accepted or cancaled?", id),
            FileRecvError::Timeout => write!(f,"Timeout when waiting response from remote."),
            FileRecvError::FsError{path,error} => {
                match error {
                    ErrorKind::AlreadyExists => write!(f,"File(or folder) {} already exists. Overwritting is not allowed. Please delete the file before accepting the request.", path),
                    ErrorKind::PermissionDenied => write!(f, "Cannot write to file(or folder) {}: Permission denied. Please make sure you have properly set the permission.", path),
                    e => write!(f,"OS reported OtherError {}", e)
                }
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CancellationError {
    IdNotFound,
    PeerNotFound,
}
impl std::error::Error for CancellationError {}
impl Display for CancellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CancellationError::*;
        match self {
            IdNotFound => write!(f, "ID not found"),
            PeerNotFound => write!(f, "Peer not found"),
        }
    }
}
