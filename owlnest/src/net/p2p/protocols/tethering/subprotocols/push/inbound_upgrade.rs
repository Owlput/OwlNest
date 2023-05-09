use super::PUSH_PROTOCOL_NAME;
use futures::{future::BoxFuture, AsyncReadExt, AsyncWriteExt, FutureExt};
use libp2p::{core::upgrade, swarm::NegotiatedSubstream};

pub struct Upgrade;

impl upgrade::UpgradeInfo for Upgrade {
    type Info = &'static str;
    type InfoIter = core::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        core::iter::once(PUSH_PROTOCOL_NAME)
    }
}

impl upgrade::InboundUpgrade<NegotiatedSubstream> for Upgrade {
    type Output = NegotiatedSubstream;
    type Error = UpgradeError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: NegotiatedSubstream, _: Self::Info) -> Self::Future {
        async move {
            // Receive SYN
            let mut syn_recv = [0u8; 8];
            let syn_recv = match socket.read_exact(&mut syn_recv).await {
                Ok(_) => u64::from_be_bytes(syn_recv),
                Err(e) => return Err(UpgradeError::StreamError(e.to_string())),
            };
            // Send ACK
            socket
                .write_all(&(syn_recv + 1).to_be_bytes())
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            socket
                .flush()
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            // Send SYN
            let syn = rand::random::<u64>();
            socket
                .write_all(&syn.to_be_bytes())
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            socket
                .flush()
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            // Receive ACK
            let mut ack_recv = [0u8; 8];
            let ack_recv = match socket.read_exact(&mut ack_recv).await {
                Ok(_) => u64::from_be_bytes(ack_recv),
                Err(e) => return Err(UpgradeError::StreamError(e.to_string())),
            };
            if ack_recv.wrapping_sub(1) != syn{
                return Err(UpgradeError::UnexpectedACK(syn,ack_recv))
            }
            Ok(socket)
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum UpgradeError {
    StreamError(String),
    UnexpectedACK(u64,u64)
}
