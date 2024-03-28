use crate::net::p2p::protocols::tethering::subprotocols::{write_flush, read_u64};

use super::*;
use libp2p::{core::upgrade, swarm::Stream};
use std::fmt::Display;

pub struct Upgrade;

impl upgrade::UpgradeInfo for Upgrade {
    type Info = &'static str;
    type InfoIter = core::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        core::iter::once(EXEC_PROTOCOL_NAME)
    }
}

impl upgrade::InboundUpgrade<Stream> for Upgrade {
    type Output = Stream;
    type Error = UpgradeError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: Stream, _: Self::Info) -> Self::Future {
        // A TCP-style handshake.
        async move {
            let syn_recv = read_u64::<UpgradeError>(&mut socket).await?;
            // Send ACK
            write_flush::<UpgradeError>(&mut socket, &(syn_recv + 1).to_be_bytes()).await?;
            // Send SYN
            let syn = rand::random::<u64>();
            write_flush::<UpgradeError>(&mut socket, &syn.to_be_bytes()).await?;
            // Receive ACK
            let ack_recv = read_u64::<UpgradeError>(&mut socket).await?;
            if ack_recv.wrapping_sub(1) != syn {
                return Err(UpgradeError::UnexpectedACK(syn, ack_recv));
            }
            Ok(socket)
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum UpgradeError {
    StreamError(std::io::Error),
    UnexpectedACK(u64, u64),
}
impl Display for UpgradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamError(e) => f.write_str(&format!("Stream error: {}", e)),
            UpgradeError::UnexpectedACK(expected, received) => f.write_str(&format!(
                "ACK mismatch, expected: {}, got: {}",
                expected, received
            )),
        }
    }
}
impl std::error::Error for UpgradeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
impl From<std::io::Error> for UpgradeError {
    fn from(value: std::io::Error) -> Self {
        Self::StreamError(value)
    }
}
