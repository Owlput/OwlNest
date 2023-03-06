use std::time::Duration;

use super::subprotocols::{exec, push};
use super::*;

/// Meta enum for all possible results returned by subprotocols of `tethering`.
#[derive(Debug)]
pub enum HandleError {
    LocalExec(TetheringOpError),
    RemoteExec(exec::HandleError),
    Push(push::HandleError),
}
impl From<exec::HandleError> for HandleError {
    fn from(value: exec::HandleError) -> Self {
        Self::RemoteExec(value)
    }
}
impl TryInto<exec::HandleError> for HandleError {
    type Error = ();
    fn try_into(self) -> Result<exec::HandleError, Self::Error> {
        match self {
            HandleError::RemoteExec(e) => Ok(e),
            _ => Err(()),
        }
    }
}
impl From<TetheringOpError> for HandleError {
    fn from(value: TetheringOpError) -> Self {
        Self::LocalExec(value)
    }
}
impl TryInto<TetheringOpError> for HandleError {
    type Error = ();

    fn try_into(self) -> Result<TetheringOpError, Self::Error> {
        match self {
            HandleError::LocalExec(e) => Ok(e),
            _ => Err(()),
        }
    }
}
impl From<push::HandleError> for HandleError {
    fn from(value: push::HandleError) -> Self {
        Self::Push(value)
    }
}
impl TryInto<push::HandleError> for HandleError {
    type Error = ();

    fn try_into(self) -> Result<push::HandleError, Self::Error> {
        match self {
            Self::Push(e) => Ok(e),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum HandleOk {
    LocalExec(()),
    RemoteExec(Duration),
    Push(()),
}
