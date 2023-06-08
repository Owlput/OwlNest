use futures::{future::BoxFuture, FutureExt};
pub mod handler;
mod inbound_upgrade;
pub mod op;
mod outbound_upgrade;
pub mod result;

use super::EXEC_PROTOCOL_NAME;
pub use handler::InEvent;
pub use handler::OutEvent;
pub use op::Op;
pub use result::*;

mod protocol {
    pub use crate::net::p2p::protocols::universal::protocol::*;
}
