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

impl upgrade::InboundUpgrade<Stream> for Upgrade {
    type Output = Stream;
    type Error = UpgradeError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: Stream, _: Self::Info) -> Self::Future {
        async move {
            // Receive SYN
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
impl From<std::io::Error> for UpgradeError {
    fn from(value: std::io::Error) -> Self {
        UpgradeError::StreamError(value)
    }
}
