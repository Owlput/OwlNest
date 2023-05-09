use std::fmt::Display;
use libp2p::{core::upgrade, swarm::NegotiatedSubstream};
use super::*;

pub struct Upgrade;

impl upgrade::UpgradeInfo for Upgrade {
    type Info = &'static str;
    type InfoIter = core::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        core::iter::once(EXEC_PROTOCOL_NAME)
    }
}

impl upgrade::OutboundUpgrade<NegotiatedSubstream> for Upgrade {
    type Output = NegotiatedSubstream;
    type Error = UpgradeError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        // Initialize a TCP style handshake
        async move {
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
            let mut ack = [0u8; 8];
            socket
                .read_exact(&mut ack)
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            let ack = u64::from_be_bytes(ack);
            if ack.wrapping_sub(1) != syn {
                return Err(UpgradeError::UnexpectedACK(syn, ack));
            }
            let mut syn_recv = [0u8; 8];
            let syn_recv = match socket.read_exact(&mut syn_recv).await {
                Ok(_) => u64::from_be_bytes(syn_recv),
                Err(e) => return Err(UpgradeError::StreamError(e.to_string())),
            };
            // Send ACK
            socket
                .write_all(&(syn_recv.wrapping_add(1)).to_be_bytes())
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            socket
                .flush()
                .await
                .map_err(|e| UpgradeError::StreamError(e.to_string()))?;
            Ok(socket)
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum UpgradeError {
    StreamError(String),
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
