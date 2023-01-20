use std::{io, time::{Duration, Instant}};
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, AsyncReadExt};

pub const PROTOCOL_NAME: &[u8] = b"/owlput/tethering/0.0.1";

pub(crate) async fn send<S>(mut stream: S,msg_bytes: Vec<u8>,stamp:u128) -> io::Result<(S, u128, Duration)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{   
    let stamp_bytes = stamp.to_be_bytes();
    stream.write_all(&msg_bytes).await?;
    drop(msg_bytes);
    stream.flush().await?;
    let started = Instant::now();
    let mut stamp_returned = [0u8; 16];
    stream.read_exact(&mut stamp_returned).await?;
    if stamp_returned == stamp_bytes {
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
