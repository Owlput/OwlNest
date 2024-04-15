use futures::prelude::*;
use std::io;
use std::time::{Duration, Instant};
use tracing::{trace, trace_span};
use xxhash_rust::xxh3::xxh3_128;
pub const PROTOCOL_NAME: &str = "/owlnest/blob/0.0.1";

/// Universal protocol for sending bytes

// Send and receive operation are performed on different negoticated substreams
//      send()-->Outbound------>Inbound-->recv()
//      recv()<--Inbound<------Outbound<--send()
//              Peer A          Peer B
pub async fn send<S>(mut stream: S, msg_bytes: Vec<u8>) -> io::Result<(S, Duration)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let span = trace_span!("protocol::send");
    let verf = xxh3_128(&msg_bytes);
    let len = msg_bytes.len();
    span.in_scope(|| {
        trace!("{} bytes to write, hash {}", len, verf);
    });
    stream.write_all(&(len as u64).to_be_bytes()).await?;
    stream.flush().await?;
    span.in_scope(|| {
        trace!("stream length written and flushed");
    });
    let mut cursor = 0;
    loop {
        let bytes_write = stream.write(&msg_bytes[cursor..]).await?;
        cursor += bytes_write;
        let _entered = span.enter();
        trace!("cursor at {} bytes, {} bytes total", bytes_write, len);
        if cursor >= len {
            trace!("All bytes written, exiting");
            break;
        }
    }
    stream.flush().await?;
    span.in_scope(|| {
        trace!("all bytes written and flushed");
    });
    drop(msg_bytes);
    let now = Instant::now();
    let mut verf_read = [0u8; 16];
    span.in_scope(|| {
        trace!("reading hash verifier from rmeote");
    });
    stream.read_exact(&mut verf_read).await?;
    let _entered = span.enter();
    if verf_read == verf.to_be_bytes() {
        trace!("send finished");
        return Ok((stream, now.elapsed()));
    }
    trace!("verifier mismatch");
    Err(std::io::Error::new(
        io::ErrorKind::InvalidData,
        "Verifier mismatch",
    ))
}

pub async fn recv<S>(mut stream: S) -> io::Result<(S, Vec<u8>)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let span = trace_span!("protocol::recv");
    let mut buf = [0u8; 8];
    stream.read_exact(&mut buf).await?;
    let bytes_to_read = u64::from_be_bytes(buf);
    span.in_scope(|| {
        trace!("reading stream length {}", bytes_to_read);
    });
    if bytes_to_read > (1 << 18) {
        return io::Result::Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Stream too long. Terminating.",
        ));
    }
    let mut msg_buf = [0u8; 1 << 17];
    let mut cursor = 0;
    loop {
        let bytes_read = stream.read(&mut msg_buf[cursor..]).await?;
        cursor += bytes_read;
        span.in_scope(|| {
            trace!("cursor at {} byte, {} bytes total", cursor, bytes_to_read);
        });
        if cursor as u64 >= bytes_to_read {
            break;
        }
    }
    stream
        .write_all(&xxh3_128(&msg_buf[..cursor]).to_be_bytes())
        .await?;
    stream.flush().await?;
    span.in_scope(|| {
        trace!("verifier written and flushed");
    });
    let mut msg = Vec::with_capacity(bytes_to_read as usize);
    msg.extend_from_slice(&msg_buf[..cursor]);
    Ok((stream, msg))
}
