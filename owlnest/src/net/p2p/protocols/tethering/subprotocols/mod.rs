pub mod exec;
pub mod push;

/// Subprotocol name used for pushing notifications to remote peer.
pub const PUSH_PROTOCOL_NAME: &'static str = "/owlnest/tethering/0.0.1/push";

pub const EXEC_PROTOCOL_NAME: &'static str = "/owlnest/tethering/0.0.1/exec";
