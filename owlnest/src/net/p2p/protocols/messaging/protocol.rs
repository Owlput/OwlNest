use futures::prelude::*;
use std::io;
use std::time::{Duration, Instant};
use xxhash_rust::xxh3::xxh3_128;

pub const PROTOCOL_NAME: &[u8] = b"/owlput/messaging/0.0.1";

// Send and receive operation are performed in different negoticated substreams
//      send()<--Outbound-->|<--Inbound-->recv()
//      recv()<--Inbound-->|<--Outbound-->send()
//              Peer A          Peer B
pub(crate) async fn send<S>(mut stream: S,mut msg_bytes: Vec<u8>,stamp:u128) -> io::Result<(S, u128, Duration)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{   
    let verf = xxh3_128(&msg_bytes);
    let verf_bytes :[u8; 16] = verf.to_be_bytes();
    msg_bytes.extend_from_slice(&verf_bytes);
    stream.write_all(&msg_bytes).await?;
    drop(msg_bytes);
    stream.flush().await?;
    let started = Instant::now();
    let mut verf_returned = [0u8; 16];
    stream.read_exact(&mut verf_returned).await?;
    if verf_returned == verf_bytes {
        Ok((stream, stamp, started.elapsed()))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Returned verifier mismatch",
        ))
    }
}

pub(crate) async fn recv<S>(mut stream: S) -> io::Result<(S, Vec<u8>)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf: Vec<u8> = Vec::new();
    stream.read(&mut buf).await?;
    stream.write_all(&buf.split_off(buf.len() - 15)).await?;
    stream.flush().await?;
    Ok((stream, buf))
}
