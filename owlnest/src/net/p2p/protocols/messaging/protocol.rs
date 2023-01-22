use futures::prelude::*;
use std::io;
use std::time::{Duration, Instant};
use xxhash_rust::xxh3::xxh3_128;

pub const PROTOCOL_NAME: &[u8] = b"/owlput/messaging/0.0.1";

// Send and receive operation are performed in different negoticated substreams
//      send()<--Outbound-->|<--Inbound-->recv()
//      recv()<--Inbound-->|<--Outbound-->send()
//              Peer A          Peer B
pub async fn send<S>(mut stream: S,msg_bytes: Vec<u8>,stamp:u128) -> io::Result<(S,u128,Duration)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{   
    let verf = xxh3_128(&msg_bytes);
    let chunks_number = (msg_bytes.len()/256)+1;
    stream.write(&chunks_number.to_be_bytes()).await?;
    stream.flush().await?;
    let mut chunks = msg_bytes.chunks_exact(256);
    for _ in 1..chunks_number{
        stream.write_all(chunks.next().unwrap()).await?;
        stream.flush().await?;
    }
    let mut remainder = chunks.remainder().to_vec();
    drop(msg_bytes);
    remainder.resize(256, 32u8);
    stream.write_all(&remainder).await?;
    stream.flush().await?;
    drop(remainder);
    let now = Instant::now();
    let mut verf_read = [0u8;16];
    stream.read_exact(&mut verf_read).await?;
    if verf_read == verf.to_be_bytes(){
        return Ok((stream,stamp,now.elapsed()))
    }
    Err(std::io::Error::new(io::ErrorKind::InvalidData, "Verifier mismatch"))
}

pub async fn recv<S>(mut stream: S) -> io::Result<(S,Vec<u8>)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = [0u8;8];
    stream.read_exact(&mut buf).await?;
    let chunks_to_read = usize::from_be_bytes(buf);
    drop(buf);
    let mut msg_buf:Vec<u8> = Vec::new();
    for _ in 0..chunks_to_read{
        let mut buf = [0u8;256];
        stream.read_exact(&mut buf).await?;
        msg_buf.extend_from_slice(&buf) 
    }
    msg_buf = msg_buf.trim_ascii_end().to_vec();
    stream.write(&xxh3_128(&msg_buf).to_be_bytes()).await?;
    stream.flush().await?;
    Ok((stream,msg_buf))
}
