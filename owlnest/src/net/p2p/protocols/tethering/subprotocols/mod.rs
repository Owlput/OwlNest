use libp2p_swarm::Stream;

pub mod exec;
pub mod push;

/// Subprotocol name used for pushing notifications to remote peer.
pub const PUSH_PROTOCOL_NAME: &'static str = "/owlnest/tethering/0.0.1/push";

pub const EXEC_PROTOCOL_NAME: &'static str = "/owlnest/tethering/0.0.1/exec";

#[inline]
async fn write_flush<T: From<std::io::Error>>(socket: &mut Stream, bytes: &[u8]) -> Result<(), T> {
    use futures::AsyncWriteExt;
    socket.write_all(bytes).await.map_err(|e| From::from(e))?;
    socket.flush().await.map_err(|e| From::from(e))
}

#[inline]
async fn read_u64<T:From<std::io::Error>>(socket:&mut Stream)->Result<u64,T>{
    use futures::AsyncReadExt;
    let mut recv_buf = [0u8; 8];
    match socket.read_exact(&mut recv_buf).await {
        Ok(_) => Ok(u64::from_be_bytes(recv_buf)),
        Err(e) => Err(From::from(e)),
    }
}