use futures::prelude::*;
use std::io;
use std::time::{Duration, Instant};
use tracing::{trace, trace_span};
use xxhash_rust::xxh3::xxh3_128;
pub const PROTOCOL_NAME: &str = "/owlnest/blob/0.0.1";
#[allow(unused)]
const MAX_PACKET_SIZE: usize = 1 << 18;

/// buf includes header, so len will be the actuall message + 8 bytes of header
pub async fn send<S>(mut stream: S, buf: Vec<u8>, message_type: u8) -> io::Result<(S, Duration)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let len = buf.len() as u64;
    let span = trace_span!("protocol::send");
    let _entered = span.enter();
    let verf = xxh3_128(&buf);
    let mut header = len.to_be_bytes();
    header[0] = message_type;
    stream.write_all(&header).await?;
    stream.write_all(&buf).await?;
    let now = Instant::now();
    stream.flush().await?;
    let mut verf_read = [0u8; 16];
    stream.read_exact(&mut verf_read).await?;
    trace!("reading verifier");
    if u128::from_be_bytes(verf_read) == verf {
        return Ok((stream, now.elapsed()));
    }
    Err(std::io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Verifier mismatch! expected {:?}, got {:?}",
            verf.to_be_bytes(),
            verf_read
        ),
    ))
}

pub async fn recv<S>(mut stream: S) -> io::Result<(S, Vec<u8>, u8)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let span = trace_span!("protocol::recv");
    let mut header = [0u8; 8]; // length of the streamed bytes and message type
    stream.read_exact(&mut header).await?;
    let message_type = header[0];
    header[0] = 0;
    let bytes_to_read = u64::from_be_bytes(header);
    span.in_scope(|| trace!("reading stream length {}", bytes_to_read));
    if bytes_to_read > ((1 << 18) + 16) {
        // bytes_to_read won't exceed 2^32, so it is safe to cast to usize
        // unless you are running on a 16-bit platform
        return io::Result::Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Stream too long. Terminating.",
        ));
    }
    let bytes_to_read = bytes_to_read as usize;
    let mut buf = Vec::with_capacity(0);
    buf.resize(bytes_to_read, 0u8);
    cursor_read(&mut stream, &mut buf, bytes_to_read).await?;
    span.in_scope(|| trace!("All bytes read and verified"));
    Ok((stream, buf, message_type))
}

async fn cursor_read<S>(s: &mut S, buf: &mut [u8], len: usize) -> Result<(), std::io::Error>
where
    S: AsyncRead + AsyncWrite + std::marker::Unpin,
{
    let mut cursor = 0;
    loop {
        if cursor >= len {
            break;
        }
        let bytes_read = s.read(&mut buf[cursor..]).await?;
        cursor += bytes_read;
    }
    s.write_all(&xxh3_128(&buf[..cursor]).to_be_bytes()).await?;
    s.flush().await?;
    Ok(())
}
