pub mod push;
pub mod exec;



/// Subprotocol name used for pushing notifications to remote peer.
pub const PUSH_PROTOCOL_NAME:&[u8;29]=b"/owlnest/tethering/0.0.1/push";

pub const EXEC_PROTOCOL_NAME:&[u8;29]=b"/owlnest/tethering/0.0.1/exec";

