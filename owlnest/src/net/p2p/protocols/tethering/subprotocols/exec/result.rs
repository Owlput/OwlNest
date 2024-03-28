use serde::{Deserialize, Serialize};

/// Operation result for sending to the peer who sends the request.
/// Won't be exposed via `tethering::OutEvent` but be sent through sender provided by the request caller after unwrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpResult {
    Messaging(u128),
    Tethering(),
}

