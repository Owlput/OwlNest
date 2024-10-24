pub mod handler_prelude {
    pub use futures::{future::BoxFuture, FutureExt};
    pub use libp2p::core::upgrade::ReadyUpgrade;
    pub use libp2p::swarm::{
        handler::{
            ConnectionEvent, DialUpgradeError, FullyNegotiatedInbound, FullyNegotiatedOutbound,
        },
        ConnectionHandler, ConnectionHandlerEvent, Stream, StreamUpgradeError, SubstreamProtocol,
    };
    pub use std::io;
    pub use std::task::Poll;
}

pub mod swarm_prelude {}
pub mod behaviour_prelude {
    pub use libp2p::core::transport::PortUse;
    pub use libp2p::core::Endpoint;
    pub use libp2p::swarm::NetworkBehaviour;
    pub use libp2p::swarm::NotifyHandler;
    pub use libp2p::swarm::{ConnectionClosed, ConnectionDenied, ConnectionHandler, ConnectionId};
    pub use libp2p::swarm::{FromSwarm, ToSwarm};
    pub use std::task::Poll;
}

pub mod lib_prelude {
    pub use libp2p::{Multiaddr, PeerId};
}

pub mod utils {
    pub mod protocol {
        #[cfg(feature = "universal-protocol")]
        pub mod universal {
            use futures::prelude::*;
            use std::io;
            use std::time::{Duration, Instant};
            use xxhash_rust::xxh3::xxh3_128;
            /// Universal protocol for sending bytes.  
            /// Send and receive operation are performed on different negoticated substreams
            ///```norun
            ///      send()-->Outbound------>Inbound-->recv()
            ///      recv()<--Inbound<------Outbound<--send()
            ///         Peer A                   Peer B
            /// ```
            pub async fn send<S>(mut stream: S, msg_bytes: Vec<u8>) -> io::Result<(S, Duration)>
            where
                S: AsyncRead + AsyncWrite + Unpin,
            {
                let verf = xxh3_128(&msg_bytes);
                let len = msg_bytes.len();
                stream.write_all(&len.to_be_bytes()).await?;
                stream.flush().await?;
                let mut cursor = 0;
                loop {
                    let bytes_written = stream.write(&msg_bytes[cursor..]).await?;
                    cursor += bytes_written;
                    if cursor >= len {
                        break;
                    }
                }
                stream.flush().await?;
                let now = Instant::now();
                let mut verf_read = [0u8; 16];
                stream.read_exact(&mut verf_read).await?;
                if verf_read == verf.to_be_bytes() {
                    return Ok((stream, now.elapsed()));
                }
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Verifier mismatch",
                ))
            }

            pub async fn recv<S>(mut stream: S) -> io::Result<(S, Vec<u8>)>
            where
                S: AsyncRead + AsyncWrite + Unpin,
            {
                let mut buf = [0u8; 8];
                stream.read_exact(&mut buf).await?;
                let bytes_to_read = usize::from_be_bytes(buf);
                if bytes_to_read / 1024 > 128 {
                    return io::Result::Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "Stream too long. Terminating.",
                    ));
                }
                let mut msg_buf: Vec<u8> = Vec::with_capacity(bytes_to_read);
                let mut cursor = 0;
                loop {
                    let mut buf = [0u8; 2048];
                    let bytes_read = stream.read(&mut buf).await?;
                    io::Write::write(&mut msg_buf, &buf[0..bytes_read]).expect("Write to vec");
                    cursor += bytes_read;
                    if cursor >= bytes_to_read {
                        break;
                    }
                }
                stream.write_all(&xxh3_128(&msg_buf).to_be_bytes()).await?;
                stream.flush().await?;
                Ok((stream, msg_buf))
            }
        }
    }
}
