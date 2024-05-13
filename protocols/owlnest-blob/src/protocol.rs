use futures::prelude::*;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{trace, trace_span};
use xxhash_rust::xxh3::xxh3_128;
pub const PROTOCOL_NAME: &str = "/owlnest/blob/0.0.1";
#[allow(unused)]
const MAX_PACKET_SIZE: usize = 1 << 18;

/// buf includes header, so len will be the actuall message + 8 bytes of header
pub async fn send<S>(mut stream: S, buf: Arc<[u8]>, len: usize) -> io::Result<(S, Duration)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let verf = xxh3_128(&buf[8..len]);
    let mut cursor = 0;
    loop {
        let bytes_write = stream.write(&buf[cursor..len]).await?;
        cursor += bytes_write;
        if cursor >= len {
            trace!("All {} bytes written, exiting", len);
            break;
        }
    }
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
        "Verifier mismatch",
    ))
}

pub async fn recv<S>(mut stream: S) -> io::Result<(S, Arc<[u8]>, usize, u8)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let span = trace_span!("protocol::recv");
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await?;
    let msg_type = header[0];
    header[0] = 0u8;
    let bytes_to_read = u64::from_be_bytes(header);
    span.in_scope(|| {
        trace!("reading stream length {}", bytes_to_read);
    });
    if bytes_to_read > (1 << 18) {
        return io::Result::Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Stream too long. Terminating.",
        ));
    }
    let buf: Arc<[u8]> = if msg_type == 2 {
        let mut buf = [0u8; (1 << 18) + 16];
        cursor_read(&mut stream, &mut buf, bytes_to_read as usize).await?;
        Arc::new(buf)
    } else {
        let mut buf = [0u8; 1 << 6];
        cursor_read(&mut stream, &mut buf, bytes_to_read as usize).await?;
        Arc::new(buf)
    };
    span.in_scope(|| {
        trace!("All bytes read");
    });
    Ok((stream, buf, bytes_to_read as usize, msg_type))
}

async fn cursor_read<S>(s: &mut S, buf: &mut [u8], len: usize) -> Result<(), std::io::Error>
where
    S: AsyncRead + AsyncWrite + std::marker::Unpin,
{
    let mut cursor = 0usize;
    loop {
        if cursor >= len {
            break;
        }
        let bytes_read = s.read(&mut buf[cursor..]).await?;
        cursor += bytes_read;
    }
    s.write_all(&xxh3_128(&buf[0..cursor]).to_be_bytes())
        .await?;
    s.flush().await?;
    Ok(())
}
