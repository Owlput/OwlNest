use super::PUSH_PROTOCOL_NAME;
use crate::net::p2p::protocols::tethering::subprotocols::{read_u64, write_flush};
use futures::{future::BoxFuture, FutureExt};
use libp2p::{core::upgrade, swarm::Stream};

pub struct Upgrade;

impl upgrade::UpgradeInfo for Upgrade {
    type Info = &'static str;
    type InfoIter = core::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        core::iter::once(PUSH_PROTOCOL_NAME)
    }
}

impl upgrade::OutboundUpgrade<Stream> for Upgrade {
    type Output = Stream;
    type Error = UpgradeError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: Stream, _info: Self::Info) -> Self::Future {
        // Initialize a TCP style handshake
        async move {
            let syn = rand::random::<u64>();
            write_flush::<UpgradeError>(&mut socket, &syn.to_be_bytes()).await?;
            let ack = read_u64::<UpgradeError>(&mut socket).await?;
            if ack.wrapping_sub(1) != syn {
                return Err(UpgradeError::UnexpectedACK(syn, ack));
            }
            let syn_recv = read_u64::<UpgradeError>(&mut socket).await?;
            write_flush::<UpgradeError>(&mut socket, &(syn_recv.wrapping_add(1)).to_be_bytes())
                .await?;
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
impl From<std::io::Error> for UpgradeError {
    fn from(value: std::io::Error) -> Self {
        Self::StreamError(value)
    }
}
